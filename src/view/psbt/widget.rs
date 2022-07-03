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

use std::ffi::OsStr;
use std::path::{self, Path};

use ::wallet::address::AddressFormat;
use ::wallet::psbt::Psbt;
use bitcoin::blockdata::constants::WITNESS_SCALE_FACTOR;
use bitcoin::Address;
use gladis::Gladis;
use gtk::gdk_pixbuf::Pixbuf;
use gtk::prelude::*;
use gtk::{
    gdk, ApplicationWindow, Button, Dialog, Entry, Expander, HeaderBar, Image, Label, LevelBar,
    ListBox, ListStore, MenuItem, RadioMenuItem, TextView, TreeView,
};
use miniscript::{Legacy, Miniscript, Segwitv0};
use relm::Relm;
use wallet::onchain::PublicNetwork;

use super::{Msg, ViewModel};
use crate::view::launch::Page;
use crate::view::psbt::sign_row;
use crate::view::psbt::sign_row::SigningModel;
use crate::view::{launch, APP_ICON, APP_ICON_TOOL};

// Create the structure that holds the widgets used in the view.
#[derive(Clone, Gladis)]
pub struct Widgets {
    window: ApplicationWindow,

    header_bar: HeaderBar,
    logo_img: Image,
    save_btn: Button,
    publish_btn: Button,

    network_lbl: Label,
    mainnet_mi: RadioMenuItem,
    testnet_mi: RadioMenuItem,
    signet_mi: RadioMenuItem,

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

    sign_dlg: Dialog,
    sign_msg_lbl: Label,
}

impl Widgets {
    pub fn init_ui(&self) {
        let icon = Pixbuf::from_read(APP_ICON).expect("app icon is missed");
        self.window.set_icon(Some(&icon));

        let img = Pixbuf::from_read(APP_ICON_TOOL).expect("small app icon is missed");
        self.logo_img.set_pixbuf(Some(&img));
    }

