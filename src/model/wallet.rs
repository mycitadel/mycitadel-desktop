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

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Deref;

use crate::model::{ElectrumSec, ElectrumServer};
use bitcoin::secp256k1::SECP256K1;
use bitcoin::util::bip32::{ChildNumber, DerivationPath, Fingerprint};
use bitcoin::{Address, Network, PublicKey};
use chrono::{DateTime, Utc};
use miniscript::descriptor::{DescriptorType, Sh, Wsh};
use miniscript::policy::concrete::Policy;
use miniscript::{Descriptor, Legacy, Segwitv0, Tap};
use wallet::descriptors::DescrVariants;
use wallet::hd::standards::DerivationBlockchain;
use wallet::hd::{
    Bip43, DerivationStandard, Descriptor as DescriptorExt, HardenedIndex, HardenedIndexExpected,
    SegmentIndexes, TerminalStep, TrackingAccount, UnhardenedIndex,
};
use wallet::locks::{LockTime, SeqNo};
use wallet::psbt::Psbt;
use wallet::slip132::KeyApplication;

use super::{
    DescriptorClass, PublicNetwork, Signer, SigsReq, TimelockReq, TimelockedSigs, ToTapTree,
    Unsatisfiable, XpubkeyCore,
};

// TODO: Move to bpro library
#[derive(Getters, Clone, Debug, Default)]
#[derive(StrictEncode, StrictDecode)]
pub struct Wallet {
    #[getter(skip)]
    descriptor: WalletSettings,
    last_indexes: BTreeMap<UnhardenedIndex, UnhardenedIndex>,
    state: WalletState,
    history: Vec<Psbt>,
    wip: Vec<Psbt>,
}

impl Wallet {
    pub fn with(descriptor: WalletSettings) -> Self {
        Wallet {
            descriptor,
            ..default!()
        }
    }

    pub fn as_descriptor(&self) -> &WalletSettings {
        &self.descriptor
    }

    pub fn to_descriptor(&self) -> WalletSettings {
        self.descriptor.clone()
    }

    pub fn into_descriptor(self) -> WalletSettings {
        self.descriptor
    }

    pub fn next_default_index(&self) -> UnhardenedIndex {
        *self
            .last_indexes
            .get(&UnhardenedIndex::zero())
            .unwrap_or(&UnhardenedIndex::zero())
    }

    pub fn next_address(&self) -> Address {
        let (descriptor, _) = self
            .as_descriptor()
            .descriptors_all()
            .expect("invalid wallet descriptor");
        DescriptorExt::<PublicKey>::address(
            &descriptor,
            &SECP256K1,
            &[UnhardenedIndex::zero(), self.next_default_index()],
        )
        .expect("unable to derive address for the wallet descriptor")
    }

    pub fn update_signers(
        &mut self,
        signers: impl IntoIterator<Item = Signer>,
    ) -> Result<u16, DescriptorError> {
        self.descriptor.update_signers(signers)
    }

    pub fn add_descriptor_class(&mut self, descriptor_class: DescriptorClass) -> bool {
        self.descriptor.add_descriptor_class(descriptor_class)
    }
}

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug, Display, Error)]
#[display(doc_comments)]
pub enum DescriptorError {
    /// signer with fingerprint {0} is not part of the wallet descriptor.
    UnknownSigner(Fingerprint),
    /// spending condition {0} references unknown signer with fingerprint {1}.
    UnknownConditionSigner(SpendingCondition, Fingerprint),
    /// no signers present.
    NoSigners,
    /// no spending conditions present.
    NoConditions,
    /// no information about scriptPubkey construction present.
    NoDescriptorClasses,
    /// duplicated spending condition {1} at depth {0}.
    DuplicateCondition(u8, SpendingCondition),
    /// signer {0} key with fingerprint {1} is already present among signers.
    DuplicateSigner(String, Fingerprint),
    /// insufficient number of signers {0} to support spending condition {1} requirements.
    InsufficientSignerCount(u16, SpendingCondition),
}

