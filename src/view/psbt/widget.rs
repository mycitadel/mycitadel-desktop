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

use gladis::Gladis;
use gtk::prelude::*;
use gtk::{ApplicationWindow, MenuItem};
use relm::Relm;

use super::{Msg, ViewModel};

// Create the structure that holds the widgets used in the view.
#[derive(Clone, Gladis)]
pub struct Widgets {
    window: ApplicationWindow,
    about_mi: MenuItem,
}

impl Widgets {
    pub fn update_ui(&self, _model: &ViewModel) {}

    pub fn show(&self) {
        self.window.show()
    }
    pub fn hide(&self) {
        self.window.hide()
    }
    pub fn close(&self) {
        self.window.close()
    }

    pub fn to_root(&self) -> ApplicationWindow {
        self.window.clone()
    }
    pub fn as_root(&self) -> &ApplicationWindow {
        &self.window
    }

    pub(super) fn connect(&self, relm: &Relm<super::Component>) {
        connect!(relm, self.about_mi, connect_activate(_), Msg::About);
        connect!(
            relm,
            self.window,
            connect_delete_event(_, _),
            return (Msg::Close, Inhibit(false))
        );
    }
}
