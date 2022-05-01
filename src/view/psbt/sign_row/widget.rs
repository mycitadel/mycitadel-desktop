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
use gtk::{glib, Button, Label, ListBoxRow};
use relm::Relm;

use super::Signing;
use crate::view::psbt;

#[derive(Clone, Gladis)]
pub struct RowWidgets {
    signing_row: ListBoxRow,
    name_lbl: Label,
    status_lbl: Label,
    fingerprint_lbl: Label,
    sign_btn: Button,
}

impl RowWidgets {
    pub fn init(relm: Relm<psbt::Component>, item: &glib::Object) -> gtk::Widget {
        let glade_src = include_str!("sign_row.glade");
        let row_widgets = RowWidgets::from_string(glade_src).expect("glade file broken");

        let signing = item
            .downcast_ref::<Signing>()
            .expect("Row data is of wrong type");
        row_widgets.bind_model(signing);

        let row = row_widgets.signing_row.clone();
        connect!(
            relm,
            row_widgets.sign_btn,
            connect_clicked(_),
            psbt::Msg::Sign(row.index() as u32)
        );

        row_widgets.signing_row.upcast::<gtk::Widget>()
    }

    fn bind_model(&self, signing: &Signing) {
        let flags_ro = glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE;

        signing
            .bind_property("name", &self.name_lbl, "label")
            .flags(flags_ro)
            .build();
        signing
            .bind_property("master-fp", &self.fingerprint_lbl, "label")
            .transform_to(|_, value| {
                let s: String = value.clone().get().expect("non-string fingerprint");
                Some(format!("[{}]", s).to_value())
            })
            .flags(flags_ro)
            .build();
        signing
            .bind_property("signable", &self.sign_btn, "visible")
            .flags(flags_ro)
            .build();
        signing
            .bind_property("status", &self.status_lbl, "label")
            .flags(flags_ro)
            .build();
    }
}
