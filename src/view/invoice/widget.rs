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
use gtk::{Dialog, ResponseType};
use relm::Relm;

use super::{Msg, ViewModel};

// Create the structure that holds the widgets used in the view.
#[derive(Clone, Gladis)]
pub struct Widgets {
    dialog: Dialog,
}
impl Widgets {
    pub fn update_ui(&self, _model: &ViewModel) {}

    pub fn show(&self) {
        self.dialog.show()
    }
    pub fn hide(&self) {
        self.dialog.hide()
    }
    pub fn close(&self) {
        self.dialog.close()
    }

    pub fn to_root(&self) -> Dialog {
        self.dialog.clone()
    }
    pub fn as_root(&self) -> &Dialog {
        &self.dialog
    }

    pub(super) fn connect(&self, relm: &Relm<super::Component>) {
        connect!(
            relm,
            self.dialog,
            connect_response(_, resp),
            Msg::Response(resp)
        );
        self.dialog.set_response_sensitive(ResponseType::Ok, false);

        connect!(
            relm,
            self.dialog,
            connect_delete_event(_, _),
            return (None, Inhibit(true))
        );
    }
}
