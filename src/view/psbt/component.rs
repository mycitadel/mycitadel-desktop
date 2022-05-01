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

use std::str::FromStr;
use std::{fs, io, thread};

use bitcoin::consensus::Encodable;
use bitcoin::psbt::PartiallySignedTransaction;
use bitcoin::secp256k1::SECP256K1;
use electrum_client::ElectrumApi;
use gladis::Gladis;
use gtk::prelude::ListModelExt;
use gtk::{ApplicationWindow, MessageType};
use hwi::HWIDevice;
use miniscript::psbt::PsbtExt;
use relm::{Cast, Channel, Relm, Sender, StreamHandle, Update, Widget};

use super::sign_row::Signing;
use super::{ModelParam, Msg, SignMsg, ViewModel, Widgets};
use crate::view::psbt::PublishMsg;
use crate::view::{error_dlg, file_save_dlg, launch, msg_dlg};
use crate::worker::electrum::electrum_connect;

pub struct Component {
    model: ViewModel,
    widgets: Widgets,
    signer_sender: Sender<SignMsg>,
    publisher_sender: Sender<PublishMsg>,
    launcher_stream: Option<StreamHandle<launch::Msg>>,
}

impl Component {
    pub fn close(&self) {
        // TODO: Check modified status and ask to save the file
        self.widgets.close();
        self.launcher_stream
            .as_ref()
            .map(|stream| stream.emit(launch::Msg::PsbtClosed));
    }

    pub fn sign(&mut self, signer_index: u32) {
        let signer: Signing = self
            .model
            .signing()
            .item(signer_index)
            .expect("wrong signer no")
            .downcast()
            .expect("wrong signer");
        let name = signer.name();
        let master_fp = signer.master_fp();
        let device = HWIDevice {
            device_type: s!(""),
            model: s!(""),
            path: s!(""),
            needs_pin_sent: false,
            needs_passphrase_sent: false,
            fingerprint: master_fp,
        };

        self.widgets
            .show_sign(&format!("Signing with device {} [{}]", name, master_fp));

        let psbt = self.model.psbt().clone().into();
        let sender = self.signer_sender.clone();
        thread::spawn(move || {
            match device
                .sign_tx(&psbt, false)
                .map_err(|e| e.to_string())
                .and_then(|resp| {
                    PartiallySignedTransaction::from_str(&resp.psbt).map_err(|e| e.to_string())
                }) {
                Err(err) => sender.send(SignMsg::Failed(name, master_fp, err.to_string())),
                Ok(psbt) => sender.send(SignMsg::Signed(psbt.into())),
            }
            .expect("channel broken");
        });
    }

    pub fn finalize(&mut self) -> Result<(), Vec<miniscript::psbt::Error>> {
        let mut psbt = PartiallySignedTransaction::from(self.model.psbt().clone());
        self.model.clear_finalized_tx();
        psbt.finalize_mut(&SECP256K1)?;
        let tx = psbt.extract_tx();
        self.model.set_finalized_tx(tx);
        self.widgets.update_ui(&self.model);
        Ok(())
    }

    pub fn publish(&mut self) {
        if self.finalize().is_err() {
            return;
        }
        if let Some(tx) = self.model.finalized_tx() {
            self.widgets.publish_pending();

            let tx = tx.clone();
            let sender = self.publisher_sender.clone();
            thread::spawn(move || {
                let _ = match electrum_connect(&"ssl://blockstream.info:700")
                    .and_then(|client| client.transaction_broadcast(&tx))
                {
                    Err(err) => sender.send(PublishMsg::Declined(err.to_string())),
                    Ok(_txid) => sender.send(PublishMsg::Published),
                };
            });
        }
    }

