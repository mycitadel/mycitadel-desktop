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

use std::path::PathBuf;

pub use component::Component;
use wallet::psbt::Psbt;
pub(self) use widget::Widgets;

use crate::model::{PublicNetwork, WalletSettings};

pub struct ViewModel {}

#[derive(Msg)]
pub enum Msg {
    Show(Page),
    Close,
    Template(i32),
    Duplicate(WalletSettings, String),
    Import,
    Wallet,
    Psbt(Option<PublicNetwork>),
    Recent,
    About,
    WalletCreated(PathBuf),
    WalletClosed,
    CreatePsbt(Psbt, PublicNetwork),
    PsbtClosed,
}

#[derive(Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub enum Page {
    Template = 0,
    Open = 1,
    Import = 2,
}
