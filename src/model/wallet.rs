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
use bitcoin::util::bip32::Fingerprint;
use std::collections::BTreeSet;
use wallet::descriptors::DescrVariants;
use wallet::hd::Bip43;
use wallet::psbt::Psbt;

use super::{PublicNetwork, Signer, SigsReq, SpendingCondition};

// TODO: Move to citadel-runtime
#[derive(Getters, Clone, Eq, PartialEq, Debug, Default)]
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

#[derive(Getters, Clone, Eq, PartialEq, Hash, Debug, Default)]
pub struct WalletDescriptor {
    format: WalletFormat,
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
                    .find(|s| s.fingerprint == signer)
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

// TODO: Move to descriptor wallet library
#[derive(Clone, Eq, PartialEq, Hash, Debug, From)]
pub enum WalletFormat {
    #[from]
    LnpBp(DescrVariants),
    #[from]
    Bip43(Bip43),
}

impl Default for WalletFormat {
    fn default() -> Self {
        WalletFormat::Bip43(Bip43::Bip48Native)
    }
}

pub trait WalletFormatExt {
    fn descriptor_class(&self) -> Option<DescriptorClass>;
}

impl WalletFormatExt for Bip43 {
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
pub struct WalletState {
    balance: Sats,
}

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug, Default)]
pub struct Sats(u64);
