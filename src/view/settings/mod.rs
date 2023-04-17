// MyCitadel desktop wallet: bitcoin & RGB wallet based on GTK framework.
//
// Written in 2022 by
//     Dr. Maxim Orlovsky <orlovsky@pandoraprime.ch>
//
// Copyright (C) 2022 by Pandora Prime SA, Switzerland.
//
// This software is distributed without any warranty. You should have received
// a copy of the AGPL-3.0 License along with this software. If not, see
// <https://www.gnu.org/licenses/agpl-3.0-standalone.html>.

mod component;
pub(self) mod spending_row;
mod view_model;
mod widget;
pub(self) mod xpub_dlg;

use std::path::PathBuf;

use ::wallet::descriptors::DescriptorClass;
use ::wallet::onchain::PublicNetwork;
use bitcoin::util::bip32::{ExtendedPubKey, Fingerprint};
use bpro::{ElectrumPreset, ElectrumSec, HardwareDevice, WalletSettings, WalletTemplate};
pub use component::Component;
use gtk::ResponseType;
use relm::StreamHandle;
pub(self) use view_model::{ElectrumModel, ViewModel};
pub(self) use widget::Widgets;

use crate::view::{launch, wallet};

#[derive(Msg)]
pub enum Msg {
    New(WalletTemplate, PathBuf),
    Duplicate(WalletSettings, PathBuf),
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
    EnableRgb,
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
