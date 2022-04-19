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
use gtk::{Box, Entry, Image, Label, MessageDialog};
use relm::Relm;

use super::Msg;

#[derive(Clone, Gladis)]
pub struct Widgets {
    dialog: MessageDialog,
    xpub_entry: Entry,
    msg_box: Box,
    msg_lbl: Label,
    msg_img: Image,
}

impl Widgets {
    pub fn to_root(&self) -> MessageDialog {
        self.dialog.clone()
    }

    pub fn open(&self) {
        self.xpub_entry.set_text("");
        self.hide_message();
        self.dialog.show();
    }

    pub fn show_error(&self, msg: &str) {
        self.msg_img.set_icon_name(Some("dialog-error-symbolic"));
        self.msg_lbl.set_label(msg);
        self.msg_box.show_all();
    }

    pub fn show_info(&self, msg: &str) {
        self.msg_img
            .set_icon_name(Some("dialog-information-symbolic"));
        self.msg_lbl.set_label(msg);
        self.msg_box.show_all();
    }

    pub fn show_warning(&self, msg: &str) {
        self.msg_img.set_icon_name(Some("dialog-warning-symbolic"));
        self.msg_lbl.set_label(msg);
        self.msg_box.show_all();
    }

    pub fn hide_message(&self) {
        self.msg_box.hide()
    }

    pub(super) fn connect(&self, relm: &Relm<super::Component>) {
        connect!(relm, self.xpub_entry, connect_changed(_), Msg::Edit);
    }
}
