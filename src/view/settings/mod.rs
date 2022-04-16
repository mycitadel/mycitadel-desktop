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

pub mod spending_row;
mod view;
mod view_model;

pub use view::Win;
pub use view_model::ViewModel;

use std::sync::{Arc, Mutex};

use bitcoin::util::bip32::Fingerprint;

use crate::model::{DescriptorClass, HardwareDevice};

#[derive(Msg)]
pub enum Msg {
    Show,
    Init(Arc<Mutex<ViewModel>>),
    Devices,
    AddDevice(Fingerprint, HardwareDevice),
    SignerSelect,
    ToggleDescr(DescriptorClass),
    ExportFormat(bool),
    Save,
    Cancel,
}