#[derive(Getters, Clone, PartialEq, Eq, Hash, Debug)]
#[derive(StrictEncode, StrictDecode)]
pub struct WalletSettings {
    network: PublicNetwork,
    core: WalletDescriptor,
    signers: Vec<Signer>,
    electrum: ElectrumServer,
}

impl Default for WalletSettings {
    fn default() -> Self {
        WalletSettings {
            network: PublicNetwork::default(),
            core: default!(),
            signers: empty!(),
            electrum: ElectrumServer {
                sec: ElectrumSec::Tls,
                server: s!("electrum.blockstream.info"),
                port: PublicNetwork::default().electrum_port(),
            },
        }
    }
}

impl Deref for WalletSettings {
    type Target = WalletDescriptor;

    fn deref(&self) -> &Self::Target {
        &self.core
    }
}

/// Wallet descriptor defines a deterministic part of the wallet. It fully controls how different
/// scriptPubkeys can be formed and satisfied. Change in any of wallet descriptor parameters will
/// create a different set of addresses/scriptPubkeys/satisfactions, thus changing the wallet.
///
/// Tagged hash of strict-encoded wallet descriptor data operates as a globally unique wallet
/// descriptor.
#[derive(Getters, Clone, PartialEq, Eq, Hash, Debug, Default)]
#[derive(StrictEncode, StrictDecode)]
pub struct WalletDescriptor {
    /// We commit to the information whether
    pub(self) testnet: bool,
    /// We operate set of descriptor types, such that each wallet can produce addresses of different
    /// kind, depending on the co-signer software and supported addresses by the payee.
    ///
    /// Each descriptor type defines a deterministic way of transforming DFS-ordered tree of
    /// spending conditions into a scriptPubkey. In case of taproot, this means that the descriptor
    /// type must also define how the key path spending condition is constructed (aggregated key or
    /// a unsatisfiable condition).
    pub(self) descriptor_classes: BTreeSet<DescriptorClass>,
    /// Terminal defines a way how a public keys are derived from signing extended keys. Terminals
    /// always consists of unhardened indexes or unhardened index wildcards - and always contain
    /// at least one wildcard marking index position.
    // TODO: Define a TerminalPath type which will ensure that at least one wildcard is present
    pub(self) terminal: Vec<TerminalStep>,
    /// Deterministic information about signing keys.
    ///
    /// The order of keys matters only for BIP45, so we have maintain it using `Vec` instead of
    /// `BTreeSet`.
    pub(self) signing_keys: Vec<XpubkeyCore>,
    /// DFS-ordered alternative spending conditions.
    pub(self) spending_conditions: BTreeSet<(u8, SpendingCondition)>,
}

impl WalletSettings {
    pub fn with(
        signers: impl IntoIterator<Item = Signer>,
        spending_conditions: impl IntoIterator<Item = (u8, SpendingCondition)>,
        descriptor_classes: impl IntoIterator<Item = DescriptorClass>,
        terminal: Vec<TerminalStep>,
        network: PublicNetwork,
        electrum: ElectrumServer,
    ) -> Result<WalletSettings, DescriptorError> {
        let mut descriptor = WalletSettings {
            signers: empty!(),
            network,
            electrum,
            core: WalletDescriptor {
                testnet: network.is_testnet(),
                descriptor_classes: empty!(),
                terminal,
                signing_keys: empty!(),
                spending_conditions: empty!(),
            },
        };

        for signer in signers {
            descriptor.add_signer(signer)?;
        }

        for (depth, condition) in spending_conditions {
            descriptor.add_condition(depth, condition)?;
        }

        for class in descriptor_classes {
            descriptor.add_descriptor_class(class);
        }

        if descriptor.signers.is_empty() {
            return Err(DescriptorError::NoSigners);
        }

        if descriptor.core.spending_conditions.is_empty() {
            return Err(DescriptorError::NoConditions);
        }

        if descriptor.core.descriptor_classes.is_empty() {
            return Err(DescriptorError::NoDescriptorClasses);
        }

        Ok(descriptor)
    }

