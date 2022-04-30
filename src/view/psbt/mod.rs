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
pub(self) mod sign_row;
mod view_model;
mod widget;

use relm::StreamHandle;
pub(super) use view_model::ModelParam;
pub(self) use view_model::ViewModel;
pub(self) use widget::Widgets;

pub use self::component::Component;
use crate::view::launch;

#[derive(Msg)]
pub enum Msg {
    Close,
    Save,
    Publish,
    Launcher(launch::Msg),
    Sign(u32),
    RegisterLauncher(StreamHandle<launch::Msg>),
}
