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

pub use self::component::Component;
use crate::model::WalletDescriptor;
pub use view_model::ModelParam;
pub(self) use view_model::ViewModel;

#[derive(Msg)]
pub enum Msg {
    New,
    Open,
    Settings,
    Update(WalletDescriptor),
    Send,
    Receive,
    Refresh,
    Select(usize),
    Create,
    Quit,
}