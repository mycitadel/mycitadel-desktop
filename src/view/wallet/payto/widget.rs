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
use rgbstd::interface::TypedState;
use rgbwallet::{Beneficiary, RgbInvoice};

use super::Msg;
use crate::view::wallet;
use crate::view::wallet::asset_row::AssetInfo;

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

    ticker_lbl: Label,
    name_lbl: Label,
    contract_lbl: Label,

    beneficiary_fld: Entry,
    amount_fld: Entry,
    max_btn: ToggleButton,
}

impl Widgets {
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

    pub fn init_ui(&self, _model: &wallet::ViewModel) {}

    pub fn update_ui(&self, asset: AssetInfo, invoice: Option<RgbInvoice>) {
        let is_asset = invoice.is_some();

        self.batch_btn.set_visible(!is_asset);
        self.beneficiary_fld.set_text("");
        self.amount_fld.set_text("");

        self.info_bar.set_visible(false);

        if let Some(invoice) = invoice {
            self.beneficiary_fld
                .set_primary_icon_name(Some("dialog-information-symbolic"));
            match invoice.beneficiary {
                Beneficiary::BlindedSeal(seal) => {
                    self.beneficiary_fld.set_primary_icon_tooltip_text(Some(
                        "You are paying to an existing blinded unspent transaction output (UTXOb)",
                    ));
                    self.beneficiary_fld.set_text(&seal.to_string());
                }
                Beneficiary::WitnessUtxo(address) => {
                    self.beneficiary_fld.set_primary_icon_tooltip_text(Some(
                        "You will create a new output for the beneficiary and allocate there a \
                         small amount of bitcoins",
                    ));
                    self.beneficiary_fld.set_text(&address.to_string());
                }
            }
            if let TypedState::Amount(amount) = invoice.owned_state {
                self.amount_fld.set_text(&asset.amount_fmt(amount));
            }
        } else {
            self.beneficiary_fld.set_primary_icon_tooltip_text(None);
            self.beneficiary_fld.set_primary_icon_name(None);
        }

        self.header_bar.set_subtitle(Some(&format!(
            "{} {} available",
            asset.amount_display(),
            asset.ticker(),
        )));

        self.contract_lbl.set_visible(is_asset);
        self.contract_lbl.set_text(&asset.contract_name());
        self.ticker_lbl.set_text(&asset.ticker());
        self.name_lbl.set_text(&asset.name());
    }
}
