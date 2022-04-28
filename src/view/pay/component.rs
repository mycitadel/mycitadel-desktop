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

use ::wallet::descriptors::InputDescriptor;
use ::wallet::locks::{LockTime, SeqNo};
use ::wallet::psbt;
use ::wallet::psbt::{Construct, Psbt};
use ::wallet::scripts::PubkeyScript;
use bitcoin::blockdata::constants::WITNESS_SCALE_FACTOR;
use bitcoin::policy::DUST_RELAY_TX_FEE;
use bitcoin::secp256k1::SECP256K1;
use bitcoin::util::address;
use bitcoin::{EcdsaSighashType, Transaction, TxIn, TxOut};
use gladis::Gladis;
use gtk::prelude::*;
use gtk::{Dialog, ResponseType};
use miniscript::DescriptorTrait;
use relm::{Relm, StreamHandle, Update, Widget};

use super::{Msg, ViewModel, Widgets};
use crate::model::Wallet;
use crate::view::pay::beneficiary_row::{AmountError, Beneficiary};
use crate::view::wallet;

#[derive(Debug, Display, From, Error)]
#[display(doc_comments)]
pub enum Error {
    /// available wallet funds are insufficient to cover the transaction
    InsufficientFunds,

    /// one or more of beneficiaries has incorrect address (please see exclamation marks next to the addresses).
    #[from(address::Error)]
    Address,

    /// one or more of payment amounts are invalid (please see exclamation marks next to the addresses).
    #[from(AmountError)]
    Amount,

    /// internal error (wallet descriptor inconsistency)
    #[from]
    Miniscript(miniscript::Error),

    /// internal error (PSBT constructor inconsistency)
    #[from]
    PsbtConstruct(psbt::construct::Error),
}

pub struct Component {
    model: ViewModel,
    widgets: Widgets,
    wallet_stream: Option<StreamHandle<wallet::Msg>>,
}

impl Component {
    pub fn compose_psbt(&mut self) -> Result<Psbt, Error> {
        let wallet = self.model.as_wallet();

        let output_count = self.model.beneficiaries().n_items();
        let mut txouts = Vec::with_capacity(output_count as usize);
        let mut output_value = 0u64;
        for no in 0..output_count {
            let beneficiary = self
                .model
                .beneficiaries()
                .item(no)
                .expect("BeneficiaryModel is broken")
                .downcast::<Beneficiary>()
                .expect("BeneficiaryModel is broken");
            let script_pubkey = beneficiary.address()?.script_pubkey();
            let value = beneficiary.amount_sats()?;
            output_value += value;
            txouts.push(TxOut {
                script_pubkey,
                value,
            });
        }

        // TODO: Support constructing PSBTs from multiple descriptors (at descriptor-wallet lib)
        let (descriptor, _) = self.model.as_settings().descriptors_all()?;
        let lock_time = LockTime::since_now();
        let change_index = wallet.next_change_index();

        let fee_rate = self.model.fee_rate();
        let mut fee = DUST_RELAY_TX_FEE;
        let mut next_fee = fee;
        let mut prevouts = bset! {};
        let satisfaciton_weights = descriptor.max_satisfaction_weight()? as f32;
        // TODO: Test that his fee selection algorithm has deterministic end
        while fee <= DUST_RELAY_TX_FEE && fee != next_fee {
            fee = next_fee;
            prevouts = wallet
                .coinselect(output_value + fee as u64)
                .ok_or(Error::InsufficientFunds)?
                .0;
            let txins = prevouts
                .iter()
                .map(|p| TxIn {
                    previous_output: p.outpoint,
                    script_sig: none!(),
                    sequence: 0, // TODO: Support spending from CSV outputs
                    witness: none!(),
                })
                .collect::<Vec<_>>();

            let tx = Transaction {
                version: 1,
                lock_time: lock_time.as_u32(),
                input: txins,
                output: txouts.clone(),
            };
            let vsize = tx.vsize() as f32 + satisfaciton_weights / WITNESS_SCALE_FACTOR as f32;
            next_fee = (fee_rate * vsize).ceil() as u32;
        }

        let inputs = prevouts
            .into_iter()
            .map(|prevout| InputDescriptor {
                outpoint: prevout.outpoint,
                terminal: prevout.terminal(),
                seq_no: SeqNo::default(), // TODO: Support spending from CSV outputs
                tweak: None,
                sighash_type: EcdsaSighashType::All, // TODO: Support more sighashes in the UI
            })
            .collect::<Vec<_>>();
        let outputs = txouts
            .into_iter()
            .map(|txout| (PubkeyScript::from(txout.script_pubkey), txout.value))
            .collect::<Vec<_>>();

        let psbt = Psbt::construct(
            &SECP256K1,
            &descriptor,
            lock_time,
            &inputs,
            &outputs,
            change_index,
            fee as u64,
            wallet,
        )?;

        // TODO: Update latest change index in wallet settings and save the wallet by sending message to the wallet

        Ok(psbt)
    }
}

impl Update for Component {
    // Specify the model used for this widget.
    type Model = ViewModel;
    // Specify the model parameter used to init the model.
    type ModelParam = Wallet;
    // Specify the type of the messages sent to the update function.
    type Msg = Msg;

    fn model(_relm: &Relm<Self>, wallet: Self::ModelParam) -> Self::Model {
        ViewModel::with(wallet)
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::Show => {
                self.model.beneficiaries_mut().clear();
                self.model.beneficiaries_mut().append(&Beneficiary::new());
                self.widgets.init_ui(&self.model);
                self.widgets.show();
            }
            Msg::BeneficiaryAdd => {
                self.model.beneficiaries_mut().append(&Beneficiary::new());
            }
            Msg::BeneficiaryRemove => {
                self.widgets.selected_beneficiary_index().map(|index| {
                    self.model.beneficiaries_mut().remove(index);
                });
            }
            Msg::SelectBeneficiary(index) => self.widgets.select_beneficiary(index),
            Msg::BeneficiaryEdit(index) => {
                self.widgets.select_beneficiary(index);
                /* Check correctness of the model data */
            }
            Msg::FeeChange => { /* Update fee and total tx amount */ }
            Msg::FeeSetBlocks(_) => { /* Update fee and total tx amount */ }
            Msg::Response(ResponseType::Ok) => {
                self.widgets.hide();
            }
            Msg::Response(ResponseType::Cancel) => {
                self.widgets.hide();
            }
            Msg::Response(_) => {}
            Msg::SetWallet(stream) => {
                self.wallet_stream = Some(stream);
            }
        }
    }
}

impl Widget for Component {
    // Specify the type of the root widget.
    type Root = Dialog;

    // Return the root widget.
    fn root(&self) -> Self::Root {
        self.widgets.to_root()
    }

    fn view(relm: &Relm<Self>, model: Self::Model) -> Self {
        let glade_src = include_str!("pay.glade");
        let widgets = Widgets::from_string(glade_src).expect("glade file broken");

        widgets.connect(relm);
        widgets.bind_beneficiary_model(relm, &model);
        widgets.init_ui(&model);

        Component {
            model,
            widgets,
            wallet_stream: None,
        }
    }
}