    pub fn update_ui(&self, model: &ViewModel) {
        let psbt: &Psbt = model.psbt();
        let tx = psbt.to_unsigned_tx();

        self.update_path(model.path().as_deref());

        self.publish_btn.set_visible(model.finalized_tx().is_some());

        self.update_network(model.network());

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

        let mut sigs_present = 0u32;
        let signing_model: &SigningModel = model.signing();
        let sigs_possible = signing_model.n_items() as u32;
        for no in 0..sigs_possible {
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

        self.header_bar.set_subtitle(
            model
                .path()
                .as_ref()
                .and_then(|p| p.file_name())
                .and_then(OsStr::to_str),
        );

        self.txid_fld.set_text(&tx.txid().to_string());
        // TODO: Extract notes and description from proprietary keys

        self.progress_bar.set_value(sigs_present as f64);
        self.progress_bar.set_max_value(sigs_possible as f64);
        self.sigs_lbl
            .set_label(&format!("{} of possible {}", sigs_present, sigs_possible));

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

        self.update_addresses(psbt, model.network());
    }

    pub fn show(&self) { self.window.show() }
    pub fn hide(&self) { self.window.hide() }
    pub fn close(&self) { self.window.close() }

    pub fn show_sign(&self, msg: &str) {
        self.sign_msg_lbl.set_text(msg);
        self.sign_dlg.show();
    }

    pub fn hide_sign(&self) { self.sign_dlg.hide(); }

    pub fn to_root(&self) -> ApplicationWindow { self.window.clone() }
    pub fn as_root(&self) -> &ApplicationWindow { &self.window }

    pub(super) fn connect(&self, relm: &Relm<super::Component>) {
        connect!(relm, self.save_btn, connect_clicked(_), Msg::Save);
        connect!(relm, self.publish_btn, connect_clicked(_), Msg::Publish);

        connect!(
            relm,
            self.new_wallet_mi,
            connect_activate(_),
            Msg::Launch(launch::Msg::Template(5))
        );
        connect!(
            relm,
            self.new_template_mi,
            connect_activate(_),
            Msg::Launch(launch::Msg::ShowPage(Page::Template))
        );
        connect!(
            relm,
            self.open_wallet_mi,
            connect_activate(_),
            Msg::Launch(launch::Msg::Wallet)
        );
        connect!(
            relm,
            self.open_psbt_mi,
            connect_activate(_),
            Msg::Launch(launch::Msg::Psbt(None))
        );
        connect!(
            relm,
            self.import_mi,
            connect_activate(_),
            Msg::Launch(launch::Msg::ShowPage(Page::Import))
        );
        connect!(
            relm,
            self.launcher_mi,
            connect_activate(_),
            Msg::Launch(launch::Msg::Show)
        );
        connect!(
            relm,
            self.about_mi,
            connect_activate(_),
            Msg::Launch(launch::Msg::About)
        );

        connect!(
            relm,
            self.mainnet_mi,
            connect_toggled(mi),
            if mi.is_active() {
                Msg::Network(PublicNetwork::Mainnet)
            } else {
                Msg::NoOp
            }
        );
        connect!(
            relm,
            self.testnet_mi,
            connect_toggled(mi),
            if mi.is_active() {
                Msg::Network(PublicNetwork::Testnet)
            } else {
                Msg::NoOp
            }
        );
        connect!(
            relm,
            self.signet_mi,
            connect_toggled(mi),
            if mi.is_active() {
                Msg::Network(PublicNetwork::Signet)
            } else {
                Msg::NoOp
            }
        );

        self.txid_fld.connect_icon_press(|entry, _, _| {
            let val = entry.text();
            gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD).set_text(&val);
        });

        // We prohibit manual closing of the signing dialog
        self.sign_dlg.connect_response(|_, _| {});
        self.sign_dlg.connect_delete_event(|_, _| Inhibit(true));

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

    pub fn update_path(&self, path: Option<&Path>) {
        self.header_bar.set_subtitle(
            path.map(Path::display)
                .as_ref()
                .map(path::Display::to_string)
                .as_deref(),
        );
        self.save_btn.set_sensitive(path.is_none());
    }

    pub fn set_unsaved(&self) { self.save_btn.set_sensitive(true); }

    pub fn update_network(&self, network: PublicNetwork) {
        let network_name = network.to_string();
        let network_name = network_name[0..1].to_uppercase() + &network_name[1..];
        self.network_lbl.set_text(&network_name);

        self.mainnet_mi
            .set_active(network == PublicNetwork::Mainnet);
        self.testnet_mi
            .set_active(network == PublicNetwork::Testnet);
        self.signet_mi.set_active(network == PublicNetwork::Signet);
    }

    pub fn update_addresses(&self, psbt: &Psbt, network: PublicNetwork) {
        self.address_store.clear();
        for output in &psbt.outputs {
            let address = Address::from_script(&output.script, network.into());
            let address_str = address
                .as_ref()
                .map(Address::to_string)
                .unwrap_or_else(|| output.script.to_string());
            let address_type = address
                .map(AddressFormat::from)
                .as_ref()
                .map(AddressFormat::to_string)
                .unwrap_or(s!("custom"));
            self.address_store.insert_with_values(None, &[
                (0, &address_str),
                (1, &format!("{:.08}", output.amount as f64 / 100_000_000.0)),
                (
                    2,
                    &!(output.bip32_derivation.is_empty() && output.tap_key_origins.is_empty()),
                ),
                (3, &address_type),
            ]);
        }
    }

    pub fn publish_pending(&self) {
        self.publish_btn.set_always_show_image(false);
        self.publish_btn.set_label("Sending...");
        self.publish_btn.set_sensitive(false);
    }

    pub fn publish_restore(&self, success: bool) {
        if success {
            self.publish_btn.set_label("Published");
            self.publish_btn.set_sensitive(false);
        } else {
            self.publish_btn.set_always_show_image(true);
            self.publish_btn.set_label("Broadcast");
            self.publish_btn.set_sensitive(true);
        }
    }
}
