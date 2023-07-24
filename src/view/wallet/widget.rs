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

use bpro::{
    AddressSummary, ElectrumSec, ElectrumServer, HistoryEntry, OnchainStatus, OnchainTxid,
    UtxoTxid, WalletState,
};
use chrono::{DateTime, NaiveDateTime, Utc};
use electrum_client::HeaderNotification;
use gladis::Gladis;
use gtk::gdk_pixbuf::Pixbuf;
use gtk::prelude::*;
use gtk::{
    gdk, Adjustment, ApplicationWindow, Button, CheckButton, Entry, HeaderBar, Image, Label,
    ListBox, ListStore, Menu, MenuItem, Notebook, Popover, RadioMenuItem, SortColumn, SortType,
    SpinButton, Spinner, Statusbar, TextBuffer, TreeView,
};
use relm::Relm;
use rgb::contract::SealWitness;
use rgbstd::interface::FungibleAllocation;
use rgbstd::stl::Precision;
use wallet::hd::SegmentIndexes;

use super::asset_row::{self, AssetModel};
use super::{payto, ElectrumState, Msg, ViewModel};
use crate::model::{display_accounting_amount, FormatDate, UI as UIColorTrait};
use crate::view::{launch, APP_ICON};
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
    new_btn: Button,
    open_btn: Button,
    settings_btn: Button,
    redefine_mi: MenuItem,
    import_mi: MenuItem,
    settings_mi: MenuItem,
    launcher_mi: MenuItem,
    about_mi: MenuItem,

    refresh_btn: Button,
    refresh_spin: Spinner,
    refresh_img: Image,

    main_tabs: Notebook,

    paybtc_btn: Button,
    pay20_btn: Button,

    balance_lead_lbl: Label,
    balance_tail_lbl: Label,
    balance_zero_lbl: Label,
    balance_fiat_lbl: Label,
    balance_cents_lbl: Label,
    fiat_name_lbl: Label,
    value_lbl: Label,

    exchange_lbl: Label,
    fiat_usd: RadioMenuItem,
    fiat_eur: RadioMenuItem,
    fiat_chf: RadioMenuItem,
    fiat_pair_lbl: Label,

    asset_list: ListBox,

    ticker_lbl: Label,
    asset_lbl: Label,
    details_lbl: Label,
    id20_entry: Entry,
    asset_lead_lbl: Label,
    asset_tail_lbl: Label,
    asset_zero_lbl: Label,

    history_store: ListStore,
    utxo_store: ListStore,
    address_store: ListStore,
    allocation_store: ListStore,
    operation_store: ListStore,

    address_list: TreeView,
    utxo_list: TreeView,
    history_list: TreeView,
    allocation_list: TreeView,
    operation_list: TreeView,

    history_menu: Menu,
    hist_copy_txid_mi: MenuItem,
    hist_copy_desc_mi: MenuItem,
    hist_copy_amount_mi: MenuItem,
    hist_copy_balance_mi: MenuItem,
    hist_copy_height_mi: MenuItem,

    address_menu: Menu,
    addr_copy_mi: MenuItem,
    addr_copy_volume_mi: MenuItem,
    addr_copy_balance_mi: MenuItem,

    coin_menu: Menu,
    coin_copy_txid_mi: MenuItem,
    coin_copy_addr_mi: MenuItem,
    coin_copy_amount_mi: MenuItem,
    coin_copy_height_mi: MenuItem,

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

    contract_text: TextBuffer,
    import_popover: Popover,
    import_btn: Button,
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
            self.paybtc_btn,
            connect_clicked(_),
            Msg::PayTo(payto::Msg::Show)
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
            self.asset_list,
            connect_row_activated(_, row),
            Msg::ChangeAsset(row.index() as u32)
        );

        let menu = self.history_menu.clone();
        self.history_list
            .connect_button_release_event(move |me, event| {
                if event.button() == 3 {
                    me.emit_popup_menu();
                    Inhibit(true)
                } else {
                    Inhibit(false)
                }
            });
        self.history_list.connect_popup_menu(move |_me| {
            menu.popup(None::<&Menu>, None::<&MenuItem>, |_, _, _| false, 0, 0);
            true
        });

        let menu = self.coin_menu.clone();
        self.utxo_list
            .connect_button_release_event(move |me, event| {
                if event.button() == 3 {
                    me.emit_popup_menu();
                    Inhibit(true)
                } else {
                    Inhibit(false)
                }
            });
        self.utxo_list.connect_popup_menu(move |_me| {
            menu.popup(None::<&Menu>, None::<&MenuItem>, |_, _, _| false, 0, 0);
            true
        });

        let menu = self.address_menu.clone();
        self.address_list
            .connect_button_release_event(move |me, event| {
                if event.button() == 3 {
                    me.emit_popup_menu();
                    Inhibit(true)
                } else {
                    Inhibit(false)
                }
            });
        self.address_list.connect_popup_menu(move |_me| {
            menu.popup(None::<&Menu>, None::<&MenuItem>, |_, _, _| false, 0, 0);
            true
        });

        let list = self.history_list.clone();
        self.hist_copy_txid_mi.connect_activate(move |_| {
            if let Some(iter) = list.selection().selected().map(|(_, iter)| iter) {
                let val = list.model().unwrap().value(&iter, 1);
                gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD).set_text(val.get::<&str>().unwrap());
            }
        });
        let list = self.history_list.clone();
        self.hist_copy_desc_mi.connect_activate(move |_| {
            if let Some(iter) = list.selection().selected().map(|(_, iter)| iter) {
                let val = list.model().unwrap().value(&iter, 7);
                gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD).set_text(val.get::<&str>().unwrap());
            }
        });
        let list = self.history_list.clone();
        self.hist_copy_amount_mi.connect_activate(move |_| {
            if let Some(iter) = list.selection().selected().map(|(_, iter)| iter) {
                let val = list.model().unwrap().value(&iter, 2);
                gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD).set_text(val.get::<&str>().unwrap());
            }
        });
        let list = self.history_list.clone();
        self.hist_copy_balance_mi.connect_activate(move |_| {
            if let Some(iter) = list.selection().selected().map(|(_, iter)| iter) {
                let val = list.model().unwrap().value(&iter, 3);
                gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD).set_text(val.get::<&str>().unwrap());
            }
        });
        let list = self.history_list.clone();
        self.hist_copy_height_mi.connect_activate(move |_| {
            if let Some(iter) = list.selection().selected().map(|(_, iter)| iter) {
                let val = list.model().unwrap().value(&iter, 6);
                gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD)
                    .set_text(&val.get::<u32>().unwrap().to_string());
            }
        });

        let list = self.utxo_list.clone();
        self.coin_copy_txid_mi.connect_activate(move |_| {
            if let Some(iter) = list.selection().selected().map(|(_, iter)| iter) {
                let val = list.model().unwrap().value(&iter, 1);
                gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD).set_text(val.get::<&str>().unwrap());
            }
        });
        let list = self.utxo_list.clone();
        self.coin_copy_addr_mi.connect_activate(move |_| {
            if let Some(iter) = list.selection().selected().map(|(_, iter)| iter) {
                let val = list.model().unwrap().value(&iter, 0);
                gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD).set_text(val.get::<&str>().unwrap());
            }
        });
        let list = self.utxo_list.clone();
        self.coin_copy_amount_mi.connect_activate(move |_| {
            if let Some(iter) = list.selection().selected().map(|(_, iter)| iter) {
                let val = list.model().unwrap().value(&iter, 2);
                gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD).set_text(val.get::<&str>().unwrap());
            }
        });
        let list = self.utxo_list.clone();
        self.coin_copy_height_mi.connect_activate(move |_| {
            if let Some(iter) = list.selection().selected().map(|(_, iter)| iter) {
                let val = list.model().unwrap().value(&iter, 4);
                gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD)
                    .set_text(&val.get::<u32>().unwrap().to_string());
            }
        });

        let list = self.address_list.clone();
        self.addr_copy_mi.connect_activate(move |_| {
            if let Some(iter) = list.selection().selected().map(|(_, iter)| iter) {
                let val = list.model().unwrap().value(&iter, 0);
                gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD).set_text(val.get::<&str>().unwrap());
            }
        });
        let list = self.address_list.clone();
        self.addr_copy_volume_mi.connect_activate(move |_| {
            if let Some(iter) = list.selection().selected().map(|(_, iter)| iter) {
                let val = list.model().unwrap().value(&iter, 2);
                gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD).set_text(val.get::<&str>().unwrap());
            }
        });
        let list = self.address_list.clone();
        self.addr_copy_balance_mi.connect_activate(move |_| {
            if let Some(iter) = list.selection().selected().map(|(_, iter)| iter) {
                let val = list.model().unwrap().value(&iter, 1);
                gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD).set_text(val.get::<&str>().unwrap());
            }
        });

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

        let import_btn = self.import_btn.clone();
        self.contract_text.connect_changed(move |buffer| {
            import_btn.set_sensitive(buffer.char_count() > 0);
        });
        let popover = self.import_popover.clone();
        let contract_text = self.contract_text.clone();
        let stream = relm.stream().clone();
        self.import_btn.connect_clicked(move |_| {
            let (start, end) = contract_text.bounds();
            let text = contract_text.text(&start, &end, false).unwrap().to_string();
            stream.emit(Msg::ImportRgbContract(text));
            contract_text.set_text("");
            popover.hide();
        });

        connect!(
            relm,
            self.window,
            connect_delete_event(_, _),
            return (Msg::Close, Inhibit(false))
        );
    }

    pub fn init_ui(&mut self, model: &ViewModel) {
        let settings = model.as_settings();

        let icon = Pixbuf::from_read(APP_ICON).expect("app icon is missed");
        self.window.set_icon(Some(&icon));

        self.header_bar
            .set_title(model.path().file_name().and_then(OsStr::to_str));
        self.header_bar
            .set_subtitle(Some(&format!("{}", settings.core())));
        let network = settings.network().to_string();
        self.network_lbl
            .set_text(&(network[0..1].to_uppercase() + &network[1..]));
        self.electrum_lbl.set_text(&settings.electrum().server);

        self.fiat_usd.set_active(model.fiat == Fiat::USD);
        self.fiat_eur.set_active(model.fiat == Fiat::EUR);
        self.fiat_chf.set_active(model.fiat == Fiat::CHF);

        if !settings.is_rgb() {
            self.main_tabs.set_show_tabs(false);
        }

        self.bind_asset_model(model.asset_model());

        self.history_store
            .set_sort_column_id(SortColumn::Index(6), SortType::Descending);
        self.utxo_store
            .set_sort_column_id(SortColumn::Index(4), SortType::Descending);
        self.address_store
            .set_sort_column_id(SortColumn::Index(6), SortType::Ascending);
        self.operation_store
            .set_sort_column_id(SortColumn::Index(6), SortType::Descending);
        self.allocation_store
            .set_sort_column_id(SortColumn::Index(3), SortType::Descending);

        self.update_invoice(model);
    }

    fn bind_asset_model(&self, model: &AssetModel) {
        self.asset_list
            .bind_model(Some(model), move |item| asset_row::RowWidgets::init(item));
    }

    pub fn update_asset(&mut self, model: &mut ViewModel) {
        let info = model.asset_info();
        self.ticker_lbl.set_text(&info.ticker());
        self.asset_lbl.set_text(&info.name());
        self.details_lbl.set_text(&info.details());
        self.id20_entry.set_text(&info.contract_name());

        self.update_outpoints(model);
        self.update_balance(model);
    }

    pub fn update_invoice(&self, model: &ViewModel) {
        let invoice = model.as_invoice();
        let wallet = model.wallet();
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
                self.paybtc_btn.set_sensitive(true);
                self.pay20_btn.set_sensitive(true);
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
                self.paybtc_btn.set_sensitive(false);
                self.pay20_btn.set_sensitive(false);

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
            let descr_color = gdk::RGBA::new(80.0 / 255.0, 80.0 / 255.0, 80.0 / 255.0, 1.0);
            let date = match item.onchain.status {
                OnchainStatus::Blockchain(height) => item
                    .onchain
                    .date_time()
                    .map(|dt| dt.format("%F %H:%M").to_string())
                    .unwrap_or_else(|| format!("{height}")),
                OnchainStatus::Mempool => s!("mempool"),
            };
            self.history_store.insert_with_values(None, &[
                (0, &item.icon_name()),
                (1, &item.onchain.txid.to_string()),
                (2, &btc),
                (3, &btc_balance),
                (4, &date),
                (5, &item.color()),
                (6, &item.onchain.status.into_u32()),
                // TODO: Use description
                (7, &item.onchain.txid.to_string()),
                // TODO: Change color depending on the presence of description
                (8, &descr_color),
            ]);
        }
    }

    pub fn update_outpoints(&mut self, model: &mut ViewModel) {
        self.update_utxos(model.wallet().utxos());

        if model.asset().is_some() {
            let info = model.asset_info();
            let allocations = model.asset_allocations();
            let rgb = model.wallet().rgb().unwrap();
            self.update_allocations(
                allocations,
                info.precision(),
                &info.issue(),
                rgb.witness_txes(),
            );
        }
    }

    pub fn update_operations(&mut self, model: &mut ViewModel) {
        let info = model.asset_info();
        let operations = model.asset_allocations();
        let rgb = model.wallet().rgb().unwrap();

        let precision = info.precision();
        let issue = info.issue();
        let witness_txes = rgb.witness_txes();

        let pow = 10u64.pow(precision as u32);
        self.operation_store.clear();
        for allocation in allocations {
            let int = allocation.value / pow;
            let fract = allocation.value - int * pow;
            let date = match allocation.witness {
                SealWitness::Genesis => issue.to_string(),
                SealWitness::Present(txid) => witness_txes
                    .iter()
                    .find(|info| info.txid.as_ref() == txid.as_ref().as_slice())
                    .map(OnchainTxid::format_date)
                    .unwrap_or_else(|| s!("unknown")),
                SealWitness::Extension => s!("issue"),
            };
            self.operation_store.insert_with_values(None, &[
                (0, &item.icon_name()),
                (1, &item.onchain.txid.to_string()),
                (2, &btc),
                (3, &btc_balance),
                (4, &date),
                (5, &item.color()),
                (6, &item.onchain.status.into_u32()),
                // TODO: Use description
                (7, &item.onchain.txid.to_string()),
                // TODO: Change color depending on the presence of description
                (8, &descr_color),
            ]);
        }
    }

    pub fn update_allocations(
        &mut self,
        allocations: Vec<FungibleAllocation>,
        precision: u8,
        issue: &str,
        witness_txes: &BTreeSet<OnchainTxid>,
    ) {
        let pow = 10u64.pow(precision as u32);
        self.allocation_store.clear();
        for allocation in allocations {
            let int = allocation.value / pow;
            let fract = allocation.value - int * pow;
            let date = match allocation.witness {
                SealWitness::Genesis => issue.to_string(),
                SealWitness::Present(txid) => witness_txes
                    .iter()
                    .find(|info| info.txid.as_ref() == txid.as_ref().as_slice())
                    .map(OnchainTxid::format_date)
                    .unwrap_or_else(|| s!("unknown")),
                SealWitness::Extension => s!("issue"),
            };
            self.allocation_store.insert_with_values(None, &[
                (0, &allocation.owner.to_string()),
                (
                    1,
                    &format!("{int}.{fract}")
                        .trim_end_matches('0')
                        .trim_end_matches('.'),
                ),
                (2, &date),
                (3, &0u32),
            ]);
        }
    }

    pub fn update_utxos(&mut self, utxos: &BTreeSet<UtxoTxid>) {
        self.utxo_store.clear();
        for item in utxos {
            self.utxo_store.insert_with_values(None, &[
                (0, &item.addr_src.address.to_string()),
                (1, &format!("{}:{}", item.onchain.txid, item.vout)),
                (2, &format_btc_value(item.value)),
                (3, &item.onchain.format_date()),
                (4, &item.onchain.status.into_u32()),
            ]);
        }
    }

    pub fn update_addresses(&mut self, address_info: &[AddressSummary]) {
        self.address_store.clear();
        for info in address_info {
            let balance = format_btc_value(info.balance);
            let volume = format_btc_value(info.volume);
            let terminal = info.terminal_string();
            let terminal_sort =
                (info.addr_src.index.first_index() as u64) | ((info.addr_src.change as u64) << 32);
            let style = self.address_list.style_context();
            let addr_color = match (info.balance == 0, info.volume == 0) {
                (true, true) => style.lookup_color("theme_text_color").unwrap(),
                (true, false) => gdk::RGBA::parse("grey").unwrap(),
                (false, false) => gdk::RGBA::parse("dark grey").unwrap(),
                _ => unreachable!("address with zero volume but positive balance"),
            };
            self.address_store.insert_with_values(None, &[
                (0, &info.addr_src.address.to_string()),
                (1, &balance),
                (2, &volume),
                (3, &info.tx_count),
                (4, &info.icon_name()),
                (5, &terminal),
                (6, &terminal_sort),
                (7, &addr_color),
            ]);
        }
    }

    pub fn update_balance(&self, model: &mut ViewModel) {
        self.update_btc_balance(model);
        self.update_asset_balance(model);
    }

    pub fn update_btc_balance(&self, model: &mut ViewModel) {
        let wallet = model.wallet();
        let state = wallet.state();
        let exchange_rate = model.exchange_rate;

        display_accounting_amount(
            state.balance,
            Precision::default(),
            &self.balance_lead_lbl,
            &self.balance_tail_lbl,
            &self.balance_zero_lbl,
        );

        let s = format!("{:.02}", state.balance_btc() * exchange_rate);
        let (fiat, cents) = s.split_once('.').expect("formatting produces decimal");
        self.balance_fiat_lbl.set_text(fiat);
        self.balance_cents_lbl.set_text(cents);

        self.balance_lbl.set_text(&format!("{} sat", state.balance));
    }

    pub fn update_asset_balance(&self, model: &mut ViewModel) {
        let asset = model.asset_info();
        let precision: u8 = asset.property("precision");
        let balance: u64 = asset.property("amount");
        display_accounting_amount(
            balance,
            precision,
            &self.asset_lead_lbl,
            &self.asset_tail_lbl,
            &self.asset_zero_lbl,
        );
    }

    pub fn update_fiat(&self, fiat: Fiat) {
        self.fiat_pair_lbl.set_text(fiat.pair());
        self.fiat_name_lbl.set_text(fiat.symbol());

        self.exchange_lbl.set_text(&"...");
        self.balance_fiat_lbl.set_text("?");
        self.balance_cents_lbl.set_text("");
        //self.volume_fiat_lbl.set_text("?");
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

            let s = format!("{:.02}", state.balance_btc() * exchange_rate);
            let (fiat, cents) = s.split_once('.').expect("formatting produces decimal");
            self.balance_fiat_lbl.set_text(fiat);
            self.balance_cents_lbl.set_text(cents);
            //self.volume_fiat_lbl
            //    .set_text(&format!("{:.2}", state.volume_btc() * exchange_rate));
        }
    }

    pub fn update_exchange_error(&self, _err: String) {
        self.exchange_lbl.set_text(&"n/a");
        self.balance_fiat_lbl.set_text("n/a");
        self.balance_cents_lbl.set_text("");
        //self.volume_fiat_lbl.set_text("n/a");
    }
}

fn format_btc_value(value: u64) -> String {
    if value == 0 {
        s!("0")
    } else {
        format!("{:.08}", value as f64 / 100_000_000.0)
    }
}
