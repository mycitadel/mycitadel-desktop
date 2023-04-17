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

use std::collections::BTreeSet;
use std::ffi::OsStr;

use bpro::{AddressSummary, ElectrumSec, ElectrumServer, HistoryEntry, UtxoTxid, WalletState};
use chrono::{DateTime, NaiveDateTime, Utc};
use electrum_client::HeaderNotification;
use gladis::Gladis;
use gtk::gdk_pixbuf::Pixbuf;
use gtk::prelude::*;
use gtk::{
    gdk, Adjustment, ApplicationWindow, Button, CheckButton, Entry, HeaderBar, Image, Label,
    ListBox, ListStore, MenuItem, Popover, RadioMenuItem, SpinButton, Spinner, Statusbar, TreeView,
};
use relm::Relm;
use wallet::hd::SegmentIndexes;

use super::asset_row::{self, AssetModel};
use super::{pay, ElectrumState, Msg, ViewModel};
use crate::model::UI as UIColorTrait;
use crate::view::{launch, APP_ICON, APP_ICON_TOOL};
use crate::worker::exchange::{Exchange, Fiat};

trait UI {
    fn icon_name(self) -> &'static str;
    fn tooltip(self) -> &'static str;
}

impl UI for ElectrumSec {
    fn icon_name(self) -> &'static str {
        match self {
            ElectrumSec::Tor => "security-high-symbolic",
            ElectrumSec::Tls => "security-medium-symbolic",
            ElectrumSec::None => "security-low-symbolic",
        }
    }

    fn tooltip(self) -> &'static str {
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
    logo_img: Image,
    new_btn: Button,
    open_btn: Button,
    settings_btn: Button,
    pay_btn: Button,
    redefine_mi: MenuItem,
    import_mi: MenuItem,
    settings_mi: MenuItem,
    launcher_mi: MenuItem,
    about_mi: MenuItem,

    asset_list: ListBox,

    balance_btc_lbl: Label,
    balance_sat_lbl: Label,
    balance_fiat_lbl: Label,
    volume_btc_lbl: Label,
    volume_sat_lbl: Label,
    volume_fiat_lbl: Label,
    txcount_lbl: Label,

    exchange_lbl: Label,
    fiat_usd: RadioMenuItem,
    fiat_eur: RadioMenuItem,
    fiat_chf: RadioMenuItem,
    fiat_pair_lbl: Label,
    fiat_name1_lbl: Label,
    fiat_name2_lbl: Label,

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
    amount_chk: CheckButton,
    amount_stp: SpinButton,
    amount_adj: Adjustment,
    index_chk: CheckButton,
    index_stp: SpinButton,
    index_adj: Adjustment,
    index_img: Image,
    address_fld: Entry,
}

impl Widgets {
    pub fn show(&self) { self.window.show() }
    pub fn hide(&self) { self.window.hide() }
    pub fn close(&self) { self.window.close() }

    pub fn to_root(&self) -> ApplicationWindow { self.window.clone() }
    pub fn as_root(&self) -> &ApplicationWindow { &self.window }

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
        connect!(relm, self.redefine_mi, connect_activate(_), Msg::Duplicate);
        connect!(relm, self.import_mi, connect_activate(_), Msg::Import);
        connect!(relm, self.settings_mi, connect_activate(_), Msg::Settings);
        connect!(
            relm,
            self.launcher_mi,
            connect_activate(_),
            Msg::Launch(launch::Msg::Show)
        );
        connect!(relm, self.about_mi, connect_activate(_), Msg::About);

        connect!(
            relm,
            self.fiat_usd,
            connect_activate(_),
            Msg::Fiat(Fiat::USD)
        );
        connect!(
            relm,
            self.fiat_eur,
            connect_activate(_),
            Msg::Fiat(Fiat::EUR)
        );
        connect!(
            relm,
            self.fiat_chf,
            connect_activate(_),
            Msg::Fiat(Fiat::CHF)
        );

        connect!(
            relm,
            self.amount_chk,
            connect_toggled(chk),
            Msg::InvoiceAmountToggle(chk.is_active())
        );
        connect!(
            relm,
            self.index_chk,
            connect_toggled(chk),
            Msg::InvoiceIndexToggle(chk.is_active())
        );
        connect!(
            relm,
            self.amount_adj,
            connect_value_changed(adj),
            Msg::InvoiceAmount(adj.value())
        );
        connect!(
            relm,
            self.index_adj,
            connect_value_changed(adj),
            Msg::InvoiceIndex(adj.value() as u32)
        );

