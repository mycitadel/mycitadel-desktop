// MyCitadel desktop wallet: bitcoin & RGB wallet based on GTK framework.
//
// Written in 2022 by
//     Dr. Maxim Orlovsky <orlovsky@pandoraprime.ch>
//
// Copyright (C) 2022 by Pandora Prime Sarl, Switzerland.
//
// This software is distributed without any warranty. You should have received
// a copy of the AGPL-3.0 License along with this software. If not, see
// <https://www.gnu.org/licenses/agpl-3.0-standalone.html>.

use std::collections::BTreeSet;
use std::sync::Arc;

use bitcoin::hashes::{sha256, Hash};
use bitcoin::secp256k1::{self, SECP256K1};
use bitcoin::util::bip32::{ChildNumber, ExtendedPubKey};
use miniscript::descriptor::{Sh, TapTree, Tr, Wsh};
use miniscript::policy::concrete::Policy;
use miniscript::{Descriptor, Legacy, Miniscript, Segwitv0, Tap};
use wallet::hd::{
    AccountStep, DerivationScheme, HardenedIndex, SegmentIndexes, TerminalStep, TrackingAccount,
    XpubRef,
};

use super::spending_row::SpendingModel;
use crate::model::{
    DescriptorClass, HardwareList, PublicNetwork, Signer, WalletDescriptor, WalletTemplate,
};

pub struct ViewModel {
    pub class: DescriptorClass,
    pub scheme: DerivationScheme,
    pub network: PublicNetwork,
    pub signers: BTreeSet<Signer>,
    pub spendings: SpendingModel,

    // Data provided by the parent window
    pub template: Option<WalletTemplate>,
    pub new_wallet: bool,
    pub format_lnpbp: bool,

    // Non-persisting / dynamic data for this window
    pub active_signer: Option<Signer>,
    pub devices: HardwareList,
    pub descriptor: Option<Descriptor<TrackingAccount>>,
}

impl Default for ViewModel {
    fn default() -> Self {
        ViewModel {
            // TODO: Add `ScriptType` to descriptor-wallet and simplify constructor
            scheme: DerivationScheme::Bip48 {
                script_type: HardenedIndex::from_index(2u32).unwrap(),
            },
            devices: none!(),
            signers: none!(),
            active_signer: None,
            spendings: SpendingModel::new(),
            network: PublicNetwork::Mainnet,
            descriptor: None,
            new_wallet: true,
            template: None,
            class: DescriptorClass::SegwitV0,
            format_lnpbp: false,
        }
    }
}

impl From<WalletTemplate> for ViewModel {
    fn from(template: WalletTemplate) -> Self {
        ViewModel {
            template: Some(template),
            new_wallet: true,
            ..default!()
        }
    }
}

impl From<WalletDescriptor> for ViewModel {
    fn from(_descr: WalletDescriptor) -> Self {
        ViewModel {
            new_wallet: false,
            // TODO Fix it
            ..default!()
        }
    }
}

impl From<&ViewModel> for WalletDescriptor {
    fn from(_model: &ViewModel) -> Self {
        // TODO Fix it
        WalletDescriptor::default()
    }
}

impl ViewModel {
    pub fn signer_by(&self, xpub: ExtendedPubKey) -> Option<&Signer> {
        self.signers.iter().find(|signer| signer.xpub == xpub)
    }

    pub fn derivation_for(&self, signer: &Signer) -> TrackingAccount {
        let path: Vec<ChildNumber> = self
            .scheme
            .to_account_derivation(
                signer.account.unwrap_or_default().into(),
                self.network.into(),
            )
            .into();
        TrackingAccount {
            seed_based: true,
            master: XpubRef::Fingerprint(signer.fingerprint),
            account_path: path
                .into_iter()
                .map(AccountStep::try_from)
                .collect::<Result<_, _>>()
                .expect("inconsistency in constructed derivation path"),
            account_xpub: signer.xpub,
            revocation_seal: None,
            terminal_path: vec![TerminalStep::Wildcard, TerminalStep::Wildcard],
        }
    }

    pub fn replace_signer(&mut self, signer: Signer) -> bool {
        self.signers.replace(signer).is_some()
    }

    pub fn update_signers(&mut self) {
        let known_xpubs = self
            .signers
            .iter()
            .map(|signer| signer.xpub)
            .collect::<BTreeSet<_>>();

        self.devices
            .iter()
            .filter(|(_, device)| !known_xpubs.contains(&device.default_xpub))
            .for_each(|(fingerprint, device)| {
                self.signers.insert(Signer::with_device(
                    *fingerprint,
                    device.clone(),
                    &self.scheme,
                    self.network,
                ));
            });

        self.update_descriptor();
    }

