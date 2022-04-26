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
    Box, Button, Dialog, HeaderBar, Image, Label, ListBox, Menu, RadioMenuItem, ResponseType,
    Scale, SpinButton, ToolButton,
};
use relm::Relm;

use super::{beneficiary_row, beneficiary_row::BeneficiaryModel, Msg, ViewModel};
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
            self.cancel_btn,
            connect_clicked(_),
            Msg::Response(ResponseType::Cancel)
        );

        connect!(
            relm,
            self.dialog,
            connect_delete_event(_, _),
            return (None, Inhibit(true))
        );
    }

    pub(super) fn bind_spending_model(
        &self,
        relm: &Relm<super::Component>,
        model: &BeneficiaryModel,
    ) {
        let relm = relm.clone();
        self.beneficiary_list.bind_model(Some(model), move |item| {
            beneficiary_row::RowWidgets::init(relm.clone(), item)
        });
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
