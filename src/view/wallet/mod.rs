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
mod view_model;
mod widget;

pub use self::component::Component;
pub(super) use view_model::ViewModel;
pub(self) use widget::Widgets;

use std::collections::BTreeSet;
use std::path::PathBuf;

use relm::StreamHandle;

use crate::model::{DescriptorClass, ElectrumSec, ElectrumServer, Signer};
use crate::view::{launch, pay};
use crate::worker::electrum;

#[derive(Msg)]
pub enum Msg {
    New,
    Open,
    Close,
    About,
    FileError(PathBuf, String),
    Save,
    Settings,
    Update(Vec<Signer>, BTreeSet<DescriptorClass>, ElectrumServer),
    PayMsg(pay::Msg),
    Receive,
    Refresh,
    Select(usize),
    Create,
    ElectrumWatch(electrum::Msg),
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
