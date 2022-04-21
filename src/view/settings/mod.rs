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

mod component;
pub(self) mod spending_row;
mod view_model;
mod widget;
pub(self) mod xpub_dlg;

pub use component::Component;
pub(self) use view_model::ViewModel;
pub(self) use widget::Widgets;

use bitcoin::util::bip32::{ExtendedPubKey, Fingerprint};
use relm::StreamHandle;

use crate::model::{DescriptorClass, HardwareDevice, WalletDescriptor, WalletTemplate};
use crate::view::{launch, wallet};

#[derive(Msg)]
pub enum Msg {
    New(Option<WalletTemplate>),
    View(WalletDescriptor),
    AddDevices,
    AddReadOnly,
    AddXpub(ExtendedPubKey),
    SignerAddDevice(Fingerprint, HardwareDevice),
    SignerSelect,
    SignerFingerprintChange,
    SignerNameChange,
    SignerOwnershipChange,
    ConditionAdd,
    ConditionRemove,
    ConditionSelect,
    ConditionChange,
    ToggleClass(DescriptorClass),
    ExportFormat(bool),
    Update,
    Close,
    SetWallet(StreamHandle<wallet::Msg>),
    SetLauncher(StreamHandle<launch::Msg>),
}
