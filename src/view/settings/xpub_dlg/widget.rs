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

use crate::view::settings::xpub_dlg::view_model::XpubModel;
use gladis::Gladis;
use gtk::prelude::*;
use gtk::{glib, Box, Entry, Image, Label, MessageDialog, MessageType, TextBuffer};

#[derive(Clone, Gladis)]
pub struct Widgets {
    dialog: MessageDialog,
    xpub_entry: Entry,
    xpub_buf: TextBuffer,
    msg_box: Box,
    msg_lbl: Label,
    msg_img: Image,
}

impl Widgets {
    pub fn to_root(&self) -> MessageDialog {
        self.dialog.clone()
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

    pub fn bind_model(&self, model: &XpubModel) {
        let flags = glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::DEFAULT;

        self.xpub_buf
            .bind_property("text", model, "xpub")
            .flags(flags)
            .build();
        self.msg_box
            .bind_property("visible", model, "message")
            .flags(flags)
            .transform_to(|_, v| Some(v.get::<Option<String>>().unwrap().is_some().to_value()))
            .build();
        self.msg_lbl
            .bind_property("label", model, "message")
            .flags(flags)
            .build();
        self.msg_img
            .bind_property("icon-name", model, "message-type")
            .flags(flags)
            .transform_to(|_, v| {
                Some(
                    match v.get::<MessageType>().unwrap() {
                        MessageType::Info => "dialog-information-symbolic",
                        MessageType::Warning => "dialog-warning-symbolic",
                        MessageType::Question => "dialog-question-symbolic",
                        MessageType::Error => "dialog-error-symbolic",
                        _ => "",
                    }
                    .to_value(),
                )
            })
            .build();
    }
}
