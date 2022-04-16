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

use std::path::PathBuf;

use crate::model::{Wallet, WalletDescriptor};

pub enum ModelParam {
    Open(PathBuf),
    New(WalletDescriptor),
}

pub struct ViewModel {}

impl From<Wallet> for ViewModel {
    fn from(_wallet: Wallet) -> Self {
        ViewModel {}
    }
}
