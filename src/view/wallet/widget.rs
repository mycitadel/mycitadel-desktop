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

use std::collections::BTreeMap;
use std::ffi::OsStr;

use bitcoin::{Transaction, Txid};
use chrono::{DateTime, NaiveDateTime, Utc};
use electrum_client::HeaderNotification;
use gladis::Gladis;
use gtk::prelude::*;
use gtk::{
    ApplicationWindow, Button, Entry, HeaderBar, IconSize, Image, Label, ListStore, MenuItem,
    Popover, Spinner, Statusbar, TreeView,
};
use relm::Relm;

use super::{ElectrumState, Msg, ViewModel};
use crate::model::{AddressInfo, ElectrumSec, ElectrumServer, WalletState};
use crate::view::pay;
use crate::worker::{HistoryTxid, UtxoTxid};

impl ElectrumSec {
    pub fn icon_name(self) -> &'static str {
        match self {
            ElectrumSec::Tor => "security-high-symbolic",
            ElectrumSec::Tls => "security-medium-symbolic",
            ElectrumSec::None => "security-low-symbolic",
        }
    }

    pub fn tooltip(self) -> &'static str {
        match self {
            ElectrumSec::Tor => "high security and privacy connection using Tor",
            ElectrumSec::Tls => "medium security connection using SSL/TLS",
            ElectrumSec::None => "unsecure connection",
        }
    }
}

// Create the structure that holds the widgets used in the view.
#[derive(Clone, Gladis)]
pub struct Widgets {
    window: ApplicationWindow,

    header_bar: HeaderBar,
    new_btn: Button,
    open_btn: Button,
    settings_btn: Button,
    pay_btn: Button,
    about_mi: MenuItem,

    balance_btc_lbl: Label,
    balance_sat_lbl: Label,
    balance_fiat_lbl: Label,
    volume_btc_lbl: Label,
    volume_sat_lbl: Label,
    volume_fiat_lbl: Label,
    txcount_lbl: Label,

    refresh_btn: Button,
    refresh_spin: Spinner,
    refresh_img: Image,

    history_store: ListStore,
    utxo_store: ListStore,
    address_store: ListStore,

    address_list: TreeView,
    utxo_list: TreeView,
    history_list: TreeView,

    status_bar: Statusbar,
    status_lbl: Label,
    balance_lbl: Label,
    lastblock_lbl: Label,
    height_lbl: Label,
    network_lbl: Label,
    electrum_lbl: Label,
    connection_img: Image,
    electrum_spin: Spinner,

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
        let settings = model.as_settings();

        self.header_bar
            .set_title(model.path().file_name().and_then(OsStr::to_str));
        let network = settings.network().to_string();
        self.network_lbl
            .set_text(&(network[0..1].to_uppercase() + &network[1..]));
        self.electrum_lbl.set_text(&settings.electrum().server);

