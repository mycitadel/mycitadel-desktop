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

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};
use std::ops::{Deref, RangeInclusive};

use bitcoin::secp256k1::SECP256K1;
use bitcoin::util::bip32::{ChildNumber, DerivationPath, Fingerprint};
use bitcoin::{Address, BlockHash, Network, PublicKey, Transaction, Txid};
use chrono::{DateTime, Utc};
use electrum_client::HeaderNotification;
use miniscript::descriptor::{DescriptorType, Sh, Wsh};
use miniscript::policy::concrete::Policy;
use miniscript::{Descriptor, Legacy, Segwitv0, Tap};
use strict_encoding::{StrictDecode, StrictEncode};
use wallet::descriptors::DescrVariants;
use wallet::hd::standards::DerivationBlockchain;
use wallet::hd::{
    Bip43, DerivationStandard, Descriptor as DescriptorExt, HardenedIndex, HardenedIndexExpected,
    SegmentIndexes, TerminalStep, TrackingAccount, UnhardenedIndex,
};
use wallet::locks::{LockTime, SeqNo};
use wallet::onchain::{ResolveTx, TxResolverError};
use wallet::psbt::Psbt;
use wallet::scripts::address::AddressCompat;
use wallet::scripts::PubkeyScript;
use wallet::slip132::KeyApplication;

use super::{
    DescriptorClass, PublicNetwork, Signer, SigsReq, TimelockReq, TimelockedSigs, ToTapTree,
    Unsatisfiable, XpubkeyCore,
};
use crate::model::{ElectrumSec, ElectrumServer, Prevout};
use crate::worker::{HistoryTxid, UtxoTxid};

// TODO: Move to bpro library
#[derive(Getters, Clone, Debug, Default)]
#[derive(StrictEncode, StrictDecode)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate")
)]
pub struct Wallet {
    #[getter(skip)]
    settings: WalletSettings,

    last_indexes: BTreeMap<UnhardenedIndex, UnhardenedIndex>,

    #[getter(as_copy)]
    last_block: BlockHash,

    #[getter(as_copy)]
    height: u32,

    #[getter(as_copy)]
    state: WalletState,

    ephemerals: WalletEphemerals,

    utxos: Vec<UtxoTxid>,

    history: Vec<HistoryTxid>,

    transactions: BTreeMap<Txid, Transaction>,

    wip: Vec<Psbt>,
}

impl Wallet {
    pub fn with(settings: WalletSettings) -> Self {
        Wallet {
            settings,
            ..default!()
        }
    }

    pub fn as_settings(&self) -> &WalletSettings {
        &self.settings
    }

    pub fn to_settings(&self) -> WalletSettings {
        self.settings.clone()
    }

    pub fn into_settings(self) -> WalletSettings {
        self.settings
    }

    pub fn tx_count(&self) -> usize {
        self.transactions.len()
    }

    pub fn next_default_index(&self) -> UnhardenedIndex {
        *self
            .last_indexes
            .get(&UnhardenedIndex::zero())
            .unwrap_or(&UnhardenedIndex::zero())
    }

    pub fn next_change_index(&self) -> UnhardenedIndex {
        *self
            .last_indexes
            .get(&UnhardenedIndex::one())
            .unwrap_or(&UnhardenedIndex::zero())
    }

    pub fn next_address(&self) -> Address {
        let (descriptor, _) = self
            .as_settings()
            .descriptors_all()
            .expect("invalid wallet descriptor");
        DescriptorExt::<PublicKey>::address(
            &descriptor,
            &SECP256K1,
            &[UnhardenedIndex::zero(), self.next_default_index()],
        )
        .expect("unable to derive address for the wallet descriptor")
    }

    // TODO: Implement multiple coinselect algorithms
    pub fn coinselect(&self, value: u64) -> Option<(BTreeSet<Prevout>, u64)> {
        let mut prevouts = self.utxos.iter().map(Prevout::from).collect::<Vec<_>>();
        prevouts.sort_by_key(|p| p.amount);
        let mut acc = 0u64;
        let prevouts = prevouts
            .into_iter()
            .take_while(|p| {
                acc += p.amount;
                acc < value
            })
            .collect();
        if acc < value {
            None
        } else {
            Some((prevouts, acc))
        }
    }

    pub fn address_info(&self) -> Vec<AddressInfo> {
        let addresses = self
            .utxos
            .iter()
            .map(|utxo| AddressInfo {
                address: utxo.address,
                balance: utxo.value,
                volume: utxo.value,
                tx_count: 1,
                index: utxo.index,
                change: utxo.change,
            })
            .fold(
                BTreeMap::<AddressCompat, AddressInfo>::new(),
                |mut list, info| {
                    match list.entry(info.address) {
                        Entry::Vacant(entry) => entry.insert(info),
                        Entry::Occupied(entry) => {
                            let info2 = entry.into_mut();
                            info2.balance += info.balance;
                            info2.volume += info.volume;
                            info2.tx_count += 1;
                            info2
                        }
                    };
                    list
                },
            );

        let addresses = self
            .history
            .iter()
            .map(|item| AddressInfo {
                address: item.address,
                balance: 0,
                volume: 0, // TODO: Update from transaction information
                tx_count: 1,
                index: item.index,
                change: item.change,
            })
            .fold(addresses, |mut list, info| {
                match list.entry(info.address) {
                    Entry::Vacant(entry) => entry.insert(info),
                    Entry::Occupied(entry) => {
                        let info2 = entry.into_mut();
                        info2.volume += info.volume;
                        info2.tx_count += 1;
                        info2
                    }
                };
                list
            });

        addresses.into_values().collect()
    }

    pub fn update_signers(
        &mut self,
        signers: impl IntoIterator<Item = Signer>,
    ) -> Result<u16, DescriptorError> {
        self.settings.update_signers(signers)
    }

