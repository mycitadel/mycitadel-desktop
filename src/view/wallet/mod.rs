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
mod view_model;
mod widget;

pub use self::component::Component;
pub(self) use view_model::ViewModel;
pub(self) use widget::Widgets;

use std::collections::BTreeSet;
use std::path::PathBuf;

use relm::StreamHandle;

use crate::model::{DescriptorClass, Signer};
use crate::view::launch;
use crate::worker::WatchMsg;

#[derive(Msg)]
pub enum Msg {
    New,
    Open,
    Close,
    FileError(PathBuf, String),
    Save,
    Settings,
    Update(Vec<Signer>, BTreeSet<DescriptorClass>),
    Pay,
    Receive,
    Refresh,
    Select(usize),
    Create,
    ElectrumWatch(WatchMsg),
    RegisterLauncher(StreamHandle<launch::Msg>),
}