    pub fn save(&mut self) -> Result<bool, io::Error> {
        let psbt = PartiallySignedTransaction::from(self.model.psbt().clone());
        let path = match file_save_dlg(
            self.widgets.as_root(),
            "Save transaction",
            "Partially signed bitcoin transaction",
            "*.psbt",
        ) {
            None => return Ok(false),
            Some(path) if path.extension().is_some() => path,
            Some(mut path) => {
                path.set_extension("psbt");
                path
            }
        };
        let file = fs::File::create(&path)?;
        psbt.consensus_encode(file)?;
        self.model.set_path(path);
        self.widgets.update_path(self.model.path().as_deref());
        Ok(true)
    }
}

impl Update for Component {
    // Specify the model used for this widget.
    type Model = ViewModel;
    // Specify the model parameter used to init the model.
    type ModelParam = ModelParam;
    // Specify the type of the messages sent to the update function.
    type Msg = Msg;

    fn model(_relm: &Relm<Self>, param: Self::ModelParam) -> Self::Model {
        let path = param.path();
        let network = param.network();
        let psbt = param.into_psbt();
        ViewModel::with(psbt, path, network)
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::Save => {
                if let Err(err) = self.save() {
                    error_dlg(
                        self.widgets.as_root(),
                        "Error",
                        "Unable to save PSBT file",
                        Some(&err.to_string()),
                    );
                }
            }
            Msg::Close => self.close(),

            Msg::Sign(signer_index) => self.sign(signer_index),
            Msg::Signed(psbt) => {
                self.widgets.hide_sign();
                self.model.replace_psbt(psbt);
                self.widgets.update_ui(&self.model);
                self.widgets.set_unsaved();
                let _ = self.finalize();
            }
            Msg::Failed(name, fp, err) => {
                self.widgets.hide_sign();
                error_dlg(
                    self.widgets.as_root(),
                    "Error",
                    &format!("Unable to sign with {} [{}]", name, fp),
                    Some(&err),
                );
            }

            Msg::Publish => self.publish(),
            Msg::Published => {
                msg_dlg(
                    self.widgets.as_root(),
                    MessageType::Info,
                    "Success",
                    "Transaction was successfully published",
                    None,
                );
                self.widgets.publish_restore(true);
            }
            Msg::Declined(err) => {
                error_dlg(
                    self.widgets.as_root(),
                    "Not published",
                    "Transaction was declined by the network",
                    Some(&err),
                );
                self.widgets.publish_restore(false);
            }

            Msg::Launch(msg) => {
                self.launcher_stream.as_ref().map(|stream| stream.emit(msg));
            }
            Msg::RegisterLauncher(stream) => {
                self.launcher_stream = Some(stream);
            }
        }
    }
}

impl Widget for Component {
    // Specify the type of the root widget.
    type Root = ApplicationWindow;

    // Return the root widget.
    fn root(&self) -> Self::Root {
        self.widgets.to_root()
    }

    fn view(relm: &Relm<Self>, model: Self::Model) -> Self {
        let glade_src = include_str!("psbt.glade");
        let widgets = Widgets::from_string(glade_src).expect("glade file broken");

        let stream = relm.stream().clone();
        let (_channel, signer_sender) = Channel::new(move |msg| match msg {
            SignMsg::Signed(psbt) => stream.emit(Msg::Signed(psbt)),
            SignMsg::Failed(name, fp, err) => stream.emit(Msg::Failed(name, fp, err)),
        });

        let stream = relm.stream().clone();
        let (_channel, publisher_sender) = Channel::new(move |msg| match msg {
            PublishMsg::Published => stream.emit(Msg::Published),
            PublishMsg::Declined(err) => stream.emit(Msg::Declined(err)),
        });

        widgets.init_ui();
        widgets.connect(relm);
        widgets.bind_signing_model(relm, model.signing());
        widgets.update_ui(&model);
        widgets.show();

        let mut component = Component {
            model,
            widgets,
            signer_sender,
            publisher_sender,
            launcher_stream: None,
        };
        let _ = component.finalize();

        component
    }
}