        self.address_fld.connect_icon_press(|entry, _, _| {
            let val = entry.text();
            gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD).set_text(&val);
        });

        connect!(
            relm,
            self.window,
            connect_delete_event(_, _),
            return (Msg::Close, Inhibit(false))
        );
    }

    pub fn init_ui(&self, model: &ViewModel) {
        let settings = model.as_settings();

        let icon = Pixbuf::from_read(APP_ICON).expect("app icon is missed");
        self.window.set_icon(Some(&icon));

        let img = Pixbuf::from_read(APP_ICON_TOOL).expect("small app icon is missed");
        self.logo_img.set_pixbuf(Some(&img));

        self.header_bar
            .set_title(model.path().file_name().and_then(OsStr::to_str));
        let network = settings.network().to_string();
        self.network_lbl
            .set_text(&(network[0..1].to_uppercase() + &network[1..]));
        self.electrum_lbl.set_text(&settings.electrum().server);

        self.fiat_usd.set_active(model.fiat == Fiat::USD);
        self.fiat_eur.set_active(model.fiat == Fiat::EUR);
        self.fiat_chf.set_active(model.fiat == Fiat::CHF);

        self.bind_asset_model(model.asset_model());

        self.update_invoice(model);
    }

    fn bind_asset_model(&self, model: &AssetModel) {
        self.asset_list
            .bind_model(Some(model), move |item| asset_row::RowWidgets::init(item));
    }

    pub fn update_invoice(&self, model: &ViewModel) {
        let invoice = model.as_invoice();
        let wallet = model.as_wallet();
        let next_index = wallet.next_default_index();
        let address = wallet.indexed_address(invoice.index.unwrap_or(next_index));
        let index_reuse = invoice.index.unwrap_or(next_index) >= next_index;

        self.amount_chk.set_active(invoice.amount.is_some());
        self.amount_stp.set_sensitive(invoice.amount.is_some());

        self.index_chk.set_active(invoice.index.is_some());
        self.index_stp.set_sensitive(invoice.index.is_some());
        self.index_adj
            .set_upper((next_index.first_index() + 19) as f64);
        self.index_adj
            .set_value(invoice.index.unwrap_or(next_index).first_index() as f64);
        self.index_img.set_visible(!index_reuse);

        let invoice_str = match invoice.amount {
            Some(amount) => format!(
                "bitcoin:{}?amount={}",
                address,
                amount as f64 / 100_000_000.0
            ),
            None => address.to_string(),
        };

        self.address_fld.set_text(&invoice_str);
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
        let naive = NaiveDateTime::from_timestamp_opt(ts as i64, 0).expect("invalid block time");
        let dt = DateTime::<chrono::Local>::from(DateTime::<Utc>::from_utc(naive, Utc));
        let time = dt.time();
        self.lastblock_lbl
            .set_text(&format!("{}", time.format("%-I:%M %p")));
        self.height_lbl.set_text(&last_block.height.to_string());
    }

    pub fn update_history(&mut self, history: &BTreeSet<HistoryEntry>) {
        self.history_store.clear();
        let mut balance = 0i64;
        for item in history {
            balance += item.balance();
            let btc = format!("{:+.08}", item.balance() as f64 / 100_000_000.0);
            let btc_balance = format!("{:.08}", balance as f64 / 100_000_000.0);
            self.history_store.insert_with_values(None, &[
                (0, &item.icon_name()),
                (1, &item.onchain.txid.to_string()),
                (2, &btc),
                (3, &btc_balance),
                (4, &item.mining_info()),
                (5, &item.color()),
            ]);
        }
    }

    pub fn update_utxos(&mut self, utxos: &BTreeSet<UtxoTxid>) {
        self.utxo_store.clear();
        for item in utxos {
            let btc = format_btc_value(item.value);
            self.utxo_store.insert_with_values(None, &[
                (0, &item.addr_src.address.to_string()),
                (1, &item.onchain.txid.to_string()),
                (2, &btc),
                (3, &item.mining_info()),
            ]);
        }
    }

    pub fn update_addresses(&mut self, address_info: &[AddressSummary]) {
        self.address_store.clear();
        for info in address_info {
            let balance = format_btc_value(info.balance);
            let volume = format_btc_value(info.volume);
            let terminal = info.terminal_string();
            self.address_store.insert_with_values(None, &[
                (0, &info.addr_src.address.to_string()),
                (1, &balance),
                (2, &volume),
                (3, &info.tx_count),
                (4, &info.icon_name()),
                (5, &terminal),
            ]);
        }
    }

    pub fn update_state(&self, state: WalletState, tx_count: usize, exchange_rate: f64) {
        self.balance_lbl
            .set_text(&format!("{} sat", state.balance.to_string()));
        self.balance_btc_lbl
            .set_text(&format!("{:.4}", state.balance_btc()));
        self.balance_sat_lbl.set_text(&state.balance.to_string());
        self.volume_btc_lbl
            .set_text(&format!("{:.2}", state.volume_btc()));
        self.volume_sat_lbl.set_text(&state.volume.to_string());
        self.txcount_lbl.set_text(&tx_count.to_string());

        self.balance_fiat_lbl
            .set_text(&format!("{:.2}", state.balance_btc() * exchange_rate));
        self.volume_fiat_lbl
            .set_text(&format!("{:.2}", state.volume_btc() * exchange_rate));
    }

    pub fn update_fiat(&self, fiat: Fiat) {
        self.fiat_pair_lbl.set_text(fiat.pair());
        self.fiat_name1_lbl.set_text(fiat.fiat());
        self.fiat_name2_lbl.set_text(fiat.fiat());

        self.exchange_lbl.set_text(&"...");
        self.balance_fiat_lbl.set_text("?");
        self.volume_fiat_lbl.set_text("?");
    }

    pub fn update_exchange_rate(
        &self,
        fiat: Fiat,
        _exchange: Exchange,
        exchange_rate: f64,
        state: WalletState,
    ) {
        self.update_fiat(fiat);

        if exchange_rate > 0.0 {
            self.exchange_lbl.set_text(&format!("{:.0}", exchange_rate));
            self.balance_fiat_lbl
                .set_text(&format!("{:.2}", state.balance_btc() * exchange_rate));
            self.volume_fiat_lbl
                .set_text(&format!("{:.2}", state.volume_btc() * exchange_rate));
        }
    }

    pub fn update_exchange_error(&self, _err: String) {
        self.exchange_lbl.set_text(&"n/a");
        self.balance_fiat_lbl.set_text("n/a");
        self.volume_fiat_lbl.set_text("n/a");
    }
}

fn format_btc_value(value: u64) -> String {
    if value == 0 {
        s!("0")
    } else {
        format!("{:.08}", value as f64 / 100_000_000.0)
    }
}