    fn add_descriptor_class(&mut self, class: DescriptorClass) -> bool {
        self.core.descriptor_classes.insert(class)
    }

    fn add_condition(
        &mut self,
        depth: u8,
        condition: impl Into<SpendingCondition>,
    ) -> Result<(), DescriptorError> {
        let condition = condition.into();

        if self.signers.is_empty() {
            return Err(DescriptorError::NoSigners);
        }
        if self.core.spending_conditions.contains(&(depth, condition)) {
            return Err(DescriptorError::DuplicateCondition(depth, condition));
        }
        let signer_count = self.signers.len();
        match condition {
            SpendingCondition::Sigs(ts) => match ts.sigs {
                SigsReq::AtLeast(n) if (n as usize) < signer_count => {
                    Err(DescriptorError::InsufficientSignerCount(n, condition))
                }
                SigsReq::Specific(signer_fp)
                    if self
                        .signers
                        .iter()
                        .find(|s| s.fingerprint() == signer_fp)
                        .is_none() =>
                {
                    Err(DescriptorError::UnknownConditionSigner(
                        condition, signer_fp,
                    ))
                }
                _ => {
                    self.core.spending_conditions.insert((depth, condition));
                    Ok(())
                }
            },
        }
    }

    fn add_signer(&mut self, signer: Signer) -> Result<(), DescriptorError> {
        let xpub = signer.xpub.into();
        if self.core.signing_keys.contains(&xpub) {
            return Err(DescriptorError::DuplicateSigner(
                signer.name.clone(),
                signer.fingerprint(),
            ));
        }
        self.core.signing_keys.push(xpub);
        self.signers.push(signer);
        Ok(())
    }

    fn update_signers(
        &mut self,
        signers: impl IntoIterator<Item = Signer>,
    ) -> Result<u16, DescriptorError> {
        let mut count = 0u16;
        for signer in signers {
            let fingerprint = signer.fingerprint();
            if !self.update_signer(signer) {
                return Err(DescriptorError::UnknownSigner(fingerprint));
            }
            count += 1;
        }
        Ok(count)
    }

    fn update_signer(&mut self, signer: Signer) -> bool {
        if !self.signers.contains(&signer) {
            return false;
        }
        self.signers.push(signer);
        true
    }

    pub fn descriptors_all(
        &self,
    ) -> Result<
        (
            Descriptor<TrackingAccount>,
            Vec<Descriptor<TrackingAccount>>,
        ),
        miniscript::Error,
    > {
        let mut descriptors = self
            .descriptor_classes
            .iter()
            .map(|class| self.descriptor_for_class(*class));
        Ok((
            descriptors
                .next()
                .expect("wallet core without descriptor class")?,
            descriptors.collect::<Result<_, _>>()?,
        ))
    }

