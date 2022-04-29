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

use bitcoin::consensus::{Decodable, Encodable};
use bitcoin::psbt::PartiallySignedTransaction;
use gladis::Gladis;
use gtk::ApplicationWindow;
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

    fn model(relm: &Relm<Self>, param: Self::ModelParam) -> Self::Model {
        let path = param.path();
        let network = param.network();
        let psbt = match param {
            ModelParam::Open(ref path, _) => {
                let file = match fs::File::open(&path) {
                    Ok(file) => file,
                    Err(err) => {
                        relm.stream()
                            .emit(Msg::FileError(path.clone(), err.to_string()));
                        relm.stream().emit(Msg::Close);
                        return ViewModel::default();
                    }
                };
                match PartiallySignedTransaction::consensus_decode(&file) {
                    Ok(psbt) => psbt.into(),
                    Err(err) => {
                        relm.stream()
                            .emit(Msg::FileError(path.clone(), err.to_string()));
                        relm.stream().emit(Msg::Close);
                        return ViewModel::default();
                    }
                }
            }
            ModelParam::Create(psbt, _) => psbt,
        };
        ViewModel::with(psbt.into(), path, network)
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::Sign(_output_no) => {
                // TODO: Implement signing
            }
            Msg::Launcher(msg) => {
                self.launcher_stream.as_ref().map(|stream| stream.emit(msg));
            }
            Msg::Close => self.close(),
            Msg::FileError(path, err) => {
                self.widgets.hide();
                error_dlg(
                    self.widgets.as_root(),
                    "Error opening PSBT",
                    &path.display().to_string(),
                    Some(&err.to_string()),
                );
                self.close();
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
