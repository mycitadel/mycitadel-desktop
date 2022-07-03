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

//! Main library module. For a good starting point see main application
//! component: [`crate::view::launch::Component`].

#![allow(dead_code)]

#[macro_use]
extern crate amplify;
#[macro_use]
extern crate relm;
#[macro_use]
extern crate relm_derive;
#[cfg(feature = "serde")]
extern crate serde_crate as serde;
#[cfg(feature = "serde")]
#[macro_use]
extern crate serde_with;

pub mod model;
#[cfg(feature = "ui")]
pub mod view;
pub mod worker;