    pub fn descriptor_for_class(
        &self,
        class: DescriptorClass,
    ) -> Result<Descriptor<TrackingAccount>, miniscript::Error> {
        if self.signers.len() <= 1 {
            let first_key = self
                .signers
                .first()
                .ok_or(miniscript::Error::Unexpected(s!(
                    "wallet core does not contain any signers"
                )))?
                .to_tracking_account(self.terminal.clone());

            return Ok(match class {
                DescriptorClass::PreSegwit => Descriptor::new_pk(first_key),
                DescriptorClass::SegwitV0 => Descriptor::new_wpkh(first_key)?,
                DescriptorClass::NestedV0 => Descriptor::new_sh_wpkh(first_key)?,
                DescriptorClass::TaprootC0 => Descriptor::new_tr(first_key, None)?,
            });
        }

        // 1. Construct accounts
        let accounts: BTreeMap<Fingerprint, TrackingAccount> = self
            .signers
            .iter()
            .map(|signer| {
                (
                    signer.fingerprint(),
                    signer.to_tracking_account(self.terminal.clone()),
                )
            })
            .collect();

        // 2. Construct policy fragments
        let mut dfs_tree = self
            .spending_conditions
            .iter()
            .map(|(depth, cond)| (depth, cond.policy(&accounts)));

        // 3. Pack miniscript fragments according to the descriptor class
        if class == DescriptorClass::TaprootC0 {
            let tree = dfs_tree.try_fold::<_, _, Result<_, miniscript::Error>>(
                Vec::new(),
                |mut acc, (depth, policy)| {
                    acc.push((*depth, policy.compile::<Tap>()?));
                    Ok(acc)
                },
            )?;

            return Descriptor::new_tr(
                TrackingAccount::unsatisfiable((self.network, self.terminal.clone())),
                Some(tree.to_tap_tree().ok_or(miniscript::Error::Unexpected(s!(
                    "unable to construct TapTree from the given spending conditions"
                )))?),
            );
        }

        // Pack the tree into a linear structure
        let (policy, remnant) = dfs_tree.rfold(
            (None, None)
                as (
                    Option<Policy<TrackingAccount>>,
                    Option<Policy<TrackingAccount>>,
                ),
            |(acc, prev), (index, pol)| match (acc, prev) {
                (None, None) if index % 2 == 1 => (None, Some(pol.clone())),
                (None, None) => (Some(pol.clone()), None),
                (None, Some(prev)) => (
                    Some(Policy::Or(vec![
                        (*index as usize, pol.clone()),
                        (*index as usize + 1, prev),
                    ])),
                    None,
                ),
                (Some(acc), None) => (
                    Some(Policy::Or(vec![
                        (*index as usize, pol.clone()),
                        (*index as usize + 1, acc),
                    ])),
                    None,
                ),
                _ => unreachable!(),
            },
        );
        let policy = policy.or(remnant).ok_or(miniscript::Error::Unexpected(s!(
            "zero signing accounts must be filtered"
        )))?;

        if class.is_segwit_v0() {
            let ms_witscript = policy.compile::<Segwitv0>()?;
            let wsh = Wsh::new(ms_witscript)?;
            return Ok(match class {
                DescriptorClass::SegwitV0 => Descriptor::Wsh(wsh),
                DescriptorClass::NestedV0 => Descriptor::Sh(Sh::new_with_wsh(wsh)),
                _ => unreachable!(),
            });
        }

        let ms = policy.compile::<Legacy>()?;
        Ok(Descriptor::Sh(Sh::new(ms)?))
    }
}

#[derive(
    Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Display, From
)]
#[derive(StrictEncode, StrictDecode)]
#[display(inner)]
pub enum SpendingCondition {
    #[from]
    Sigs(TimelockedSigs),
    // In a future we may add custom script types
}

impl Default for SpendingCondition {
    fn default() -> Self {
        SpendingCondition::Sigs(default!())
    }
}

impl SpendingCondition {
    pub fn all() -> SpendingCondition {
        SpendingCondition::Sigs(TimelockedSigs {
            sigs: SigsReq::All,
            timelock: TimelockReq::Anytime,
        })
    }

    pub fn at_least(sigs: u16) -> SpendingCondition {
        SpendingCondition::Sigs(TimelockedSigs {
            sigs: SigsReq::AtLeast(sigs),
            timelock: TimelockReq::Anytime,
        })
    }

    pub fn anybody_after_date(date: DateTime<Utc>) -> SpendingCondition {
        SpendingCondition::Sigs(TimelockedSigs {
            sigs: SigsReq::Any,
            timelock: TimelockReq::AfterTime(date),
        })
    }

    pub fn after_date(sigs: SigsReq, date: DateTime<Utc>) -> SpendingCondition {
        SpendingCondition::Sigs(TimelockedSigs {
            sigs,
            timelock: TimelockReq::AfterTime(date),
        })
    }

