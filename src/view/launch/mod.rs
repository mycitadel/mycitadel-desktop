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
mod widget;

pub use component::Component;
pub(self) use widget::Widgets;

use std::path::PathBuf;

use crate::model::PublicNetwork;

pub struct ViewModel {}

#[derive(Msg)]
pub enum Msg {
    Show,
    Close,
    Template(i32),
    Import,
    Wallet,
    Psbt(Option<PublicNetwork>),
    Recent,
    About,
    WalletCreated(PathBuf),
    WalletClosed,
    OpenWallet(PathBuf),
    OpenPsbt(PathBuf),
    PsbtClosed,
}
