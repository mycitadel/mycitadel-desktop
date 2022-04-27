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

use crate::view::psbt::sign_row;
use crate::view::psbt::sign_row::SigningModel;
use gladis::Gladis;
use gtk::prelude::*;
use gtk::{
    ApplicationWindow, Button, Entry, Expander, HeaderBar, Label, LevelBar, ListBox, ListStore,
    MenuItem, TextView, TreeView,
};
use relm::Relm;

use super::{Msg, ViewModel};

// Create the structure that holds the widgets used in the view.
#[derive(Clone, Gladis)]
pub struct Widgets {
    window: ApplicationWindow,
    header_bar: HeaderBar,
    save_btn: Button,
    publish_btn: Button,

    new_wallet_mi: MenuItem,
    new_template_mi: MenuItem,
    open_wallet_mi: MenuItem,
    open_psbt_mi: MenuItem,
    import_mi: MenuItem,
    launcher_mi: MenuItem,
    about_mi: MenuItem,

    txid_fld: Entry,
    description_fld: Entry,
    notes_exp: Expander,
    notes_text: TextView,

    amount_lbl: Label,
    address_store: ListStore,
    address_list: TreeView,
    signatures_list: ListBox,
    sigs_lbl: Label,
    progress_bar: LevelBar,

    volume_lbl: Label,
    fee_lbl: Label,
    inputs_lbl: Label,
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

    pub(super) fn bind_signing_model(&self, relm: &Relm<super::Component>, model: &SigningModel) {
        let relm = relm.clone();
        self.signatures_list.bind_model(Some(model), move |item| {
            sign_row::RowWidgets::init(relm.clone(), item)
        });
    }
}
