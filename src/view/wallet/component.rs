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
use std::ffi::OsStr;
use std::path::PathBuf;

use ::wallet::descriptors::InputDescriptor;
use ::wallet::psbt::Psbt;
use bitcoin::blockdata::constants::WITNESS_SCALE_FACTOR;
use bitcoin::policy::DUST_RELAY_TX_FEE;
use bitcoin::{EcdsaSighashType, Sequence, Transaction, TxIn, TxOut};
use bitcoin_blockchain::locks::{LockTime, SeqNo};
use bitcoin_scripts::PubkeyScript;
use bpro::psbt::McKeys;
use bpro::{AddressSource, Prevout, TxidMeta, Wallet};
use gladis::Gladis;
use gtk::prelude::*;
use gtk::{ApplicationWindow, ResponseType};
use relm::{init, Channel, Relm, StreamHandle, Update, Widget};
use rgb::BlockchainResolver;
use wallet::hd::{SegmentIndexes, UnhardenedIndex};
use wallet::lex_order::lex_order::LexOrder;

use super::pay::beneficiary_row::Beneficiary;
use super::pay::FeeRate;
use super::{pay, ElectrumState, Msg, ViewModel, Widgets};
use crate::view::wallet::payto;
use crate::view::{error_dlg, launch, settings, NotificationBoxExt};
use crate::worker::{electrum, exchange, ElectrumWorker, ExchangeWorker};

pub struct Component {
    model: ViewModel,
    widgets: Widgets,
    pay_widgets: pay::Widgets,
    payto_widgets: payto::Widgets,

    exchange_channel: Channel<exchange::Msg>,
    exchange_worker: ExchangeWorker,
    electrum_channel: Channel<electrum::Msg>,
    electrum_worker: ElectrumWorker,
    tx_buffer: Vec<Transaction>,
    addr_buffer: BTreeMap<AddressSource, BTreeSet<TxidMeta>>,

    settings: relm::Component<settings::Component>,
    launcher_stream: Option<StreamHandle<launch::Msg>>,

    resolver: BlockchainResolver,
}

impl Component {
    fn close(&self) {
        self.widgets.close();
        self.launcher_stream
            .as_ref()
            .map(|stream| stream.emit(launch::Msg::WalletClosed));
    }

    fn save(&mut self) {
        match self.model.save() {
            Ok(_) => {}
            Err(err) => error_dlg(
                self.widgets.as_root(),
                "Error saving wallet",
                "It was impossible to save changes to the wallet settings due to an error",
                Some(&err.to_string()),
            ),
        }
    }

