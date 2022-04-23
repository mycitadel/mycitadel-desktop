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
use std::ops::Deref;

use bitcoin::util::bip32::{ChildNumber, DerivationPath, Fingerprint};
use bitcoin::Network;
use chrono::{DateTime, Utc};
use miniscript::descriptor::DescriptorType;
use wallet::descriptors::DescrVariants;
use wallet::hd::standards::DerivationBlockchain;
use wallet::hd::{
    Bip43, DerivationStandard, HardenedIndex, HardenedIndexExpected, TerminalStep, UnhardenedIndex,
};
use wallet::psbt::Psbt;
use wallet::slip132::KeyApplication;

use super::{
    DescriptorClass, PublicNetwork, Signer, SigsReq, TimelockReq, TimelockedSigs, XpubkeyCore,
};

// TODO: Move to bpro library
#[derive(Getters, Clone, Debug, Default)]
#[derive(StrictEncode, StrictDecode)]
pub struct Wallet {
    #[getter(skip)]
    descriptor: WalletDescriptor,
    state: WalletState,
    history: Vec<Psbt>,
    wip: Vec<Psbt>,
}

impl Wallet {
    pub fn with(descriptor: WalletDescriptor) -> Self {
        Wallet {
            descriptor,
            ..default!()
        }
    }

    pub fn as_descriptor(&self) -> &WalletDescriptor {
        &self.descriptor
    }

    pub fn to_descriptor(&self) -> WalletDescriptor {
        self.descriptor.clone()
    }

    pub fn into_descriptor(self) -> WalletDescriptor {
        self.descriptor
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

#[derive(Getters, Clone, PartialEq, Eq, Hash, Debug, Default)]
#[derive(StrictEncode, StrictDecode)]
pub struct WalletDescriptor {
    network: PublicNetwork,
    signers: BTreeSet<Signer>,
    core: WalletCore,
}

impl Deref for WalletDescriptor {
    type Target = WalletCore;

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
pub struct WalletCore {
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
    /// Deterministic information about signing keys
    pub(self) signing_keys: BTreeSet<XpubkeyCore>,
    /// DFS-ordered alternative spending conditions.
    pub(self) spending_conditions: BTreeSet<(u8, SpendingCondition)>,
}

impl WalletDescriptor {
    pub fn with(
        signers: impl IntoIterator<Item = Signer>,
        spending_conditions: impl IntoIterator<Item = (u8, SpendingCondition)>,
        descriptor_classes: impl IntoIterator<Item = DescriptorClass>,
        terminal: Vec<TerminalStep>,
        network: PublicNetwork,
    ) -> Result<WalletDescriptor, DescriptorError> {
        let mut descriptor = WalletDescriptor {
            signers: empty!(),
            network,
            core: WalletCore {
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
        if !self.core.signing_keys.insert(signer.xpub.into()) {
            return Err(DescriptorError::DuplicateSigner(
                signer.name.clone(),
                signer.fingerprint(),
            ));
        }
        self.signers.insert(signer);
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
        !self.signers.insert(signer)
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
