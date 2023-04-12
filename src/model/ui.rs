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

use bpro::HistoryEntry;
use gtk::{gdk, MessageType};

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct Notification {
    pub msg: String,
    pub ty: MessageType,
}

pub trait UI {
    fn color(&self) -> gdk::RGBA;
}

impl UI for HistoryEntry {
    fn color(&self) -> gdk::RGBA {
        match self.balance() {
            x if x > 0 => gdk::RGBA::new(38.0 / 256.0, 162.0 / 256.0, 105.0 / 256.0, 1.0),
            x if x < 0 => gdk::RGBA::new(165.0 / 256.0, 29.0 / 256.0, 45.0 / 256.0, 1.0),
            0 => gdk::RGBA::new(119.0 / 256.0, 118.0 / 256.0, 123.0 / 256.0, 1.0),
            _ => unreachable!(),
        }
    }
}
