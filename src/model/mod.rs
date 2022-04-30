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

mod electrum;
pub mod file;
mod onchain;
mod taptree;
mod template;
mod types;
mod ui;
mod unsatisfiable;
mod wallet;
mod xkey;

pub use electrum::{ElectrumSec, ElectrumServer};
pub use file::FileDocument;
pub use onchain::{
    AddressSource, AddressSummary, AddressValue, HistoryEntry, OnchainStatus, OnchainTxid, Prevout,
    UtxoTxid,
};
pub use taptree::ToTapTree;
pub use template::{Requirement, WalletTemplate};
pub use types::{
    DescriptorClass, Error, HardwareDevice, HardwareList, OriginFormat, Ownership, PublicNetwork,
    Signer, SigsReq, TimelockReq, TimelockedSigs,
};
pub use ui::Notification;
pub use unsatisfiable::Unsatisfiable;
pub use xkey::{
    NonStandardDerivation, XpubDescriptor, XpubOrigin, XpubParseError, XpubRequirementError,
    XpubkeyCore,
};

pub use self::wallet::{
    DerivationStandardExt, DerivationType, DescriptorError, SpendingCondition, Wallet,
    WalletDescriptor, WalletEphemerals, WalletSettings, WalletState,
};