    pub fn compose_psbt(&mut self) -> Result<(Psbt, UnhardenedIndex, u64, u32, f32), pay::Error> {
        let wallet = self.model.wallet();

        let output_count = self.model.beneficiaries().n_items();
        let mut txouts = Vec::with_capacity(output_count as usize);
        let mut output_value = 0u64;
        let mut output_max = None;
        for no in 0..output_count {
            let beneficiary = self
                .model
                .beneficiaries()
                .item(no)
                .expect("BeneficiaryModel is broken")
                .downcast::<Beneficiary>()
                .expect("BeneficiaryModel is broken");
            let script_pubkey = beneficiary.address()?.script_pubkey();
            let value = if beneficiary.is_amount_max() {
                match output_max {
                    None => output_max = Some(no),
                    Some(_) => return Err(pay::Error::MultipleMaxOutputs),
                }
                0
            } else {
                let value = beneficiary.amount_sats();
                if value == 0 {
                    return Err(pay::Error::Amount);
                }
                value
            };
            output_value += value;
            txouts.push(TxOut {
                script_pubkey,
                value,
            });
        }

        // TODO: Support constructing PSBTs from multiple descriptors (at descriptor-wallet lib)
        let (descriptor, _) = self.model.as_settings().descriptors_all()?;
        let lock_time = LockTime::from_height(734438).expect("hardcoded height");
        let change_index = wallet.next_change_index();

        let fee_rate = self.model.fee_rate();
        let mut fee = 0;
        let mut next_fee = DUST_RELAY_TX_FEE;
        let mut prevouts = bset! {};
        let satisfaciton_weights = descriptor.max_satisfaction_weight()? as f32;
        let mut cycle_lim = 0usize;
        let mut vsize = 0.0f32;
        while fee <= DUST_RELAY_TX_FEE && fee != next_fee {
            fee = next_fee;
            if output_max.is_some() {
                prevouts = wallet
                    .utxos()
                    .iter()
                    .map(Prevout::from)
                    .collect::<BTreeSet<_>>();
            } else {
                prevouts = wallet
                    .coinselect(output_value + fee as u64)
                    .ok_or(pay::Error::InsufficientFunds)?
                    .0;
            }
            let txins = prevouts
                .iter()
                .map(|p| TxIn {
                    previous_output: p.outpoint,
                    script_sig: none!(),
                    sequence: Sequence(0), // TODO: Support spending from CSV outputs
                    witness: none!(),
                })
                .collect::<Vec<_>>();

            let tx = Transaction {
                version: 1,
                lock_time: bitcoin::LockTime::from_consensus(lock_time.into_consensus()).into(),
                input: txins,
                output: txouts.clone(),
            };
            vsize = tx.vsize() as f32 + satisfaciton_weights / WITNESS_SCALE_FACTOR as f32;
            next_fee = (fee_rate * vsize).ceil() as u32;
            cycle_lim += 1;
            if cycle_lim > 6 {
                return Err(pay::Error::FeeFailure);
            }
        }

        let input_value = prevouts.iter().map(|p| p.amount).sum::<u64>();
        if let Some(vout) = output_max {
            let max_value = input_value - output_value - fee as u64;
            txouts[vout as usize].value = max_value;
            output_value += max_value;
        }
        if output_value == 0 {
            return Err(pay::Error::NoBeneficiaries);
        }

        let inputs = prevouts
            .into_iter()
            .map(|prevout| InputDescriptor {
                outpoint: prevout.outpoint,
                terminal: prevout.terminal(),
                seq_no: SeqNo::rbf(), // TODO: Support spending from CSV outputs
                tweak: None,
                sighash_type: EcdsaSighashType::All, // TODO: Support more sighashes in the UI
            })
            .collect::<Vec<_>>();
        let outputs = txouts
            .into_iter()
            .map(|txout| (PubkeyScript::from(txout.script_pubkey), txout.value))
            .collect::<Vec<_>>();

        let mut psbt = Psbt::construct(
            &descriptor,
            &inputs,
            &outputs,
            change_index,
            fee as u64,
            wallet,
        )?;
        psbt.fallback_locktime = Some(LockTime::from_consensus(lock_time.into_consensus()));
        psbt.lex_order();

        for signer in self.model.as_settings().signers() {
            psbt.set_signer_name(signer.master_fp, &signer.name);
        }

        Ok((psbt, change_index, output_value, fee, vsize))
    }

    pub fn sync_pay(&mut self) -> Option<(Psbt, UnhardenedIndex)> {
        match self.compose_psbt() {
            Ok((psbt, change_index, output_value, fee, vsize)) => {
                self.pay_widgets.hide_message();
                self.pay_widgets.update_info(
                    self.model.fee_rate(),
                    self.model.wallet().ephemerals().fees,
                    Some((output_value, fee, vsize)),
                );
                Some((psbt, change_index))
            }
            Err(err) => {
                self.pay_widgets.show_error(&err.to_string());
                None
            }
        }
    }

    fn handle_exchange(&mut self, msg: exchange::Msg) {
        match msg {
            exchange::Msg::Rate(fiat, exchange, rate) => {
                self.model.fiat = fiat;
                self.model.exchange = exchange;
                self.model.exchange_rate = rate;
                self.widgets.update_exchange_rate(
                    fiat,
                    exchange,
                    rate,
                    self.model.wallet().state(),
                );
            }
            exchange::Msg::Error(err) => {
                self.widgets.update_exchange_error(err);
            }
            exchange::Msg::ChannelDisconnected => {
                panic!("Broken exchange thread")
            }
        }
    }

