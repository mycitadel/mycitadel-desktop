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
use gtk::{
    Adjustment, Box, Button, Dialog, HeaderBar, Image, Label, ListBox, ListBoxRow, Menu, MenuItem,
    PositionType, ResponseType, Scale, SpinButton, ToolButton,
};
use relm::Relm;

use super::{beneficiary_row, FeeRate, Msg};
use crate::view::{wallet, NotificationBoxExt};

// Create the structure that holds the widgets used in the view.
#[derive(Clone, Gladis)]
pub struct Widgets {
    dialog: Dialog,
    header_bar: HeaderBar,

    msg_box: Box,
    msg_lbl: Label,
    msg_img: Image,

    cancel_btn: Button,
    compose_btn: Button,

    add_btn: ToolButton,
    remove_btn: ToolButton,

    beneficiary_list: ListBox,

    total_lbl: Label,
    weight_lbl: Label,
    fee_adj: Adjustment,
    fee_lbl: Label,
    fee_scale: Scale,
    fee_stp: SpinButton,
    fee_menu: Menu,
    time_lbl: Label,
    block1_mi: MenuItem,
    block2_mi: MenuItem,
    block3_mi: MenuItem,
}

impl Widgets {
    pub fn init_ui(&self, model: &wallet::ViewModel) {
        self.header_bar.set_subtitle(Some(&format!(
            "{:.08} BTC available",
            model.wallet().state().balance as f64 / 100_000_000.0
        )));

        let fees = model.wallet().ephemerals().fees;
        self.fee_adj.set_upper(fees.0 as f64 * 2.0);
        self.fee_adj.set_lower(fees.2 as f64 / 10.0);

        self.update_info(model.fee_rate(), fees, None);
    }

    pub fn show(&self) { self.dialog.show() }
    pub fn hide(&self) { self.dialog.hide() }

    pub fn to_root(&self) -> Dialog { self.dialog.clone() }
    pub fn as_root(&self) -> &Dialog { &self.dialog }

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

        connect!(
            relm,
            self.fee_adj,
            connect_value_changed(_),
            wallet::Msg::Pay(Msg::FeeSet)
        );
        connect!(
            relm,
            self.block1_mi,
            connect_activate(_),
            wallet::Msg::Pay(Msg::FeeSetBlocks(FeeRate::OneBlock))
        );
        connect!(
            relm,
            self.block2_mi,
            connect_activate(_),
            wallet::Msg::Pay(Msg::FeeSetBlocks(FeeRate::TwoBlocks))
        );
        connect!(
            relm,
            self.block3_mi,
            connect_activate(_),
            wallet::Msg::Pay(Msg::FeeSetBlocks(FeeRate::ThreeBlocks))
        );
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

    pub fn update_info(
        &self,
        fee_rate: f32,
        fees: (f32, f32, f32),
        tx_info: Option<(u64, u32, f32)>,
    ) {
        self.compose_btn.set_sensitive(tx_info.is_some());

        self.fee_adj.set_upper(fees.0 as f64 * 5.0);
        self.fee_adj.set_lower(fees.2 as f64 / 10.0);

        if let Some((total, total_fee, vsize)) = tx_info {
            let total_fee = total_fee as f64;
            let total = total as f64 + total_fee;

            self.weight_lbl.set_text(&format!("{:.0} vbytes", vsize));
            self.fee_lbl
                .set_text(&format!("{:.08} BTC", total_fee / 100_000_000.));
            self.total_lbl
                .set_text(&format!("{:.08} BTC", total / 100_000_000.));
        } else {
            self.weight_lbl.set_text("unknown");
            self.fee_lbl.set_text("-");
            self.total_lbl.set_text("unknown");
        }

        if self.fee_adj.value() as f32 != fee_rate {
            self.fee_adj.set_value(fee_rate as f64);
        }

        self.fee_scale.clear_marks();
        self.fee_scale
            .add_mark(fees.0 as f64, PositionType::Bottom, None);
        self.fee_scale
            .add_mark(fees.1 as f64, PositionType::Bottom, None);
        self.fee_scale
            .add_mark(fees.2 as f64, PositionType::Bottom, None);

        let ty = match fee_rate {
            f if f >= fees.0 => FeeRate::OneBlock,
            f if f >= fees.1 => FeeRate::TwoBlocks,
            f if f >= fees.2 => FeeRate::ThreeBlocks,
            _ => FeeRate::Unknown,
        };

        self.time_lbl.set_text(&ty.to_string());
    }

    pub fn fee_rate(&self) -> f64 { self.fee_adj.value() }

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
    fn notification_box(&self) -> &Box { &self.msg_box }
    fn main_dialog(&self) -> &Dialog { &self.dialog }
    fn main_action_button(&self) -> &Button { &self.compose_btn }
    fn notification_image(&self) -> &Image { &self.msg_img }
    fn notification_label(&self) -> &Label { &self.msg_lbl }
}
