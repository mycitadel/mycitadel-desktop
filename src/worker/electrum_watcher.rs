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
use std::io;
use std::sync::mpsc;
use std::thread::JoinHandle;

use amplify::Wrapper;
use bitcoin::{Transaction, Txid};
use electrum_client::{Client as ElectrumClient, ElectrumApi, HeaderNotification};
use relm::Sender;
use wallet::address::address::AddressCompat;
use wallet::hd::{SegmentIndexes, UnhardenedIndex};
use wallet::scripts::PubkeyScript;

use crate::model::{ElectrumServer, WalletSettings};

pub enum Cmd {
    Sync,
    Pull,
    Update(ElectrumServer),
}

pub enum ElectrumMsg {
    Connecting,
    Connected,
    Complete,
    LastBlock(HeaderNotification),
    LastBlockUpdate(HeaderNotification),
    FeeEstimate(f64, f64, f64),
    HistoryBatch(Vec<HistoryTxid>, u16),
    UtxoBatch(Vec<UtxoTxid>, u16),
    TxBatch(BTreeMap<Txid, Transaction>, f32),
    ChannelDisconnected,
    Error(electrum_client::Error),
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
#[derive(StrictEncode, StrictDecode)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate")
)]
pub struct HistoryTxid {
    pub txid: Txid,
    pub height: i32,
    #[cfg_attr(feature = "serde", serde(with = "serde_with::rust::display_fromstr"))]
    pub address: AddressCompat,
    pub index: UnhardenedIndex,
    pub change: bool,
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
#[derive(StrictEncode, StrictDecode)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate")
)]
pub struct UtxoTxid {
    pub txid: Txid,
    pub height: u32,
    pub pos: u32,
    pub value: u64,
    #[cfg_attr(feature = "serde", serde(with = "serde_with::rust::display_fromstr"))]
    pub address: AddressCompat,
    pub index: UnhardenedIndex,
    pub change: bool,
}

pub struct ElectrumWatcher {
    handle: JoinHandle<()>,
    tx: mpsc::Sender<Cmd>,
}

impl ElectrumWatcher {
    pub fn with(
        sender: Sender<ElectrumMsg>,
        mut wallet_settings: WalletSettings,
    ) -> Result<Self, io::Error> {
        let (tx, rx) = mpsc::channel::<Cmd>();
        let handle = std::thread::Builder::new()
            .name(s!("electrum"))
            .spawn(move || {
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
                                    .send(ElectrumMsg::LastBlockUpdate(last_block))
                                    .expect("electrum watcher channel is broken");
                            }
                        }),
                        (None, Ok(_)) => {
                            /* Can't handle since no client avaliable */
                            Ok(())
                        }
                        (_, Err(_)) => {
                            sender
                                .send(ElectrumMsg::ChannelDisconnected)
                                .expect("electrum channel is broken");
                            Ok(())
                        }
                    }
                    .map_err(|err| {
                        sender
                            .send(ElectrumMsg::Error(err))
                            .expect("electrum channel is broken");
                    });
                }
            })?;
        Ok(ElectrumWatcher { tx, handle })
    }

    pub fn sync(&self) {
        self.cmd(Cmd::Sync)
    }

    pub fn pull(&self) {
        self.cmd(Cmd::Pull)
    }

    pub fn update(&self, server: ElectrumServer) {
        self.cmd(Cmd::Update(server))
    }

    fn cmd(&self, cmd: Cmd) {
        self.tx.send(cmd).expect("Electrum thread is dead")
    }
}

pub fn electrum_init(
    electrum: &ElectrumServer,
    sender: &Sender<ElectrumMsg>,
) -> Option<ElectrumClient> {
    let config = electrum_client::ConfigBuilder::new()
        .timeout(Some(5))
        .expect("we do not use socks here")
        .build();
    ElectrumClient::from_config(&electrum.to_string(), config)
        .map_err(|err| {
            sender
                .send(ElectrumMsg::Error(err))
                .expect("electrum channel is broken");
        })
        .ok()
}

pub fn electrum_sync(
    client: &ElectrumClient,
    wallet_settings: &WalletSettings,
    sender: &Sender<ElectrumMsg>,
) -> Result<(), electrum_client::Error> {
    sender
        .send(ElectrumMsg::Connecting)
        .expect("electrum watcher channel is broken");

    sender
        .send(ElectrumMsg::Connected)
        .expect("electrum watcher channel is broken");

    let last_block = client.block_headers_subscribe()?;
    sender
        .send(ElectrumMsg::LastBlock(last_block))
        .expect("electrum watcher channel is broken");

    let fee = client.batch_estimate_fee([1, 2, 3])?;
    sender
        .send(ElectrumMsg::FeeEstimate(fee[0], fee[1], fee[2]))
        .expect("electrum watcher channel is broken");

    let network = bitcoin::Network::from(wallet_settings.network());

    let mut txids = bset![];
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
                .zip(&spk)
                .flat_map(|(history, (index, script))| {
                    history.into_iter().map(move |res| HistoryTxid {
                        txid: res.tx_hash,
                        height: res.height,
                        address: AddressCompat::from_script(&script.clone().into(), network)
                            .expect("broken descriptor"),
                        index: *index,
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
                .send(ElectrumMsg::HistoryBatch(history_batch, offset))
                .expect("electrum watcher channel is broken");

            let utxos: Vec<_> = client
                .batch_script_list_unspent(spk.values().map(PubkeyScript::as_inner))?
                .into_iter()
                .zip(spk)
                .flat_map(|(utxo, (index, script))| {
                    utxo.into_iter().map(move |res| UtxoTxid {
                        txid: res.tx_hash,
                        height: res.height as u32,
                        pos: res.tx_pos as u32,
                        value: res.value,
                        address: AddressCompat::from_script(&script.clone().into(), network)
                            .expect("broken descriptor"),
                        index,
                        change,
                    })
                })
                .collect();
            txids.extend(utxos.iter().map(|item| item.txid));
            sender
                .send(ElectrumMsg::UtxoBatch(utxos, offset))
                .expect("electrum watcher channel is broken");

            offset += 20;
        };
    }
    let txids = txids.into_iter().collect::<Vec<_>>();
    for (no, chunk) in txids.chunks(20).enumerate() {
        let txmap = chunk
            .iter()
            .copied()
            .zip(client.batch_transaction_get(chunk)?)
            .collect::<BTreeMap<_, _>>();
        sender
            .send(ElectrumMsg::TxBatch(
                txmap,
                (no + 1) as f32 / txids.len() as f32 / 20.0,
            ))
            .expect("electrum watcher channel is broken");
    }

    sender
        .send(ElectrumMsg::Complete)
        .expect("electrum watcher channel is broken");

    Ok(())
}
