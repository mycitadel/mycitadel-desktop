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

use crate::model::DescriptorClass;
use bitcoin::util::bip32::{ChildNumber, DerivationPath, Fingerprint};
use bitcoin::Network;
use miniscript::descriptor::DescriptorType;
use std::collections::BTreeSet;
use wallet::descriptors::DescrVariants;
use wallet::hd::standards::DerivationBlockchain;
use wallet::hd::{
    Bip43, DerivationStandard, HardenedIndex, HardenedIndexExpected, UnhardenedIndex,
};
use wallet::psbt::Psbt;
use wallet::slip132::KeyApplication;

use super::{PublicNetwork, Signer, SigsReq, SpendingCondition};

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

    pub fn set_descriptor(&mut self, descr: WalletDescriptor) {
        self.state = WalletState::default();
        self.history.clear();
        self.wip.clear();
        self.descriptor = descr;
    }
}

#[derive(
    Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug, Display, Error
)]
#[display(doc_comments)]
pub enum DescriptorError {
    /// spending condition {0} references unknown signer {1}.
    UnknownSigner(SpendingCondition, Fingerprint),
    /// unable to add spending condition when no signers are present.
    NoSigners,
    /// duplicated spending condition {0}.
    DuplicateCondition(SpendingCondition),
    /// insufficient number of signers {0} to support spending condition {1} requirements.
    InsufficientSignerCount(u16, SpendingCondition),
}

#[derive(Getters, Clone, Debug, Default)]
#[derive(StrictEncode, StrictDecode)]
pub struct WalletDescriptor {
    format: WalletStandard,
    signers: BTreeSet<Signer>,
    conditions: Vec<SpendingCondition>,
    network: PublicNetwork,
}

impl WalletDescriptor {
    fn push_condition(&mut self, condition: SpendingCondition) -> Result<(), DescriptorError> {
        if self.signers.is_empty() {
            return Err(DescriptorError::NoSigners);
        }
        if self.conditions.contains(&condition) {
            return Err(DescriptorError::DuplicateCondition(condition));
        }
        let signer_count = self.signers.len();
        match condition.sigs {
            SigsReq::AtLeast(n) if (n as usize) < signer_count => {
                Err(DescriptorError::InsufficientSignerCount(n, condition))
            }
            SigsReq::Specific(signer)
                if self
                    .signers
                    .iter()
                    .find(|s| s.master_fp == signer)
                    .is_none() =>
            {
                Err(DescriptorError::UnknownSigner(condition, signer))
            }
            _ => {
                self.conditions.push(condition);
                Ok(())
            }
        }
    }

    fn add_signer(&mut self, signer: Signer) -> bool {
        self.signers.insert(signer)
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
