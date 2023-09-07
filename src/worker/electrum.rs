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

use std::collections::{BTreeMap, BTreeSet};
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::Duration;
use std::{io, thread};

use amplify::Wrapper;
use bitcoin::Transaction;
use bitcoin_scripts::PubkeyScript;
use bpro::{AddressSource, ElectrumServer, OnchainStatus, TxidMeta, UtxoTxid, WalletSettings};
use chrono::{DateTime, NaiveDateTime, Utc};
use electrum_client::{Client as ElectrumClient, ElectrumApi, HeaderNotification};
use relm::Sender;
use wallet::hd::{SegmentIndexes, UnhardenedIndex};

enum Cmd {
    Sync,
    Pull,
    Update(ElectrumServer),
}

pub enum Msg {
    Connecting,
    Connected,
    Complete,
    LastBlock(HeaderNotification),
    LastBlockUpdate(HeaderNotification),
    FeeEstimate(f64, f64, f64),
    TxidBatch(BTreeMap<AddressSource, BTreeSet<TxidMeta>>, u16),
    UtxoBatch(BTreeSet<UtxoTxid>, u16),
    TxBatch(Vec<Transaction>, f32),
    ChannelDisconnected,
    Error(electrum_client::Error),
}

pub struct ElectrumWorker {
    worker_thread: JoinHandle<()>,
    watcher_thread: JoinHandle<()>,
    tx: mpsc::Sender<Cmd>,
}

impl ElectrumWorker {
    pub fn with(
        sender: Sender<Msg>,
        mut wallet_settings: WalletSettings,
        interval: u64,
    ) -> Result<Self, io::Error> {
        let (tx, rx) = mpsc::channel::<Cmd>();
        let worker_thread = thread::Builder::new().name(s!("electrum")).spawn(move || {
            let mut client = electrum_init(wallet_settings.electrum(), &sender);

            loop {
                let _ = match (&client, rx.recv()) {
                    (Some(_), Ok(Cmd::Update(electrum))) => {
                        wallet_settings.update_electrum(electrum);
                        client = electrum_init(wallet_settings.electrum(), &sender);
                        Ok(())
                    }
                    (Some(client), Ok(Cmd::Sync)) => {
                        electrum_sync(&client, &wallet_settings, &sender)
                    }
                    (Some(client), Ok(Cmd::Pull)) => client.block_headers_pop().map(|res| {
                        if let Some(last_block) = res {
                            sender
                                .send(Msg::LastBlockUpdate(last_block))
                                .expect("electrum watcher channel is broken");
                        }
                    }),
                    (None, Ok(_)) => {
                        /* Can't handle since no client available */
                        Ok(())
                    }
                    (_, Err(_)) => {
                        sender
                            .send(Msg::ChannelDisconnected)
                            .expect("electrum channel is broken");
                        Ok(())
                    }
                }
                .map_err(|err| {
                    sender
                        .send(Msg::Error(err))
                        .expect("electrum channel is broken");
                });
            }
        })?;

        let sender = tx.clone();
        let watcher_thread = thread::Builder::new()
            .name(s!("blockwatcher"))
            .spawn(move || loop {
                thread::sleep(Duration::from_secs(interval));
                sender.send(Cmd::Pull).expect("Electrum thread is dead")
            })
            .expect("unable to start blockchain watching thread");

        Ok(ElectrumWorker {
            tx,
            worker_thread,
            watcher_thread,
        })
    }

    pub fn sync(&self) { self.cmd(Cmd::Sync) }

    pub fn pull(&self) { self.cmd(Cmd::Pull) }

    pub fn update(&self, server: ElectrumServer) { self.cmd(Cmd::Update(server)) }

    fn cmd(&self, cmd: Cmd) { self.tx.send(cmd).expect("Electrum thread is dead") }
}

pub fn electrum_connect(url: &str) -> Result<ElectrumClient, electrum_client::Error> {
    let config = electrum_client::ConfigBuilder::new()
        .timeout(Some(5))
        .build();
    ElectrumClient::from_config(url, config)
}

fn electrum_init(electrum: &ElectrumServer, sender: &Sender<Msg>) -> Option<ElectrumClient> {
    electrum_connect(&electrum.to_string())
        .map_err(|err| {
            sender
                .send(Msg::Error(err))
                .expect("electrum channel is broken");
        })
        .ok()
}