    pub fn policy(
        &self,
        accounts: &BTreeMap<Fingerprint, TrackingAccount>,
    ) -> Policy<TrackingAccount> {
        let count = accounts.len();
        let key_policies = accounts.values().cloned().map(Policy::Key).collect();
        let sigs = match self {
            SpendingCondition::Sigs(TimelockedSigs {
                sigs: SigsReq::All, ..
            }) => Policy::Threshold(count, key_policies),
            SpendingCondition::Sigs(TimelockedSigs {
                sigs: SigsReq::Any, ..
            }) => Policy::Threshold(1, key_policies),
            SpendingCondition::Sigs(TimelockedSigs {
                sigs: SigsReq::AtLeast(k),
                ..
            }) => Policy::Threshold(*k as usize, key_policies),
            SpendingCondition::Sigs(TimelockedSigs {
                sigs: SigsReq::Specific(fp),
                ..
            }) => Policy::Key(
                accounts
                    .get(fp)
                    .expect("fingerprint is absent from the accounts")
                    .clone(),
            ),
        };
        let timelock = match self {
            SpendingCondition::Sigs(TimelockedSigs {
                timelock: TimelockReq::Anytime,
                ..
            }) => None,
            // TODO: Check that this is correct
            SpendingCondition::Sigs(TimelockedSigs {
                timelock: TimelockReq::AfterTime(datetime),
                ..
            }) => Some(Policy::After(
                LockTime::with_unix_timestamp(datetime.timestamp() as u32)
                    .unwrap()
                    .as_u32(),
            )),
            // TODO: Check that this is correct
            SpendingCondition::Sigs(TimelockedSigs {
                timelock: TimelockReq::AfterBlock(block),
                ..
            }) => Some(Policy::After(
                LockTime::with_height(*block).unwrap().as_u32(),
            )),
            // TODO: Check that this is correct
            SpendingCondition::Sigs(TimelockedSigs {
                timelock: TimelockReq::OlderTime(datetime),
                ..
            }) => Some(Policy::Older(
                SeqNo::with_time((datetime.timestamp() as u32 / 512) as u16).as_u32(),
            )),
            // TODO: Check that this is correct
            SpendingCondition::Sigs(TimelockedSigs {
                timelock: TimelockReq::OlderBlock(block),
                ..
            }) => Some(Policy::Older(SeqNo::with_height(*block).as_u32())),
        };

        timelock
            .map(|timelock| Policy::And(vec![sigs.clone(), timelock]))
            .unwrap_or(sigs)
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display, From)]
#[derive(StrictEncode, StrictDecode)]
#[display(inner)]
pub enum WalletStandard {
    #[from]
    LnpBp(DescrVariants),
    #[from]
    Bip43(Bip43),
}

impl Default for WalletStandard {
    fn default() -> Self {
        WalletStandard::Bip43(Bip43::Bip48Native)
    }
}

impl DerivationStandard for WalletStandard {
    fn deduce(derivation: &DerivationPath) -> Option<Self>
    where
        Self: Sized,
    {
        // TODO: Support LNPBP standard derivation
        Bip43::deduce(derivation).map(WalletStandard::Bip43)
    }

    fn matching(slip: KeyApplication) -> Option<Self>
    where
        Self: Sized,
    {
        Bip43::matching(slip).map(WalletStandard::Bip43)
    }

    fn purpose(&self) -> Option<HardenedIndex> {
        match self {
            // TODO: Support LNPBP standard derivation
            WalletStandard::LnpBp(_) => None,
            WalletStandard::Bip43(bip43) => bip43.purpose(),
        }
    }

    fn account_depth(&self) -> Option<u8> {
        match self {
            // TODO: Support LNPBP standard derivation
            WalletStandard::LnpBp(_) => None,
            WalletStandard::Bip43(bip43) => bip43.account_depth(),
        }
    }

    fn coin_type_depth(&self) -> Option<u8> {
        match self {
            // TODO: Support LNPBP standard derivation
            WalletStandard::LnpBp(_) => None,
            WalletStandard::Bip43(bip43) => bip43.coin_type_depth(),
        }
    }

