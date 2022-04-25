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
use gtk::{ApplicationWindow, ResponseType};
use relm::{init, Channel, Relm, StreamHandle, Update, Widget};

use super::{Msg, ViewModel, Widgets};
use crate::model::{FileDocument, Wallet};
use crate::view::wallet::WatchMsg;
use crate::view::{error_dlg, launch, pay, settings};
use crate::worker::ElectrumWatcher;

pub struct Component {
    model: ViewModel,
    widgets: Widgets,
    channel: Channel<WatchMsg>,
    watcher: ElectrumWatcher,
    settings: relm::Component<settings::Component>,
    payment: relm::Component<pay::Component>,
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
            Msg::Close => {
                self.launcher_stream
                    .as_ref()
                    .map(|stream| stream.emit(launch::Msg::WalletClosed));
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
            Msg::Pay => self.payment.emit(pay::Msg::Show),
            Msg::Settings => self.settings.emit(settings::Msg::View(
                self.model.to_settings(),
                self.model.path().clone(),
            )),
            Msg::Update(signers, descriptor_classes) => {
                if let Err(err) = self.model.update_descriptor(signers, descriptor_classes) {
                    error_dlg(
                        self.widgets.as_root(),
                        "Internal error",
                        "Please report the following information to the developer",
                        Some(&err.to_string()),
                    );
                } else {
                    self.widgets.show();
                    self.settings
                        .emit(settings::Msg::Response(ResponseType::Cancel));
                }
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

        let payment =
            init::<pay::Component>(model.to_wallet()).expect("error in settings component");
        payment.emit(pay::Msg::SetWallet(relm.stream().clone()));

        let stream = relm.stream().clone();
        let (channel, sender) = Channel::new(move |msg| stream.emit(Msg::ElectrumWatch(msg)));
        let watcher = ElectrumWatcher::with(sender, model.as_wallet().to_settings())
            .expect("unable to instantiate watcher thread");

        widgets.connect(relm);
        widgets.update_ui(&model);
        widgets.show();

        Component {
            model,
            widgets,
            settings,
            payment,
            channel,
            watcher,
            launcher_stream: None,
        }
    }
}
