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
pub mod spending_row;
mod view_model;
mod widget;

pub use component::Component;
pub(self) use view_model::ViewModel;
pub(self) use widget::Widgets;

use bitcoin::util::bip32::Fingerprint;

use crate::model::{DescriptorClass, HardwareDevice};

#[derive(Msg)]
pub enum Msg {
    New,
    View,
    DevicesList,
    SignerAddDevice(Fingerprint, HardwareDevice),
    SignerSelect,
    ConditionAdd,
    ConditionRemove,
    ConditionSelect,
    ConditionChange,
    ToggleClass(DescriptorClass),
    ExportFormat(bool),
    Update,
    Hide,
}
