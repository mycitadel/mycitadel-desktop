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
use gtk::{Button, Dialog, Entry, HeaderBar, InfoBar, Label, ResponseType, ToggleButton};
use relm::Relm;

use super::Msg;
use crate::view::wallet;

// Create the structure that holds the widgets used in the view.
#[derive(Clone, Gladis)]
pub struct Widgets {
    dialog: Dialog,
    header_bar: HeaderBar,

    info_bar: InfoBar,
    info_lbl: Label,

    cancel_btn: Button,
    compose_btn: Button,
    batch_btn: Button,

    invoice_fld: Entry,
    amount_fld: Entry,
    max_btn: ToggleButton,
    asset_lbl: Label,
}

impl Widgets {
    pub fn init_ui(&self, model: &wallet::ViewModel) {
        self.header_bar.set_subtitle(Some(&format!(
            "{:.08} BTC available",
            model.wallet().state().balance as f64 / 100_000_000.0
        )));
    }

    pub fn show(&self) { self.dialog.show() }
    pub fn hide(&self) { self.dialog.hide() }

    pub fn to_root(&self) -> Dialog { self.dialog.clone() }
    pub fn as_root(&self) -> &Dialog { &self.dialog }

    pub fn connect(&self, relm: &Relm<wallet::Component>) {
        connect!(
            relm,
            self.dialog,
            connect_response(_, resp),
            wallet::Msg::PayTo(Msg::Response(resp))
        );
        self.dialog.set_response_sensitive(ResponseType::Ok, true);

        connect!(
            relm,
            self.dialog,
            connect_delete_event(_, _),
            return (None, Inhibit(true))
        );
    }
}