    pub fn add_descriptor_class(&mut self, descriptor_class: DescriptorClass) -> bool {
        self.settings.add_descriptor_class(descriptor_class)
    }

    pub fn update_last_block(&mut self, last_block: &HeaderNotification) {
        self.last_block = last_block.header.block_hash();
        self.height = last_block.height as u32;
    }

    pub fn update_fees(&mut self, f0: f64, f1: f64, f2: f64) {
        self.ephemerals.fees = (f0 as f32, f1 as f32, f2 as f32);
    }

    pub fn update_history(&mut self, batch: Vec<HistoryTxid>) {
        let txids: BTreeSet<_> = self.history.iter().map(|item| item.txid).collect();
        for item in batch {
            if !txids.contains(&item.txid) {
                self.history.push(item);
            }
        }
    }

    pub fn update_utxos(&mut self, batch: Vec<UtxoTxid>) {
        let txids: BTreeSet<_> = self.utxos.iter().map(|item| item.txid).collect();
        let mut balance = 0u64;
        for item in batch {
            if !txids.contains(&item.txid) {
                balance += item.value;
                self.utxos.push(item);
            }
        }
        self.state.balance += balance;
    }

    pub fn update_transactions(&mut self, batch: BTreeMap<Txid, Transaction>) {
        self.transactions.extend(batch);
        self.state.volume = 0;
        for tx in self.transactions.values() {
            // TODO: Fix algorithm
            self.state.volume += tx.output.iter().map(|out| out.value).sum::<u64>();
        }
    }

    pub fn update_electrum(&mut self, electrum: ElectrumServer) -> bool {
        self.settings.update_electrum(electrum)
    }
}

impl ResolveTx for Wallet {
    fn resolve_tx(&self, txid: Txid) -> Result<Transaction, TxResolverError> {
        self.transactions
            .get(&txid)
            .cloned()
            .ok_or(TxResolverError::with(txid))
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
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate")
)]
pub struct WalletSettings {
    #[getter(as_copy)]
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
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate")
)]
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
        if let Some(index) = self.signers.iter().position(|s| s == &signer) {
            self.signers[index] = signer;
            true
        } else {
            false
        }
    }

    pub fn update_electrum(&mut self, electrum: ElectrumServer) -> bool {
        if self.electrum != electrum {
            self.electrum = electrum;
            true
        } else {
            false
        }
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

    pub fn script_pubkeys(
        &self,
        change: bool,
        range: RangeInclusive<u16>,
    ) -> Result<BTreeMap<UnhardenedIndex, PubkeyScript>, miniscript::Error> {
        let (descriptor, _) = self.descriptors_all()?;
        let len = DescriptorExt::<PublicKey>::derive_pattern_len(&descriptor)
            .expect("internal inconsistency in wallet descriptor");
        debug_assert!(len >= 2);
        let mut pat = vec![UnhardenedIndex::zero(); len];
        pat[len - 2] = if change {
            UnhardenedIndex::one()
        } else {
            UnhardenedIndex::zero()
        };
        range
            .map(UnhardenedIndex::from)
            .map(|index| -> Result<_, _> {
                pat[len - 1] = index;
                Ok((
                    index,
                    DescriptorExt::<PublicKey>::script_pubkey(&descriptor, &SECP256K1, &pat)
                        .map_err(|_| {
                            miniscript::Error::BadDescriptor(s!("unable to derive script pubkey"))
                        })?
                        .into(),
                ))
            })
            .collect()
    }

    pub fn addresses(
        &self,
        change: bool,
        range: RangeInclusive<u16>,
    ) -> Result<BTreeMap<UnhardenedIndex, AddressCompat>, miniscript::Error> {
        self.script_pubkeys(change, range)?
            .into_iter()
            .map(|(index, spk)| -> Result<_, _> {
                Ok((
                    index,
                    AddressCompat::from_script(&spk, self.network.into()).ok_or(
                        miniscript::Error::BadDescriptor(s!("address can't be generated")),
                    )?,
                ))
            })
            .collect()
    }
}

#[derive(
    Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Display, From
)]
#[derive(StrictEncode, StrictDecode)]
#[display(inner)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate")
)]
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

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct AddressInfo {
    pub address: AddressCompat,
    pub balance: u64,
    pub tx_count: u32,
    pub volume: u64,
    pub index: UnhardenedIndex,
    pub change: bool,
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
#[derive(StrictEncode, StrictDecode)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate")
)]
pub struct WalletState {
    pub balance: u64,
    pub volume: u64,
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug, Default)]
#[derive(StrictEncode, StrictDecode)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate")
)]
pub struct Sats(u64);

#[derive(Clone, PartialEq, Debug, Default)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate")
)]
pub struct WalletEphemerals {
    pub fees: (f32, f32, f32),
    pub fiat: String,
    pub exchange_rate: f64,
}

impl StrictEncode for WalletEphemerals {
    fn strict_encode<E: Write>(&self, mut e: E) -> Result<usize, strict_encoding::Error> {
        Ok(
            strict_encode_list!(e; self.fees.0, self.fees.1, self.fees.2, self.fiat, self.exchange_rate),
        )
    }
}

impl StrictDecode for WalletEphemerals {
    fn strict_decode<D: Read>(mut d: D) -> Result<Self, strict_encoding::Error> {
        Ok(WalletEphemerals {
            fees: (
                f32::strict_decode(&mut d)?,
                f32::strict_decode(&mut d)?,
                f32::strict_decode(&mut d)?,
            ),
            fiat: String::strict_decode(&mut d)?,
            exchange_rate: f64::strict_decode(&mut d)?,
        })
    }
}
