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
use gtk::{ApplicationWindow, Button, Entry, HeaderBar, ListStore, Popover, Statusbar, TreeView};
use relm::Relm;
use std::ffi::OsStr;

use super::{Msg, ViewModel};

// Create the structure that holds the widgets used in the view.
#[derive(Clone, Gladis)]
pub struct Widgets {
    window: ApplicationWindow,

    header_bar: HeaderBar,
    new_btn: Button,
    open_btn: Button,
    settings_btn: Button,
    pay_btn: Button,

    history_store: ListStore,
    utxo_store: ListStore,
    address_store: ListStore,

    address_list: TreeView,
    utxo_list: TreeView,
    history_list: TreeView,

    status_bar: Statusbar,

    invoice_popover: Popover,
    address_fld: Entry,
}

impl Widgets {
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

    pub fn update_ui(&self, model: &ViewModel) {
        let address = model.as_wallet().next_address();
        self.address_fld.set_text(&address.to_string());
        self.header_bar
            .set_subtitle(model.path().file_name().and_then(OsStr::to_str));
        // TODO: Display change addresses
        for row in model.generate_addresses(true, 20) {
            row.insert_item(&self.address_store);
        }
    }

    pub(super) fn connect(&self, relm: &Relm<super::Component>) {
        connect!(relm, self.new_btn, connect_clicked(_), Msg::New);
        connect!(relm, self.open_btn, connect_clicked(_), Msg::Open);
        connect!(relm, self.settings_btn, connect_clicked(_), Msg::Settings);
        connect!(relm, self.pay_btn, connect_clicked(_), Msg::Pay);
        connect!(
            relm,
            self.window,
            connect_delete_event(_, _),
            return (Msg::Close, Inhibit(false))
        );
    }
}
