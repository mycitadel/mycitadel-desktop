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

mod component;
mod pay;
mod view_model;
mod widget;

use std::collections::BTreeSet;

use relm::StreamHandle;
pub(super) use view_model::ViewModel;
pub(self) use widget::Widgets;

pub use self::component::Component;
use crate::model::{DescriptorClass, ElectrumSec, ElectrumServer, Signer};
use crate::view::launch;
use crate::worker::exchange::Fiat;
use crate::worker::{electrum, exchange};

#[derive(Msg)]
pub enum Msg {
    New,
    Open,
    Close,
    About,
    Duplicate,
    Import,
    Launch(launch::Msg),
    Settings,
    Update(Vec<Signer>, BTreeSet<DescriptorClass>, ElectrumServer),
    Pay(pay::Msg),
    Fiat(Fiat),
    Refresh,
    InvoiceAmountToggle(bool),
    InvoiceIndexToggle(bool),
    InvoiceAmount(f64),
    InvoiceIndex(u32),
    ElectrumWatch(electrum::Msg),
    ExchangeRefresh(exchange::Msg),
    RegisterLauncher(StreamHandle<launch::Msg>),
}

#[derive(Clone, PartialEq, Debug, Display)]
#[display(doc_comments)]
pub enum ElectrumState {
    /// Connecting to electrum server...
    Connecting,
    /// Checking latest blockchain state...
    QueryingBlockchainState,
    /// Retrieving fee information...
    RetrievingFees,
    /// Getting transaction history (batch {0})...
    RetrievingHistory(usize),
    /// Reading transactions: {0:.0}%
    RetrievingTransactions(f32),
    /// Ready
    Complete(ElectrumSec),
    /// Electrum error: {0}
    Error(String),
}
