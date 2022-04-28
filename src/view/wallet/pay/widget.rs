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
use gtk::{
    Box, Button, Dialog, HeaderBar, Image, Label, ListBox, ListBoxRow, Menu, RadioMenuItem,
    ResponseType, Scale, SpinButton, ToolButton,
};
use relm::Relm;

use super::{beneficiary_row, Msg};
use crate::view::wallet;
use crate::view::NotificationBoxExt;

// Create the structure that holds the widgets used in the view.
#[derive(Clone, Gladis)]
pub struct Widgets {
    dialog: Dialog,
    header_bar: HeaderBar,

    msg_box: Box,
    msg_lbl: Label,
    msg_img: Image,

    cancel_btn: Button,
    prepare_btn: Button,

    add_btn: ToolButton,
    remove_btn: ToolButton,

    beneficiary_list: ListBox,

    total_lbl: Label,
    weight_lbl: Label,
    fee_lbl: Label,
    fee_scale: Scale,
    fee_stp: SpinButton,
    fee_menu: Menu,
    block1_mi: RadioMenuItem,
    block2_mi: RadioMenuItem,
    block3_mi: RadioMenuItem,
    unknown_mi: RadioMenuItem,
}

impl Widgets {
    pub fn init_ui(&self, _model: &wallet::ViewModel) {}

    pub fn show(&self) {
        self.dialog.show()
    }
    pub fn hide(&self) {
        self.dialog.hide()
    }

    pub fn to_root(&self) -> Dialog {
        self.dialog.clone()
    }
    pub fn as_root(&self) -> &Dialog {
        &self.dialog
    }

    pub fn connect(&self, relm: &Relm<wallet::Component>) {
        connect!(
            relm,
            self.add_btn,
            connect_clicked(_),
            wallet::Msg::Pay(Msg::BeneficiaryAdd)
        );
        connect!(
            relm,
            self.remove_btn,
            connect_clicked(_),
            wallet::Msg::Pay(Msg::BeneficiaryRemove)
        );

        self.beneficiary_list.connect_row_activated(|list, row| {
            list.select_row(Some(row));
        });

        self.remove_btn.set_sensitive(false);
        let remove_btn = self.remove_btn.clone();
        self.beneficiary_list
            .connect_selected_rows_changed(move |list| {
                remove_btn.set_sensitive(list.selected_row().is_some())
            });

        connect!(
            relm,
            self.dialog,
            connect_response(_, resp),
            wallet::Msg::Pay(Msg::Response(resp))
        );
        self.dialog.set_response_sensitive(ResponseType::Ok, false);

        connect!(
            relm,
            self.dialog,
            connect_delete_event(_, _),
            return (None, Inhibit(true))
        );

        // TODO: Connect fee editing
    }

    pub fn bind_beneficiary_model(
        &self,
        relm: &Relm<wallet::Component>,
        model: &wallet::ViewModel,
    ) {
        let relm = relm.clone();
        let network = model.as_settings().network();
        self.beneficiary_list
            .bind_model(Some(model.beneficiaries()), move |item| {
                beneficiary_row::RowWidgets::init(relm.clone(), item, network)
            });
    }

    pub fn select_beneficiary(&self, index: u32) {
        self.beneficiary_list
            .select_row(self.beneficiary_list.row_at_index(index as i32).as_ref())
    }

    pub fn selected_beneficiary_index(&self) -> Option<u32> {
        self.beneficiary_list
            .selected_row()
            .as_ref()
            .map(ListBoxRow::index)
            .map(|i| i as u32)
    }
}

impl NotificationBoxExt for Widgets {
    fn notification_box(&self) -> &Box {
        &self.msg_box
    }
    fn main_dialog(&self) -> &Dialog {
        &self.dialog
    }
    fn main_action_button(&self) -> &Button {
        &self.prepare_btn
    }
    fn notification_image(&self) -> &Image {
        &self.msg_img
    }
    fn notification_label(&self) -> &Label {
        &self.msg_lbl
    }
}
