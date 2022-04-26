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
pub(self) use view_model::{ElectrumModel, ElectrumPreset, ViewModel};
pub(self) use widget::Widgets;

use std::path::PathBuf;

use bitcoin::util::bip32::{ExtendedPubKey, Fingerprint};
use gtk::ResponseType;
use relm::StreamHandle;

use crate::model::{
    DescriptorClass, ElectrumSec, HardwareDevice, PublicNetwork, WalletSettings, WalletTemplate,
};
use crate::view::{launch, wallet};

#[derive(Msg)]
pub enum Msg {
    New(Option<WalletTemplate>, PathBuf),
    View(WalletSettings, PathBuf),
    AddDevices,
    AddReadOnly,
    RemoveSigner,
    SignerAddXpub(ExtendedPubKey),
    SignerAddDevice(Fingerprint, HardwareDevice),
    SignerSelect,
    SignerOriginUpdate,
    SignerFingerprintChange,
    SignerNameChange,
    SignerOwnershipChange,
    SignerAccountChange,
    ConditionAdd,
    ConditionRemove,
    ConditionSelect,
    ConditionChange,
    NetworkChange(PublicNetwork),
    ToggleClass(DescriptorClass),
    ExportFormat(bool),
    ElectrumSelect(ElectrumPreset),
    ElectrumEdit,
    ElectrumPortChange,
    ElectrumSecChange(ElectrumSec),
    ElectrumTest,
    ElectrumTestOk,
    ElectrumTestFailed(String),
    Response(ResponseType),
    SetWallet(StreamHandle<wallet::Msg>),
    SetLauncher(StreamHandle<launch::Msg>),
    NoOp,
}