    fn handle_electrum(&mut self, msg: electrum::Msg) {
        match msg {
            electrum::Msg::Connecting => {
                self.widgets
                    .update_electrum_state(ElectrumState::Connecting);
            }
            electrum::Msg::Connected => {
                self.widgets
                    .update_electrum_state(ElectrumState::QueryingBlockchainState);
            }
            electrum::Msg::LastBlock(block_info) => {
                self.widgets
                    .update_electrum_state(ElectrumState::RetrievingFees);
                self.model.wallet_mut().update_last_block(&block_info);
                self.widgets.update_last_block(&block_info);
            }
            electrum::Msg::LastBlockUpdate(block_info) => {
                self.model.wallet_mut().update_last_block(&block_info);
                self.widgets.update_last_block(&block_info);
            }
            electrum::Msg::FeeEstimate(f0, f1, f2) => {
                self.widgets
                    .update_electrum_state(ElectrumState::RetrievingHistory(0));
                let wallet = self.model.wallet_mut();
                wallet.update_fees(f0, f1, f2);
                wallet.clear_utxos();
            }
            electrum::Msg::TxidBatch(batch, no) => {
                self.widgets
                    .update_electrum_state(ElectrumState::RetrievingHistory(no as usize * 2));
                self.addr_buffer.extend(batch);
            }
            electrum::Msg::UtxoBatch(batch, no) => {
                self.widgets
                    .update_electrum_state(ElectrumState::RetrievingHistory(no as usize * 2 + 1));
                let wallet = self.model.wallet_mut();
                wallet.update_utxos(batch);
                self.widgets.update_outpoints(&mut self.model);
                self.widgets.update_balance(&mut self.model);
            }
            electrum::Msg::TxBatch(batch, progress) => {
                self.widgets
                    .update_electrum_state(ElectrumState::RetrievingTransactions(progress));
                self.tx_buffer.extend(batch);
            }
            electrum::Msg::Complete => {
                self.model
                    .wallet_mut()
                    .update_complete(&self.addr_buffer, &self.tx_buffer);
                self.addr_buffer.clear();
                self.tx_buffer.clear();
                self.save();

                self.widgets.update_balance(&mut self.model);
                let wallet = self.model.wallet_mut();
                self.widgets.update_history(&wallet.history());
                self.widgets.update_addresses(&wallet.address_info(true));
                self.widgets.update_electrum_state(ElectrumState::Complete(
                    self.model.as_settings().electrum().sec,
                ));
            }
            electrum::Msg::Error(err) => {
                self.widgets
                    .update_electrum_state(ElectrumState::Error(err.to_string()));
            }
            electrum::Msg::ChannelDisconnected => {
                panic!("Broken electrum thread")
            }
        }
    }
}

impl Update for Component {
    // Specify the model used for this widget.
    type Model = ViewModel;
    // Specify the model parameter used to init the model.
    type ModelParam = (Wallet, PathBuf);
    // Specify the type of the messages sent to the update function.
    type Msg = Msg;

