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

use std::collections::BTreeSet;
use std::ffi::OsStr;

use chrono::{DateTime, NaiveDateTime, Utc};
use electrum_client::HeaderNotification;
use gladis::Gladis;
use gtk::prelude::*;
use gtk::{
    ApplicationWindow, Button, Entry, HeaderBar, Image, Label, ListStore, MenuItem, Popover,
    Spinner, Statusbar, TreeView,
};
use relm::Relm;

use super::{pay, ElectrumState, Msg, ViewModel};
use crate::model::{
    AddressSummary, ElectrumSec, ElectrumServer, HistoryEntry, UtxoTxid, WalletState,
};

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
    import_mi: MenuItem,
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
            Msg::Pay(pay::Msg::Show)
        );
        connect!(relm, self.refresh_btn, connect_clicked(_), Msg::Refresh);
        connect!(relm, self.import_mi, connect_activate(_), Msg::Import);
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
        let dt = DateTime::<chrono::Local>::from(DateTime::<Utc>::from_utc(naive, Utc));
        let time = dt.time();
        self.lastblock_lbl
            .set_text(&format!("{}", time.format("%-I:%M %p")));
        self.height_lbl.set_text(&last_block.height.to_string());
    }

    pub fn update_history(&mut self, history: &BTreeSet<HistoryEntry>) {
        self.history_store.clear();
        for item in history {
            let btc = format!("{:+.08}", item.balance() as f64 / 100_000_000.0);
            self.history_store.insert_with_values(
                None,
                &[
                    (0, &item.icon_name()),
                    (2, &item.onchain.txid.to_string()),
                    (3, &btc),
                    (4, &"?"),
                    (5, &item.mining_info()),
                    (6, &item.color()),
                ],
            );
        }
    }

    pub fn update_utxos(&mut self, utxos: &BTreeSet<UtxoTxid>) {
        self.utxo_store.clear();
        for item in utxos {
            let btc = format_btc_value(item.value);
            self.utxo_store.insert_with_values(
                None,
                &[
                    (0, &item.addr_src.address.to_string()),
                    (1, &item.onchain.txid.to_string()),
                    (2, &btc),
                    (3, &item.mining_info()),
                ],
            );
        }
    }

    pub fn update_addresses(&mut self, address_info: &[AddressSummary]) {
        self.address_store.clear();
        for info in address_info {
            let balance = format_btc_value(info.balance);
            let volume = format_btc_value(info.volume);
            let terminal = info.terminal_string();
            self.address_store.insert_with_values(
                None,
                &[
                    (0, &info.addr_src.address.to_string()),
                    (1, &balance),
                    (2, &volume),
                    (3, &info.tx_count),
                    (4, &info.icon_name()),
                    (5, &terminal),
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

fn format_btc_value(value: u64) -> String {
    if value == 0 {
        s!("0")
    } else {
        format!("{:.08}", value as f64 / 100_000_000.0)
    }
}
