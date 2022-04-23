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
use crate::view::{file_create_dlg, file_open_dlg, settings, wallet};

pub struct Component {
    model: ViewModel,
    widgets: Widgets,
    stream: StreamHandle<Msg>,
    wallet_settings: relm::Component<settings::Component>,
    // TODO: Make a BTreeMap from wallet ids
    wallets: Vec<relm::Component<wallet::Component>>,
}

impl Component {
    fn open_wallet(&mut self, path: PathBuf) {
        let wallet =
            init::<wallet::Component>(path).expect("unable to instantiate wallet settings");
        wallet.emit(wallet::Msg::RegisterLauncher(self.stream.clone()));
        self.wallets.push(wallet);
    }
}

impl Update for Component {
    // Specify the model used for this widget.
    type Model = ViewModel;
    // Specify the model parameter used to init the model.
    type ModelParam = ();
    // Specify the type of the messages sent to the update function.
    type Msg = Msg;

    fn model(_relm: &Relm<Self>, _model: Self::ModelParam) -> Self::Model {
        ViewModel {}
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::Show => self.widgets.show(),
            Msg::Quit => gtk::main_quit(),
            Msg::TemplateSelected => {
                if let Some(path) = file_create_dlg(
                    self.widgets.as_root(),
                    "Create wallet",
                    "MyCitadel wallet",
                    "*.mcw",
                    "citadel-01",
                ) {
                    self.widgets.hide();
                    self.wallet_settings
                        .emit(settings::Msg::New(self.widgets.selected_template(), path));
                }
            }
            Msg::ImportSelected => {}
            Msg::OpenSelected => {
                if let Some(path) = file_open_dlg(
                    self.widgets.as_root(),
                    "Open wallet",
                    "MyCitadel wallet",
                    "*.mcw",
                ) {
                    self.widgets.hide();
                    self.open_wallet(path)
                }
            }
            Msg::RecentSelected => {
                if let Some(path) = self.widgets.selected_recent() {
                    self.widgets.hide();
                    self.open_wallet(path)
                }
            }
            Msg::WalletCreated(path) => self.open_wallet(path),
            Msg::OpenWallet(path) => self.open_wallet(path),
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
        let glade_src = include_str!("launch.glade");
        let widgets = Widgets::from_string(glade_src).expect("glade file broken");

        let new_wallet =
            init::<settings::Component>(()).expect("unable to instantiate wallet settings");
        new_wallet.emit(settings::Msg::SetLauncher(relm.stream().clone()));

        widgets.connect(relm);
        widgets.show();

        Component {
            widgets,
            model,
            wallet_settings: new_wallet,
            wallets: empty!(),
            stream: relm.stream().clone(),
        }
    }
}