    fn model(_relm: &Relm<Self>, param: Self::ModelParam) -> Self::Model {
        ViewModel::with(param.0, param.1)
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::New => {
                self.launcher_stream
                    .as_ref()
                    .map(|stream| stream.emit(launch::Msg::ShowPage(launch::Page::Template)));
            }
            Msg::Open => {
                self.launcher_stream
                    .as_ref()
                    .map(|stream| stream.emit(launch::Msg::Wallet));
            }
            Msg::Duplicate => {
                let settings = self.model.to_settings();
                let path: PathBuf = self.model.path().clone();
                let new_path = format!(
                    "{}-copy.mcw",
                    path.file_stem().and_then(OsStr::to_str).unwrap_or_default()
                );
                self.launcher_stream
                    .as_ref()
                    .map(|stream| stream.emit(launch::Msg::Duplicate(settings, new_path)));
            }
            Msg::Import => {
                self.launcher_stream
                    .as_ref()
                    .map(|stream| stream.emit(launch::Msg::ShowPage(launch::Page::Import)));
            }
            Msg::ImportRgbContract(text) => {
                // TODO: Report error properly
                if let Err(err) = self.model.import_rgb_contract(text, &mut self.resolver) {
                    eprintln!("Error: {err}");
                } else {
                    self.save();
                }
            }
            Msg::ChangeAsset(index) => {
                let success = self.model.change_asset(index);
                debug_assert!(success, "invalid index selection");
                self.widgets.update_ui(&mut self.model);
            }
            Msg::Close => self.close(),
            Msg::About => {
                self.launcher_stream
                    .as_ref()
                    .map(|stream| stream.emit(launch::Msg::About));
            }
            Msg::Pay(msg) => self.update_pay(msg),
            Msg::PayTo(msg) => self.update_payto(msg),
            Msg::Settings => self.settings.emit(settings::Msg::View(
                self.model.to_settings(),
                self.model.path().clone(),
            )),
            Msg::Fiat(fiat) => {
                self.model.fiat = fiat;
                self.widgets.update_fiat(fiat);
                self.exchange_worker.set_fiat(fiat);
            }
            Msg::ExchangeRefresh(msg) => {
                self.handle_exchange(msg);
            }
            Msg::Refresh => {
                self.electrum_worker.sync();
            }
            Msg::Update(signers, descriptor_classes, electrum) => {
                match self
                    .model
                    .update_descriptor(signers, descriptor_classes, electrum)
                {
                    Err(err) => error_dlg(
                        self.widgets.as_root(),
                        "Internal error",
                        "Please report the following information to the developer",
                        Some(&err.to_string()),
                    ),
                    Ok(new_server) => {
                        new_server.map(|electrum| self.widgets.update_electrum_server(&electrum));
                        self.widgets.show();
                        self.settings
                            .emit(settings::Msg::Response(ResponseType::Cancel));
                    }
                }
                self.save();
            }
            Msg::InvoiceAmountToggle(set) => {
                self.model.as_invoice_mut().amount = match set {
                    true => Some(0),
                    false => None,
                };
                self.widgets.update_invoice(&self.model);
            }
            Msg::InvoiceIndexToggle(set) => {
                self.model.as_invoice_mut().index = match set {
                    true => Some(self.model.wallet().next_default_index()),
                    false => None,
                };
                self.widgets.update_invoice(&self.model);
            }
            Msg::InvoiceAmount(btc) => {
                let sats = (btc * 100_000_000.0).ceil() as u64;
                self.model
                    .as_invoice_mut()
                    .amount
                    .as_mut()
                    .map(|a| *a = sats);
                self.widgets.update_invoice(&self.model);
            }
            Msg::InvoiceIndex(index) => {
                let index = UnhardenedIndex::from_index(index)
                    .expect("unhardened index adjustment requirements broken");
                self.model
                    .as_invoice_mut()
                    .index
                    .as_mut()
                    .map(|i| *i = index);
                self.widgets.update_invoice(&self.model);
            }
            Msg::Launch(msg) => {
                self.launcher_stream.as_ref().map(|stream| stream.emit(msg));
            }
            Msg::RegisterLauncher(stream) => {
                self.launcher_stream = Some(stream);
            }
            Msg::ElectrumWatch(msg) => self.handle_electrum(msg),
        }
    }
}

impl Component {
    fn update_pay(&mut self, event: pay::Msg) {
        match event {
            pay::Msg::Show => {
                self.model.beneficiaries_mut().clear();
                self.model
                    .beneficiaries_mut()
                    .append(&Beneficiary::default());
                self.model
                    .set_fee_rate(self.model.wallet().ephemerals().fees.0);
                self.pay_widgets.init_ui(&self.model);
                self.pay_widgets.show();
            }
            pay::Msg::Response(ResponseType::Ok) => {
                let (psbt, change_index) = match self.sync_pay() {
                    Some(data) => data,
                    None => return,
                };
                self.pay_widgets.hide();
                self.launcher_stream.as_ref().map(|stream| {
                    stream.emit(launch::Msg::CreatePsbt(
                        psbt,
                        self.model.as_settings().network(),
                    ))
                });
                // Update latest change index in wallet settings by sending message to the wallet
                // component
                if self
                    .model
                    .wallet_mut()
                    .update_next_change_index(change_index)
                {
                    self.save();
                }
            }
            pay::Msg::Response(ResponseType::Cancel) => {
                self.pay_widgets.hide();
            }
            pay::Msg::Response(_) => {}
            _ => {} // Changes which update wallet tx
        }

        match event {
            pay::Msg::BeneficiaryAdd => {
                self.model
                    .beneficiaries_mut()
                    .append(&Beneficiary::default());
            }
            pay::Msg::BeneficiaryRemove => {
                self.pay_widgets.selected_beneficiary_index().map(|index| {
                    self.model.beneficiaries_mut().remove(index);
                });
            }
            pay::Msg::SelectBeneficiary(index) => self.pay_widgets.select_beneficiary(index),
            pay::Msg::BeneficiaryEdit(index) => {
                self.pay_widgets.select_beneficiary(index);
            }
            pay::Msg::FeeSet => {
                let fee_rate = self.pay_widgets.fee_rate();
                if fee_rate as f32 == self.model.fee_rate() {
                    return;
                }
                self.model.set_fee_rate(fee_rate as f32);
            }
            pay::Msg::FeeSetBlocks(ty) => {
                let fees = self.model.wallet().ephemerals().fees;
                let fee_rate = match ty {
                    FeeRate::OneBlock => fees.0,
                    FeeRate::TwoBlocks => fees.1,
                    FeeRate::ThreeBlocks => fees.2,
                    FeeRate::Unknown => unreachable!(),
                };
                if fee_rate == self.model.fee_rate() {
                    return;
                }
                self.model.set_fee_rate(fee_rate);
            }
            _ => return, // Changes which do not update wallet tx
        }

        self.sync_pay();
    }
}

