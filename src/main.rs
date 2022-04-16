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

#![allow(dead_code)]

#[macro_use]
extern crate amplify;
#[macro_use]
extern crate relm;
#[macro_use]
extern crate relm_derive;

use relm::Widget;

use crate::model::WalletDescriptor;
use crate::view::wallet::ModelParam;

mod model;
mod view;

fn main() {
    view::wallet::Component::run(ModelParam::New(WalletDescriptor::default()))
        .expect("wallet::Win::run failed");
}
