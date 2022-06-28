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
use gtk::{Box, Entry, Image, Label, MessageDialog, ResponseType};
use relm::Relm;

use super::Msg;

#[derive(Clone, Gladis)]
pub struct Widgets {
    dialog: MessageDialog,
    xpriv_entry: Entry,
    msg_box: Box,
    msg_lbl: Label,
    msg_img: Image,
}

impl Widgets {
    pub fn to_root(&self) -> MessageDialog { self.dialog.clone() }

    pub fn open(&self) {
        self.xpriv_entry.set_text("");
        self.hide_message();
        self.dialog.show();
        self.dialog.set_response_sensitive(ResponseType::Ok, false);
    }

    pub fn close(&self) {
        self.dialog.hide();
        self.dialog.set_response_sensitive(ResponseType::Ok, false);
        self.xpriv_entry.set_text("");
        self.hide_message();
    }

    pub fn show_notification(&self) { self.msg_box.show_all(); }

    pub fn show_error(&self, msg: &str) {
        self.dialog.set_response_sensitive(ResponseType::Ok, false);
        self.msg_img.set_icon_name(Some("dialog-error-symbolic"));
        self.msg_lbl.set_label(&format!("Error: {}", msg));
        self.msg_box.show_all();
    }

    pub fn show_info(&self, msg: &str) {
        self.dialog.set_response_sensitive(ResponseType::Ok, true);
        self.msg_img
            .set_icon_name(Some("dialog-information-symbolic"));
        self.msg_lbl.set_label(msg);
        self.msg_box.show_all();
    }

    pub fn show_warning(&self, msg: &str) {
        self.dialog.set_response_sensitive(ResponseType::Ok, true);
        self.msg_img.set_icon_name(Some("dialog-warning-symbolic"));
        self.msg_lbl.set_label(&format!(
            "Warning: {}\nYou still may add this key if you understand the risk.",
            msg
        ));
        self.msg_box.show_all();
    }

    pub fn hide_message(&self) {
        self.dialog.set_response_sensitive(ResponseType::Ok, true);
        self.msg_box.hide()
    }

    pub fn xpriv(&self) -> String { self.xpriv_entry.text().to_string() }

    pub(super) fn connect(&self, relm: &Relm<super::Component>) {
        connect!(relm, self.xpriv_entry, connect_changed(_), Msg::Edit);

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