impl Component {
    fn update_payto(&mut self, event: payto::Msg) {
        match event {
            payto::Msg::Show => {
                self.payto_widgets.init_ui(&self.model);
                self.payto_widgets.show();
            }
            payto::Msg::Response(ResponseType::Ok) => {
                let (psbt, change_index) = match self.sync_pay() {
                    Some(data) => data,
                    None => return,
                };
                self.payto_widgets.hide();
                self.launcher_stream.as_ref().map(|stream| {
                    stream.emit(launch::Msg::CreatePsbt(
                        psbt,
                        self.model.as_settings().network(),
                    ))
                });
                // Update latest change index in wallet settings by sending message to the wallet
                // component
                if self
                    .model
                    .wallet_mut()
                    .update_next_change_index(change_index)
                {
                    self.save();
                }
            }
            payto::Msg::Response(ResponseType::Cancel) => {
                self.payto_widgets.hide();
            }
            payto::Msg::Response(ResponseType::Other(1000)) => {
                self.payto_widgets.hide();
                self.update_pay(pay::Msg::Show);
            }
            _ => {} // Changes which update wallet tx
        }
    }
}

impl Widget for Component {
    // Specify the type of the root widget.
    type Root = ApplicationWindow;

    // Return the root widget.
    fn root(&self) -> Self::Root { self.widgets.to_root() }

    fn view(relm: &Relm<Self>, mut model: Self::Model) -> Self {
        let glade_src = include_str!("wallet.glade");
        let mut widgets = Widgets::from_string(glade_src).expect("glade file broken");

        let settings = init::<settings::Component>(()).expect("error in settings component");
        settings.emit(settings::Msg::SetWallet(relm.stream().clone()));

        let stream = relm.stream().clone();
        let (electrum_channel, sender) =
            Channel::new(move |msg| stream.emit(Msg::ElectrumWatch(msg)));
        let electrum_worker = ElectrumWorker::with(sender, model.wallet().to_settings(), 60)
            .expect("unable to instantiate electrum thread");

        let stream = relm.stream().clone();
        let (exchange_channel, sender) =
            Channel::new(move |msg| stream.emit(Msg::ExchangeRefresh(msg)));
        let exchange_worker = ExchangeWorker::with(sender, model.exchange(), model.fiat(), 600)
            .expect("unable to instantiate exchange thread");

        widgets.connect(relm);
        widgets.init_ui(&model);
        widgets.update_ui(&mut model);
        widgets.show();

        let glade_src = include_str!("pay/pay.glade");
        let pay_widgets = pay::Widgets::from_string(glade_src).expect("glade file broken");
        pay_widgets.connect(relm);
        pay_widgets.bind_beneficiary_model(relm, &model);
        pay_widgets.init_ui(&model);

        let glade_src = include_str!("payto/payto.glade");
        let payto_widgets = payto::Widgets::from_string(glade_src).expect("glade file broken");
        payto_widgets.connect(relm);
        payto_widgets.init_ui(&model);

        electrum_worker.sync();

        // TODO: remove the panic and allow user to fix resolver settings
        let resolver =
            BlockchainResolver::with(&model.wallet().as_settings().electrum().to_string())
                .expect("invalid electrum server");

        Component {
            model,
            widgets,
            pay_widgets,
            payto_widgets,
            settings,

            exchange_channel,
            exchange_worker,
            electrum_channel,
            electrum_worker,
            tx_buffer: empty!(),
            addr_buffer: empty!(),

            launcher_stream: None,
            resolver,
        }
    }
}
