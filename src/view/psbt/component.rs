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

use std::{fs, io};

use bitcoin::consensus::Encodable;
use bitcoin::psbt::PartiallySignedTransaction;
use bitcoin::secp256k1::SECP256K1;
use gladis::Gladis;
use gtk::ApplicationWindow;
use miniscript::psbt::PsbtExt;
use relm::{Relm, StreamHandle, Update, Widget};

use super::{ModelParam, Msg, ViewModel, Widgets};
use crate::view::{error_dlg, file_save_dlg, launch};

pub struct Component {
    model: ViewModel,
    widgets: Widgets,
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
            // TODO: Publish transaction
        }
    }

    pub fn save(&mut self) -> Result<bool, io::Error> {
        let psbt = PartiallySignedTransaction::from(self.model.psbt().clone());
        let path = match self.model.path() {
            Some(path) => path,
            None => {
                let path = match file_save_dlg(
                    self.widgets.as_root(),
                    "Save transaction",
                    "Partially signed bitcoin transaction",
                    "*.psbt",
                ) {
                    None => return Ok(false),
                    Some(path) => path,
                };
                self.model.set_path(path);
                self.model.path().as_ref().expect("path was just set")
            }
        };
        let file = fs::File::create(path)?;
        psbt.consensus_encode(file)?;
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
            Msg::Publish => self.publish(),
            Msg::Sign(_output_no) => {
                // TODO: Implement signing
                self.finalize();
            }
            Msg::Launcher(msg) => {
                self.launcher_stream.as_ref().map(|stream| stream.emit(msg));
            }
            Msg::Close => self.close(),
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

        widgets.connect(relm);
        widgets.bind_signing_model(relm, model.signing());
        widgets.update_ui(&model);
        widgets.show();

        Component {
            model,
            widgets,
            launcher_stream: None,
        }
    }
}
