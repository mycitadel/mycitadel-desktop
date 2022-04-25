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

use std::io;
use std::thread::JoinHandle;

use amplify::Wrapper;
use bitcoin::Txid;
use electrum_client::{Client as ElectrumClient, ElectrumApi, HeaderNotification};
use relm::Sender;
use wallet::address::address::AddressCompat;
use wallet::hd::{SegmentIndexes, UnhardenedIndex};
use wallet::scripts::PubkeyScript;

use crate::model::WalletSettings;

pub enum WatchMsg {
    LastBlock(HeaderNotification),
    FeeEstimate(Vec<f64>),
    HistoryBatch(Vec<HistoryTxid>),
    Error(electrum_client::Error),
}

pub struct HistoryTxid {
    pub txid: Txid,
    pub height: i32,
    pub address: AddressCompat,
    pub index: UnhardenedIndex,
    pub change: bool,
}

pub struct ElectrumWatcher {
    handle: JoinHandle<()>,
}

impl ElectrumWatcher {
    pub fn with(
        sender: Sender<WatchMsg>,
        wallet_settings: WalletSettings,
    ) -> Result<Self, io::Error> {
        Ok(Self {
            handle: std::thread::Builder::new()
                .name(s!("electrum-watcher"))
                .spawn(move || {
                    let err = electrum_watcher(&sender, wallet_settings).unwrap_err();
                    sender.send(WatchMsg::Error(err)).expect("channel broken");
                })?,
        })
    }
}

pub fn electrum_watcher(
    sender: &Sender<WatchMsg>,
    wallet_settings: WalletSettings,
) -> Result<(), electrum_client::Error> {
    let config = electrum_client::ConfigBuilder::new()
        .timeout(Some(5))
        .expect("we do not use socks here")
        .build();
    let client = ElectrumClient::from_config(&wallet_settings.electrum().to_string(), config)?;

    let last_block = client.block_headers_subscribe()?;
    sender
        .send(WatchMsg::LastBlock(last_block))
        .expect("electrum watcher channel broken");

    let fee = client.batch_estimate_fee([1, 2, 3])?;
    sender
        .send(WatchMsg::FeeEstimate(fee))
        .expect("electrum watcher channel broken");

    let network = bitcoin::Network::from(wallet_settings.network());

    let mut txids = vec![];
    let mut upto_index = map! { true => UnhardenedIndex::zero(), false => UnhardenedIndex::zero() };
    for change in [true, false] {
        let mut offset = 0u16;
        let mut upto = UnhardenedIndex::zero();
        *upto_index.entry(change).or_default() = loop {
            let spk = wallet_settings
                .script_pubkeys(change, offset..=(offset + 19))
                .map_err(|err| electrum_client::Error::Message(err.to_string()))?;
            let history_batch: Vec<_> = client
                .batch_script_get_history(spk.values().map(PubkeyScript::as_inner))?
                .into_iter()
                .zip(spk)
                .flat_map(|(history, (index, script))| {
                    history.into_iter().map(move |res| HistoryTxid {
                        txid: res.tx_hash,
                        height: res.height,
                        address: AddressCompat::from_script(&script.clone().into(), network)
                            .expect("broken descriptor"),
                        index,
                        change,
                    })
                })
                .collect();
            if history_batch.is_empty() {
                break upto;
            } else {
                upto = history_batch
                    .iter()
                    .map(|item| item.index)
                    .max()
                    .unwrap_or_default();
            }
            txids.extend(history_batch.iter().map(|item| item.txid));
            sender
                .send(WatchMsg::HistoryBatch(history_batch))
                .expect("electrum watcher channel broken");
            offset += 20;
        };
    }
    // TODO: Retrieve txids

    // TODO: Retrieve utxos

    // TODO: Subscribe to invoices

    unreachable!();
}
