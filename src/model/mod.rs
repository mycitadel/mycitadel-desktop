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
pub mod psbt;
mod sign;
mod taptree;
mod template;
mod types;
mod ui;
mod wallet;

pub use electrum::{ElectrumPreset, ElectrumSec, ElectrumServer};
pub use file::FileDocument;
pub use onchain::{
    AddressSource, AddressSummary, AddressValue, HistoryEntry, OnchainStatus, OnchainTxid, Prevout,
    UtxoTxid,
};
pub use sign::XprivSigner;
pub use taptree::ToTapTree;
pub use template::{Requirement, WalletTemplate};
pub use types::{
    Error, HardwareDevice, HardwareList, OriginFormat, Ownership, Signer, SigsReq,
    TimelockDuration, TimelockReq, TimelockedSigs,
};
pub use ui::Notification;

pub use self::wallet::{
    DerivationStandardExt, DerivationType, DescriptorError, SpendingCondition, Wallet,
    WalletDescriptor, WalletEphemerals, WalletSettings, WalletState,
};
