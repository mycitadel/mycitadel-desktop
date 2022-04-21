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
use wallet::hd::{DerivationScheme, SegmentIndexes};
use wallet::psbt::Psbt;
use wallet::slip132;

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

/// BIP43-based purpose fields and derivation paths formats defined by them.
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug, Display)]
// TODO: Move to descriptor wallet library
pub enum Bip43 {
    /// Account-based P2PKH derivation
    ///
    /// `m / 44' / coin_type' / account'`
    #[display("bip44", alt = "m/44h")]
    Bip44,

    /// Cosigner-index-based multisig derivation
    ///
    /// `m / 45' / cosigner_index`
    #[display("bip45", alt = "m/45h")]
    Bip45,

    /// Account-based multisig derivation with sorted keys & P2WSH nested scripts
    ///
    /// `m / 48' / coin_type' / account' / script_type'`
    #[display("bip48-nested", alt = "m/48h//1h")]
    Bip48Nested,

    /// Account-based multisig derivation with sorted keys & P2WSH native scripts
    ///
    /// `m / 48' / coin_type' / account' / script_type'`
    #[display("bip48-native", alt = "m/48h//2h")]
    Bip48Native,

    /// Account-based legacy P2WPKH-in-P2SH derivation
    ///
    /// `m / 49' / coin_type' / account'`
    #[display("bip49", alt = "m/49h")]
    Bip49,

    /// Account-based native P2WPKH derivation
    ///
    /// `m / 84' / coin_type' / account'`
    #[display("bip84", alt = "m/84h")]
    Bip84,

    /// Account-based single-key P2TR derivation
    ///
    /// `m / 86' / coin_type' / account'`
    #[display("bip86", alt = "m/86h")]
    Bip86,

    /// Account- & descriptor-based derivation for multi-sig wallets
    #[display("bip87", alt = "m/87h")]
    ///
    /// `m / 87' / coin_type' / account'`
    Bip87,
}

#[derive(
    Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug, Display, Error
)]
#[display("unsupported derivation scheme")]
pub struct UnsupportedDerivation;

impl TryFrom<&DerivationScheme> for Bip43 {
    type Error = UnsupportedDerivation;

    fn try_from(scheme: &DerivationScheme) -> Result<Self, UnsupportedDerivation> {
        Ok(match scheme {
            DerivationScheme::Bip44 => Bip43::Bip44,
            DerivationScheme::Bip84 => Bip43::Bip84,
            DerivationScheme::Bip49 => Bip43::Bip49,
            DerivationScheme::Bip86 => Bip43::Bip86,
            DerivationScheme::Bip45 => Bip43::Bip45,
            DerivationScheme::Bip48 { script_type } if script_type.first_index() == 2 => {
                Bip43::Bip48Native
            }
            DerivationScheme::Bip48 { script_type } if script_type.first_index() == 1 => {
                Bip43::Bip48Nested
            }
            DerivationScheme::Bip87 => Bip43::Bip87,
            _ => return Err(UnsupportedDerivation),
        })
    }
}

impl Bip43 {
    pub fn singlesig_pkh() -> Bip43 {
        Bip43::Bip44
    }
    pub fn singlesig_nested0() -> Bip43 {
        Bip43::Bip49
    }
    pub fn singlesig_segwit0() -> Bip43 {
        Bip43::Bip84
    }
    pub fn singlelsig_taproot() -> Bip43 {
        Bip43::Bip86
    }
    pub fn multisig_ordered_sh() -> Bip43 {
        Bip43::Bip45
    }
    pub fn multisig_nested0() -> Bip43 {
        Bip43::Bip48Nested
    }
    pub fn multisig_segwit0() -> Bip43 {
        Bip43::Bip48Native
    }
    pub fn multisig_descriptor() -> Bip43 {
        Bip43::Bip87
    }

    pub fn descriptor_class(&self) -> Option<DescriptorClass> {
        Some(match self {
            Bip43::Bip44 => DescriptorClass::PreSegwit,
            Bip43::Bip45 => DescriptorClass::PreSegwit,
            Bip43::Bip48Nested => DescriptorClass::NestedV0,
            Bip43::Bip48Native => DescriptorClass::SegwitV0,
            Bip43::Bip49 => DescriptorClass::NestedV0,
            Bip43::Bip84 => DescriptorClass::SegwitV0,
            Bip43::Bip86 => DescriptorClass::TaprootC0,
            Bip43::Bip87 => return None,
        })
    }

    pub fn slip_application(self) -> Option<slip132::KeyApplication> {
        Some(match self {
            Bip43::Bip44 => slip132::KeyApplication::Hashed,
            Bip43::Bip45 => return None,
            Bip43::Bip48Nested => slip132::KeyApplication::NestedMultisig,
            Bip43::Bip48Native => slip132::KeyApplication::SegWitMiltisig,
            Bip43::Bip49 => slip132::KeyApplication::Nested,
            Bip43::Bip84 => slip132::KeyApplication::SegWit,
            Bip43::Bip86 => return None,
            Bip43::Bip87 => return None,
        })
    }
}

#[derive(Clone, Eq, PartialEq, Hash, Debug, Default)]
pub struct WalletState {
    balance: Sats,
}

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug, Default)]
pub struct Sats(u64);
