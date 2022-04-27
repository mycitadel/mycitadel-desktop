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
use bitcoin::blockdata::constants::WITNESS_SCALE_FACTOR;
use bitcoin::Address;
use gladis::Gladis;
use gtk::prelude::*;
use gtk::{
    ApplicationWindow, Button, Entry, Expander, HeaderBar, Label, LevelBar, ListBox, ListStore,
    MenuItem, TextView, TreeView,
};
use miniscript::{Legacy, Miniscript, Segwitv0};
use relm::Relm;
use std::ffi::OsStr;
use wallet::address::address::AddressFormat;
use wallet::psbt::Psbt;

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
    pub fn update_ui(&self, model: &ViewModel) {
        let psbt: &Psbt = model.psbt();
        let tx = psbt.clone().into_transaction();

        // TODO: Move PSBT-related code to descriptor-wallet
        let mut vsize = tx.vsize() as f32;

        let mut volume = 0u64;
        for input in &psbt.inputs {
            if let Some(txout) = &input.witness_utxo {
                volume += txout.value;
            } else if let Some(tx) = &input.non_witness_utxo {
                volume += tx.output[input.previous_outpoint.vout as usize].value;
            }

            if let Some(script) = &input.witness_script {
                Miniscript::<bitcoin::PublicKey, Segwitv0>::parse_insane(script)
                    .and_then(|ms| ms.max_satisfaction_size())
                    .ok()
            } else if let Some(script) = &input.redeem_script {
                Miniscript::<bitcoin::PublicKey, Legacy>::parse_insane(script)
                    .and_then(|ms| ms.max_satisfaction_size())
                    .ok()
            } else if let Some(_pk) = &input.tap_internal_key {
                // TODO: Support script analysis
                Some(66)
            } else {
                None
            }
            .map(|witness_size| vsize += witness_size as f32 / WITNESS_SCALE_FACTOR as f32);
        }

        let sigs_required = psbt.inputs.len() as u32;
        let mut sigs_present = 0u32;
        let signing_model: &SigningModel = model.signing();
        for no in 0..signing_model.n_items() {
            if let Some(signing) = signing_model.item(no) {
                let present: u32 = signing.property("sigs-present");
                let required: u32 = signing.property("sigs-required");
                if present >= required {
                    sigs_present += 1;
                }
            }
        }

        let mut sent = 0u64;
        let mut change = 0u64;
        for output in &psbt.outputs {
            sent += output.amount;
            if output.bip32_derivation.is_empty() && output.tap_key_origins.is_empty() {
                continue;
            }
            change += output.amount;
        }
        let fee = volume - sent;
        sent += fee;
        let spent = sent - change;

        self.header_bar
            .set_subtitle(model.path().file_name().and_then(OsStr::to_str));

        self.txid_fld.set_text(&tx.txid().to_string());
        // TODO: Extract notes and description from proprietary keys

        self.progress_bar.set_value(sigs_present as f64);
        self.progress_bar.set_max_value(sigs_required as f64);
        self.sigs_lbl
            .set_label(&format!("{} of {} required", sigs_present, sigs_required));

        self.amount_lbl
            .set_label(&format!("{:.8} BTC", spent as f64 / 100_000_000.0));
        self.volume_lbl
            .set_label(&format!("{:.8} BTC", volume as f64 / 100_000_000.0));
        self.fee_lbl.set_label(&format!(
            "{} sats, {:.1} sat/vbyte",
            fee,
            fee as f32 / vsize.ceil()
        ));
        self.inputs_lbl.set_label(&format!("{}", psbt.inputs.len()));

        self.address_store.clear();
        for output in &psbt.outputs {
            let address = Address::from_script(&output.script, model.network().into());
            let address_str = address
                .as_ref()
                .map(Address::to_string)
                .unwrap_or_else(|| output.script.to_string());
            let address_type = address
                .map(AddressFormat::from)
                .as_ref()
                .map(AddressFormat::to_string)
                .unwrap_or(s!("custom"));
            self.address_store.insert_with_values(
                None,
                &[
                    (0, &address_str),
                    (1, &format!("{:.08}", output.amount as f64 / 100_000_000.0)),
                    (
                        2,
                        &!(output.bip32_derivation.is_empty() && output.tap_key_origins.is_empty()),
                    ),
                    (3, &address_type),
                ],
            );
        }
    }

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