    pub fn update_descriptor(&mut self) {
        if self.signers.is_empty() {
            self.descriptor = None;
            return;
        }

        let k = self.signers.len();
        let accounts = self
            .signers
            .iter()
            .map(|signer| self.derivation_for(signer))
            .collect::<Vec<_>>();

        let key_policies = accounts
            .iter()
            .map(|key| Policy::Key(key.clone()))
            .collect::<Vec<_>>();
        let sig_conditions = (1..=k)
            .into_iter()
            .map(|n| (n, Policy::Threshold(k - n + 1, key_policies.clone())))
            .map(|(n, node)| {
                if n > 1 {
                    (
                        n,
                        Policy::And(vec![node, Policy::Older(10u32.pow(n as u32))]),
                    )
                } else {
                    (n, node)
                }
            })
            .collect::<Vec<_>>();

        let (policy, remnant) = sig_conditions.iter().rfold(
            (None, None)
                as (
                    Option<Policy<TrackingAccount>>,
                    Option<Policy<TrackingAccount>>,
                ),
            |(acc, prev), (index, pol)| match (acc, prev) {
                (None, None) if index % 2 == 1 => (None, Some(pol.clone())),
                (None, None) => (Some(pol.clone()), None),
                (None, Some(prev)) => (
                    Some(Policy::Or(vec![(*index, pol.clone()), (*index + 1, prev)])),
                    None,
                ),
                (Some(acc), None) => (
                    Some(Policy::Or(vec![(*index, pol.clone()), (*index + 1, acc)])),
                    None,
                ),
                _ => unreachable!(),
            },
        );
        let policy = policy
            .or(remnant)
            .expect("zero signing accounts must be filtered");
        let ms_witscript = policy
            .compile::<Segwitv0>()
            .expect("policy composition  is broken");

        let wsh = Wsh::new(ms_witscript).expect("miniscript composition is broken");

        self.descriptor = Some(match self.class {
            DescriptorClass::PreSegwit => {
                let ms = policy
                    .compile::<Legacy>()
                    .expect("policy composition  is broken");
                Descriptor::Sh(Sh::new(ms).expect("miniscript composition is broken"))
            }
            DescriptorClass::SegwitV0 => Descriptor::Wsh(wsh),
            DescriptorClass::NestedV0 => Descriptor::Sh(Sh::new_with_wsh(wsh)),
            DescriptorClass::TaprootC0 => {
                let mut unspendable_key =
                    secp256k1::PublicKey::from_secret_key(&SECP256K1, &secp256k1::ONE_KEY);
                unspendable_key
                    .add_exp_assign(
                        &SECP256K1,
                        &sha256::Hash::hash(&unspendable_key.serialize()),
                    )
                    .unwrap();
                let mut buf = Vec::with_capacity(78);
                buf.extend(if self.network.is_testnet() {
                    [0x04u8, 0x35, 0x87, 0xCF]
                } else {
                    [0x04u8, 0x88, 0xB2, 0x1E]
                });
                buf.extend([0u8; 5]); // depth + fingerprint
                buf.extend([0u8; 4]); // child no
                buf.extend(&unspendable_key.serialize()[1..]);
                buf.extend(&unspendable_key.serialize());
                let unspendable_xkey =
                    ExtendedPubKey::decode(&buf).expect("broken unspendable key construction");
                let unspendable = TrackingAccount {
                    seed_based: true,
                    master: XpubRef::Unknown,
                    account_path: vec![],
                    account_xpub: unspendable_xkey,
                    revocation_seal: None,
                    terminal_path: vec![TerminalStep::Wildcard, TerminalStep::Wildcard],
                };

                let (tap_tree, remnant) = sig_conditions
                    .into_iter()
                    .map(|(depth, pol)| {
                        (
                            depth,
                            pol.compile::<Tap>()
                                .expect("tapscript construction is broken"),
                        )
                    })
                    .rfold(
                        (None, None)
                            as (
                                Option<TapTree<TrackingAccount>>,
                                Option<Miniscript<TrackingAccount, Tap>>,
                            ),
                        |(tree, prev), (depth, ms)| match (tree, prev) {
                            (None, None) if depth % 2 == 1 => (None, Some(ms)),
                            (None, None) if depth % 2 == 1 => {
                                (Some(TapTree::Leaf(Arc::new(ms))), None)
                            }
                            (None, Some(ms2)) => (
                                Some(TapTree::Tree(
                                    Arc::new(TapTree::Leaf(Arc::new(ms))),
                                    Arc::new(TapTree::Leaf(Arc::new(ms2))),
                                )),
                                None,
                            ),
                            (Some(tree), None) => (
                                Some(TapTree::Tree(
                                    Arc::new(TapTree::Leaf(Arc::new(ms))),
                                    Arc::new(tree),
                                )),
                                None,
                            ),
                            _ => unreachable!(),
                        },
                    );

                let tap_tree = tap_tree.or_else(|| remnant.map(|ms| TapTree::Leaf(Arc::new(ms))));

                Descriptor::Tr(
                    Tr::new(unspendable, tap_tree).expect("taproot construction is broken"),
                )
            }
        });
    }

    pub fn toggle_descr_class(&mut self, class: DescriptorClass) -> bool {
        if self.class == class {
            false
        } else {
            self.class = class;
            self.update_descriptor();
            true
        }
    }
}
