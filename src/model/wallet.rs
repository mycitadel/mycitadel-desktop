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
use wallet::descriptors::DescrVariants;
use wallet::psbt::Psbt;

use crate::model::{PublicNetwork, Signer, SpendingCondition};

#[derive(Getters, Clone, Eq, PartialEq, Debug, Default)]
pub struct Wallet {
    #[getter(as_clone)]
    descriptor: WalletDescriptor,
    state: WalletState,
    history: Vec<Psbt>,
    wip: Vec<Psbt>,
}

#[derive(Clone, Eq, PartialEq, Hash, Debug, Default)]
pub struct WalletDescriptor {
    format: WalletFormat,
    signers: BTreeSet<Signer>,
    conditions: Vec<SpendingCondition>,
    network: PublicNetwork,
}

// TODO: Move to descriptor wallet library
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub enum WalletFormat {
    LnpBp(DescrVariants),
    Bip43(Bip43Format),
}

impl Default for WalletFormat {
    fn default() -> Self {
        WalletFormat::Bip43(Bip43Format::Bip48Native)
    }
}

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug, Display)]
// TODO: Move to descriptor wallet library
pub enum Bip43Format {
    /// Account-based P2PKH derivation
    ///
    /// `m / 44' / coin_type' / account'`
    #[display("bip44", alt = "m/44h")]
    Bip44,

    /// Account-based native P2WPKH derivation
    ///
    /// `m / 84' / coin_type' / account'`
    #[display("bip84", alt = "m/84h")]
    Bip84,

    /// Account-based legacy P2WPH-in-P2SH derivation
    ///
    /// `m / 49' / coin_type' / account'`
    #[display("bip49", alt = "m/49h")]
    Bip49,

    /// Account-based single-key P2TR derivation
    ///
    /// `m / 86' / coin_type' / account'`
    #[display("bip86", alt = "m/86h")]
    Bip86,

    /// Cosigner-index-based multisig derivation
    ///
    /// `m / 45' / cosigner_index`
    #[display("bip45", alt = "m/45h")]
    Bip45,

    /// Account-based multisig derivation with sorted keys & P2WSH native scripts
    ///
    /// `m / 48' / coin_type' / account' / script_type'`
    #[display("bip48-naive", alt = "m/48h//2h")]
    Bip48Native,

    /// Account-based multisig derivation with sorted keys & P2WSH nested scripts
    ///
    /// `m / 48' / coin_type' / account' / script_type'`
    #[display("bip48-nested", alt = "m/48h//1h")]
    Bip48Nested,

    /// Account- & descriptor-based derivation for multi-sig wallets
    #[display("bip87", alt = "m/87h")]
    ///
    /// `m / 87' / coin_type' / account'`
    Bip87,
}

#[derive(Clone, Eq, PartialEq, Hash, Debug, Default)]
pub struct WalletState {
    balance: Sats,
}

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug, Default)]
pub struct Sats(u64);