    fn is_account_last_hardened(&self) -> Option<bool> {
        match self {
            // TODO: Support LNPBP standard derivation
            WalletStandard::LnpBp(_) => None,
            WalletStandard::Bip43(bip43) => bip43.is_account_last_hardened(),
        }
    }

    fn network(&self, path: &DerivationPath) -> Option<Result<Network, HardenedIndexExpected>> {
        match self {
            // TODO: Support LNPBP standard derivation
            WalletStandard::LnpBp(_) => None,
            WalletStandard::Bip43(bip43) => bip43.network(path),
        }
    }

    fn account_template_string(&self, blockchain: DerivationBlockchain) -> String {
        match self {
            // TODO: Support LNPBP standard derivation
            WalletStandard::LnpBp(_) => s!("m/"),
            WalletStandard::Bip43(bip43) => bip43.account_template_string(blockchain),
        }
    }

    fn to_origin_derivation(&self, blockchain: DerivationBlockchain) -> DerivationPath {
        match self {
            // TODO: Support LNPBP standard derivation
            WalletStandard::LnpBp(_) => empty!(),
            WalletStandard::Bip43(bip43) => bip43.to_origin_derivation(blockchain),
        }
    }

    fn to_account_derivation(
        &self,
        account_index: ChildNumber,
        blockchain: DerivationBlockchain,
    ) -> DerivationPath {
        match self {
            // TODO: Support LNPBP standard derivation
            WalletStandard::LnpBp(_) => empty!(),
            WalletStandard::Bip43(bip43) => bip43.to_account_derivation(account_index, blockchain),
        }
    }

    fn to_key_derivation(
        &self,
        account_index: ChildNumber,
        blockchain: DerivationBlockchain,
        index: UnhardenedIndex,
        case: Option<UnhardenedIndex>,
    ) -> DerivationPath {
        match self {
            // TODO: Support LNPBP standard derivation
            WalletStandard::LnpBp(_) => empty!(),
            WalletStandard::Bip43(bip43) => {
                bip43.to_key_derivation(account_index, blockchain, index, case)
            }
        }
    }

    fn descriptor_types(&self) -> &'static [DescriptorType] {
        match self {
            // TODO: Support LNPBP standard derivation
            WalletStandard::LnpBp(_) => &[],
            WalletStandard::Bip43(bip43) => bip43.descriptor_types(),
        }
    }

    fn slip_application(&self) -> Option<KeyApplication> {
        match self {
            // TODO: Support LNPBP standard derivation
            WalletStandard::LnpBp(_) => None,
            WalletStandard::Bip43(bip43) => bip43.slip_application(),
        }
    }
}

pub trait DerivationStandardExt: DerivationStandard {
    fn descriptor_class(&self) -> Option<DescriptorClass>;
}

impl DerivationStandardExt for Bip43 {
    fn descriptor_class(&self) -> Option<DescriptorClass> {
        Some(match self {
            Bip43::Bip44 => DescriptorClass::PreSegwit,
            Bip43::Bip45 => DescriptorClass::PreSegwit,
            Bip43::Bip48Nested => DescriptorClass::NestedV0,
            Bip43::Bip48Native => DescriptorClass::SegwitV0,
            Bip43::Bip49 => DescriptorClass::NestedV0,
            Bip43::Bip84 => DescriptorClass::SegwitV0,
            Bip43::Bip86 => DescriptorClass::TaprootC0,
            Bip43::Bip87 => return None,
            Bip43::Bip43 { .. } => return None,
            _ => return None,
        })
    }
}

#[derive(Clone, Eq, PartialEq, Hash, Debug, Default)]
#[derive(StrictEncode, StrictDecode)]
pub struct WalletState {
    balance: Sats,
}

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug, Default)]
#[derive(StrictEncode, StrictDecode)]
pub struct Sats(u64);
