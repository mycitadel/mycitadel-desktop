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

use gladis::Gladis;
use gtk::prelude::*;
use gtk::{gdk, Dialog, Entry, Image};
use relm::Relm;

use super::{Msg, ViewModel};

// Create the structure that holds the widgets used in the view.
#[derive(Clone, Gladis)]
pub struct Widgets {
    dialog: Dialog,
    logo_img: Image,
    pgp_fld: Entry,
}

impl Widgets {
    pub fn init_ui(&self, _model: &ViewModel) {}

    pub fn show(&self) { self.dialog.show() }
    pub fn hide(&self) { self.dialog.hide() }

    pub fn to_root(&self) -> Dialog { self.dialog.clone() }
    pub fn as_root(&self) -> &Dialog { &self.dialog }

    pub(super) fn connect(&self, relm: &Relm<super::Component>) {
        self.pgp_fld.connect_icon_press(|entry, _, _| {
            let val = entry.text();
            gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD).set_text(&val);
        });

        connect!(
            relm,
            self.dialog,
            connect_response(_, resp),
            Msg::Response(resp)
        );

        connect!(
            relm,
            self.dialog,
            connect_delete_event(_, _),
            return (None, Inhibit(true))
        );
    }
}
