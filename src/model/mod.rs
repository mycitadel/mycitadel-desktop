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

pub mod file;
mod template;
mod types;
mod ui;
mod wallet;
mod xkey;

pub use self::wallet::{
    DerivationStandardExt, Wallet, WalletDescriptor, WalletStandard, WalletState,
};
pub use file::FileDocument;
pub use template::{Requirement, WalletTemplate};
pub use types::{
    DescriptorClass, Error, HardwareDevice, HardwareList, OriginFormat, Ownership, PublicNetwork,
    Signer, SigsReq, SpendingCondition, TimelockReq,
};
pub use ui::Notification;
pub use xkey::{
    NonStandardDerivation, XpubDescriptor, XpubOrigin, XpubParseError, XpubRequirementError,
};