fn electrum_sync(
    client: &ElectrumClient,
    wallet_settings: &WalletSettings,
    sender: &Sender<Msg>,
) -> Result<(), electrum_client::Error> {
    sender
        .send(Msg::Connecting)
        .expect("electrum watcher channel is broken");

    sender
        .send(Msg::Connected)
        .expect("electrum watcher channel is broken");

    let last_block = client.block_headers_subscribe()?;
    sender
        .send(Msg::LastBlock(last_block))
        .expect("electrum watcher channel is broken");

    let fee = client.batch_estimate_fee([1, 2, 3])?;
    sender
        .send(Msg::FeeEstimate(fee[0], fee[1], fee[2]))
        .expect("electrum watcher channel is broken");

    let network = bitcoin::Network::from(wallet_settings.network());

    let mut txids = bset![];
    let mut block_heights = bset![];
    let mut headers = bmap![];
    let mut upto_index = map! { true => UnhardenedIndex::zero(), false => UnhardenedIndex::zero() };
    for change in [false, true] {
        let mut offset = 0u16;
        let mut upto = UnhardenedIndex::zero();
        *upto_index.entry(change).or_default() = loop {
            let spk = wallet_settings
                .script_pubkeys(change, offset..=(offset + 19))
                .map_err(|err| electrum_client::Error::Message(err.to_string()))?;
            let batch =
                client.batch_script_get_history(spk.values().map(PubkeyScript::as_inner))?;

            // Retrieve unknown headers
            let heights = batch
                .iter()
                .flatten()
                .map(|res| res.height as u32)
                .collect::<BTreeSet<_>>();
            let diff = heights
                .difference(&block_heights)
                .copied()
                .collect::<Vec<_>>();
            let new_headers = client.batch_block_header(&diff)?;
            headers.extend(diff.iter().copied().zip(new_headers));
            block_heights.extend(diff);

            let batch = batch
                .into_iter()
                .zip(&spk)
                .map(|(history, (index, script))| {
                    let addr_src = AddressSource::with(script, *index, change, network);
                    let txids = history
                        .into_iter()
                        .map(|item| {
                            let mut meta = TxidMeta::from(item);
                            if let OnchainStatus::Blockchain(height) = meta.onchain.status {
                                meta.onchain.date_time = headers
                                    .get(&height)
                                    .and_then(|header| {
                                        NaiveDateTime::from_timestamp_opt(header.time as i64, 0)
                                    })
                                    .map(|naive| {
                                        DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc)
                                    });
                            }
                            meta
                        })
                        .collect::<BTreeSet<_>>();
                    (addr_src, txids)
                })
                .collect::<BTreeMap<_, _>>();

            let new_txids = batch
                .values()
                .flat_map(|item| item.iter().map(|meta| meta.onchain.txid))
                .collect::<Vec<_>>();
            if new_txids.is_empty() {
                break upto;
            } else {
                upto = batch
                    .keys()
                    .map(|item| item.index)
                    .max()
                    .unwrap_or_default();
            }
            txids.extend(new_txids);
            sender
                .send(Msg::TxidBatch(batch, offset))
                .expect("electrum watcher channel is broken");

            // Get transactions
            let utxos =
                client.batch_script_list_unspent(spk.values().map(PubkeyScript::as_inner))?;

            // Retrieve unknown headers
            let heights = utxos
                .iter()
                .flatten()
                .map(|res| res.height as u32)
                .collect::<BTreeSet<_>>();
            let diff = heights
                .difference(&block_heights)
                .copied()
                .collect::<Vec<_>>();
            let new_headers = client.batch_block_header(&diff)?;
            headers.extend(diff.iter().copied().zip(new_headers));
            block_heights.extend(diff);

            // Construct UTXO information
            let utxos = utxos
                .into_iter()
                .zip(spk)
                .flat_map(|(utxo, (index, script))| {
                    utxo.into_iter()
                        .map(move |res| {
                            UtxoTxid::with(
                                res,
                                AddressSource::with(&script, index, change, network),
                            )
                        })
                        .map(|mut utxo| {
                            if let OnchainStatus::Blockchain(height) = utxo.onchain.status {
                                utxo.onchain.date_time = headers
                                    .get(&height)
                                    .and_then(|header| {
                                        NaiveDateTime::from_timestamp_opt(header.time as i64, 0)
                                    })
                                    .map(|naive| {
                                        DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc)
                                    });
                            }
                            utxo
                        })
                })
                .collect::<BTreeSet<_>>();
            txids.extend(utxos.iter().map(|item| item.onchain.txid));
            sender
                .send(Msg::UtxoBatch(utxos, offset))
                .expect("electrum watcher channel is broken");

            offset += 20;
        };
    }

    let txids = txids.into_iter().collect::<Vec<_>>();
    for (no, chunk) in txids.chunks(20).enumerate() {
        let tx_list = client.batch_transaction_get(chunk)?;
        let progress = (no + 1) as f32 / txids.len() as f32 / 20.0;
        sender
            .send(Msg::TxBatch(tx_list, progress))
            .expect("electrum watcher channel is broken");
    }

    sender
        .send(Msg::Complete)
        .expect("electrum watcher channel is broken");

    Ok(())
}