        let address = model.as_wallet().next_address();
        self.address_fld.set_text(&address.to_string());
    }

    pub(super) fn connect(&self, relm: &Relm<super::Component>) {
        connect!(relm, self.new_btn, connect_clicked(_), Msg::New);
        connect!(relm, self.open_btn, connect_clicked(_), Msg::Open);
        connect!(relm, self.settings_btn, connect_clicked(_), Msg::Settings);
        connect!(
            relm,
            self.pay_btn,
            connect_clicked(_),
            Msg::PayMsg(pay::Msg::Show)
        );
        connect!(relm, self.refresh_btn, connect_clicked(_), Msg::Refresh);
        connect!(relm, self.about_mi, connect_activate(_), Msg::About);

        connect!(
            relm,
            self.window,
            connect_delete_event(_, _),
            return (Msg::Close, Inhibit(false))
        );
    }

    pub fn update_electrum_server(&self, electrum: &ElectrumServer) {
        self.status_lbl
            .set_text(&"New electrum server, please refresh");
        self.electrum_lbl.set_text(&electrum.server);
        self.electrum_spin.set_visible(false);
        self.connection_img
            .set_icon_name(Some(electrum.sec.icon_name()));
        self.connection_img
            .set_tooltip_text(Some("New electrum server: data needs refresh"));
        self.connection_img.set_visible(true);
    }

    pub fn update_electrum_state(&self, state: ElectrumState) {
        self.status_lbl.set_text(&state.to_string());
        match state {
            ElectrumState::Connecting => {
                self.refresh_btn.set_sensitive(false);
                self.refresh_img.set_visible(false);
                self.refresh_spin.set_visible(true);
                self.refresh_spin.set_active(true);

                self.connection_img.set_visible(false);
                self.electrum_spin.set_visible(true);
                self.electrum_spin.set_visible(true);
            }
            ElectrumState::Complete(sec) => {
                self.refresh_btn.set_sensitive(true);
                self.refresh_img.set_visible(true);
                self.refresh_spin.set_visible(false);

                self.electrum_spin.set_visible(false);
                self.connection_img.set_icon_name(Some(sec.icon_name()));
                self.connection_img.set_tooltip_text(Some(sec.tooltip()));
                self.connection_img.set_visible(true);
                self.pay_btn.set_sensitive(true);
            }
            ElectrumState::Error(err) => {
                self.refresh_btn.set_sensitive(true);
                self.refresh_img.set_visible(true);
                self.refresh_spin.set_visible(false);

                self.electrum_spin.set_visible(false);
                self.connection_img
                    .set_icon_name(Some("emblem-important-symbolic"));
                self.connection_img.set_tooltip_text(Some(&err));
                self.connection_img.set_visible(true);
                self.pay_btn.set_sensitive(false);

                self.status_lbl.set_text("Error from electrum server");
            }
            _ => {}
        }
    }

    pub fn update_last_block(&mut self, last_block: &HeaderNotification) {
        let ts = last_block.header.time;
        let naive = NaiveDateTime::from_timestamp(ts as i64, 0);
        let dt = DateTime::<Utc>::from_utc(naive, Utc);
        let time = dt.time();
        self.lastblock_lbl
            .set_text(&format!("{}", time.format("%-I:%M %p")));
        self.height_lbl.set_text(&last_block.height.to_string());
    }

    pub fn update_fees(&mut self, _: f64, _: f64, _: f64) {
        // TODO: Nothing yet
    }

    pub fn update_history(&mut self, history: &[HistoryTxid]) {
        self.history_store.clear();
        let pixbuf = Image::from_icon_name(Some("list-add-symbolic"), IconSize::Menu).pixbuf();
        for item in history {
            let height = match item.height {
                -1 => s!("pending"),
                height => height.to_string(),
            };
            self.history_store.insert_with_values(
                None,
                &[
                    (0, &pixbuf),
                    (1, &item.address.to_string()),
                    (2, &item.txid.to_string()),
                    (3, &0),
                    (4, &0),
                    (5, &height),
                ],
            );
        }
    }

    pub fn update_utxos(&mut self, utxos: &[UtxoTxid]) {
        self.utxo_store.clear();
        for item in utxos {
            let height = match item.height {
                0 => s!("pending"),
                height => height.to_string(),
            };
            self.utxo_store.insert_with_values(
                None,
                &[
                    (0, &item.address.to_string()),
                    (1, &item.txid.to_string()),
                    (2, &item.value),
                    (3, &height),
                ],
            );
        }
    }

    pub fn update_transactions(&mut self, _transactions: &BTreeMap<Txid, Transaction>) {
        // TODO: Refresh history basing on tx info
    }

    pub fn update_addresses(&mut self, address_info: &[AddressInfo]) {
        self.address_store.clear();
        for info in address_info {
            self.address_store.insert_with_values(
                None,
                &[
                    (0, &info.address.to_string()),
                    (1, &info.balance),
                    (2, &info.volume),
                    (3, &info.tx_count),
                ],
            );
        }
    }

    pub fn update_state(&mut self, state: WalletState, tx_count: usize) {
        self.balance_lbl
            .set_text(&format!("{} sat", state.balance.to_string()));
        self.balance_btc_lbl
            .set_text(&format!("{:.3}", state.balance as f64 / 100_000_000.0));
        self.balance_sat_lbl.set_text(&state.balance.to_string());
        self.volume_btc_lbl
            .set_text(&format!("{:.2}", state.volume as f64 / 100_000_000.0));
        self.volume_sat_lbl.set_text(&state.volume.to_string());
        self.txcount_lbl.set_text(&tx_count.to_string());
    }
}
