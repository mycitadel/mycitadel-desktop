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

use std::path::PathBuf;

use gladis::Gladis;
use gtk::ApplicationWindow;
use relm::{init, Relm, StreamHandle, Update, Widget};

use super::{Msg, ViewModel, Widgets};
use crate::model::{FileDocument, Wallet};
use crate::view::{error_dlg, launch, settings};

pub struct Component {
    model: ViewModel,
    widgets: Widgets,
    settings: relm::Component<settings::Component>,
    launcher_stream: Option<StreamHandle<launch::Msg>>,
}

impl Component {
    fn close(&self) {
        // TODO: Signal to launcher
        self.widgets.close();
    }
}

impl Update for Component {
    // Specify the model used for this widget.
    type Model = ViewModel;
    // Specify the model parameter used to init the model.
    type ModelParam = PathBuf;
    // Specify the type of the messages sent to the update function.
    type Msg = Msg;

    fn model(relm: &Relm<Self>, path: Self::ModelParam) -> Self::Model {
        let wallet = Wallet::read_file(&path)
            .map_err(|err| {
                relm.stream()
                    .emit(Msg::FileError(path.clone(), err.to_string()))
            })
            .unwrap_or_default();
        ViewModel::with(wallet, path)
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::New => {
                self.launcher_stream
                    .as_ref()
                    .map(|stream| stream.emit(launch::Msg::Show));
            }
            Msg::Open => {
                self.launcher_stream
                    .as_ref()
                    .map(|stream| stream.emit(launch::Msg::OpenSelected));
            }
            Msg::FileError(path, err) => {
                self.widgets.hide();
                error_dlg(
                    self.widgets.as_root(),
                    "Error opening wallet",
                    &path.display().to_string(),
                    Some(&err.to_string()),
                );
                self.close();
            }
            Msg::Settings => self.settings.emit(settings::Msg::View(
                self.model.to_descriptor(),
                self.model.path().clone(),
            )),
            Msg::Update(descr) => {
                self.model.set_descriptor(descr);
                self.widgets.show();
            }
            Msg::RegisterLauncher(stream) => {
                self.launcher_stream = Some(stream);
            }
            _ => { /* TODO: Implement main window event handling */ }
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
        let glade_src = include_str!("wallet.glade");
        let widgets = Widgets::from_string(glade_src).expect("glade file broken");

        let settings = init::<settings::Component>(()).expect("error in settings component");
        settings.emit(settings::Msg::SetWallet(relm.stream().clone()));

        widgets.connect(relm);
        widgets.show();

        Component {
            model,
            widgets,
            settings,
            launcher_stream: None,
        }
    }
}
