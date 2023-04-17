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

use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;

use ::wallet::onchain::PublicNetwork;
use ::wallet::psbt::Psbt;
use bitcoin::consensus::Decodable;
use bitcoin::psbt::PartiallySignedTransaction;
use bpro::{FileDocument, Wallet};
use gladis::Gladis;
use gtk::{ApplicationWindow, ResponseType};
use relm::{init, Relm, StreamHandle, Update, Widget};

use super::{Msg, ViewModel, Widgets};
use crate::view::launch::Page;
use crate::view::{about, error_dlg, file_create_dlg, file_open_dlg, psbt, settings, wallet};

/// Main [`relm`] component of the application
///
/// Most importantly implements [`relm::Widget`] as an UI application.
/// See [`Component::view`] for initialization code.
pub struct Component {
    model: ViewModel,
    widgets: Widgets,
    stream: StreamHandle<Msg>,
    wallet_settings: relm::Component<settings::Component>,
    // TODO: Make a BTreeMap from wallet ids
    wallets: Vec<relm::Component<wallet::Component>>,
    psbts: Vec<relm::Component<psbt::Component>>,
    about: relm::Component<about::Component>,
    wallet_count: usize,
    window_count: usize,
}

impl Component {
    fn open_file(&mut self, path: PathBuf) -> bool {
        match path.extension().and_then(OsStr::to_str) {
            Some("mcw") => self.open_wallet(path),
            _ => self.open_psbt(path, default!()),
        }
    }

    fn open_wallet(&mut self, path: PathBuf) -> bool {
        match Wallet::read_file(&path) {
            Err(err) => {
                error_dlg(
                    self.widgets.as_root(),
                    "Error opening wallet",
                    &format!("Unable to open wallet file {}", path.display()),
                    Some(&err.to_string()),
                );
                false
            }
            Ok(wallet) => {
                let wallet = init::<wallet::Component>((wallet, path))
                    .expect("unable to instantiate wallet settings");
                self.window_count += 1;
                wallet.emit(wallet::Msg::RegisterLauncher(self.stream.clone()));
                self.wallets.push(wallet);
                true
            }
        }
    }

    fn open_psbt(&mut self, path: PathBuf, network: Option<PublicNetwork>) -> bool {
        let mut file = match fs::File::open(&path) {
            Ok(file) => file,
            Err(err) => {
                error_dlg(
                    self.widgets.as_root(),
                    "Error opening PSBT",
                    &path.display().to_string(),
                    Some(&err.to_string()),
                );
                return false;
            }
        };
        let psbt = match PartiallySignedTransaction::consensus_decode(&mut file) {
            Ok(psbt) => psbt.into(),
            Err(err) => {
                error_dlg(
                    self.widgets.as_root(),
                    "Invalid PSBT file",
                    &path.display().to_string(),
                    Some(&err.to_string()),
                );
                return false;
            }
        };

        let comppnent = init::<psbt::Component>(psbt::ModelParam::Open(
            path,
            psbt,
            network.unwrap_or(PublicNetwork::Mainnet),
        ))
        .expect("unable to instantiate wallet settings");
        self.window_count += 1;
        comppnent.emit(psbt::Msg::RegisterLauncher(self.stream.clone()));
        self.psbts.push(comppnent);
        true
    }

    fn create_psbt(&mut self, psbt: Psbt, network: PublicNetwork) {
        let psbt = init::<psbt::Component>(psbt::ModelParam::Create(psbt, network))
            .expect("unable to instantiate wallet settings");
        self.window_count += 1;
        psbt.emit(psbt::Msg::RegisterLauncher(self.stream.clone()));
        self.psbts.push(psbt);
    }
}

impl Update for Component {
    // Specify the model used for this widget.
    type Model = ViewModel;
    // Specify the model parameter used to init the model.
    type ModelParam = ();
    // Specify the type of the messages sent to the update function.
    type Msg = Msg;

    fn model(_relm: &Relm<Self>, _model: Self::ModelParam) -> Self::Model { ViewModel {} }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::Show => self.widgets.show(None),
            Msg::ShowPage(page) => self.widgets.show(Some(page)),
            Msg::Close => {
                if self.window_count == 0 {
                    gtk::main_quit();
                } else {
                    self.widgets.hide();
                }
            }
            Msg::WalletClosed => {
                self.window_count -= 1;
                if self.window_count == 0 {
                    self.widgets.show(None);
                }
                // TODO: Remove wallet window from the list of windows
            }
            Msg::PsbtClosed => {
                self.window_count -= 1;
                if self.window_count == 0 {
                    self.widgets.show(None);
                }
                // TODO: Remove PSBT window from the list of windows
            }
            Msg::Template(index) => {
                if let Some(path) = file_create_dlg(
                    Some(self.widgets.as_root()),
                    "Create wallet",
                    "MyCitadel wallet",
                    "*.mcw",
                    &Wallet::file_name("citadel", self.wallet_count),
                ) {
                    let template = self.widgets.template(index);
                    self.wallet_count += 1;
                    self.widgets.hide();
                    self.wallet_settings
                        .emit(settings::Msg::New(template, path));
                }
            }
            Msg::Duplicate(settings, path) => {
                if let Some(path) =
                    file_create_dlg(None, "Copy wallet", "MyCitadel wallet", "*.mcw", &path)
                {
                    self.wallet_count += 1;
                    self.widgets.hide();
                    self.wallet_settings
                        .emit(settings::Msg::Duplicate(settings, path));
                }
            }
            Msg::Import => {}
            Msg::Wallet => {
                if let Some(path) = file_open_dlg(None, "Open wallet", "MyCitadel wallet", "*.mcw")
                {
                    self.widgets.hide();
                    if !self.open_wallet(path) {
                        self.widgets.show(None);
                    }
                }
            }
            Msg::Psbt(network) => {
                if let Some(path) = file_open_dlg(
                    None,
                    "Open PSBT",
                    "Partially signed bitcoin transaction",
                    "*.psbt",
                ) {
                    self.widgets.hide();
                    if !self.open_psbt(path, network) {
                        self.widgets.show(None);
                    }
                }
            }
            Msg::Recent => {
                if let Some(path) = self.widgets.selected_recent() {
                    self.widgets.hide();
                    if !self.open_file(path) {
                        self.widgets.show(None);
                    }
                }
            }
            Msg::About => self.about.emit(about::Msg::Show),
            Msg::WalletCreated(path) => {
                if !self.open_wallet(path) {
                    self.widgets.show(None);
                }
            }
            Msg::CreatePsbt(psbt, network) => self.create_psbt(psbt, network),
        }
    }
}

impl Widget for Component {
    // Specify the type of the root widget.
    type Root = ApplicationWindow;

    // Return the root widget.
    fn root(&self) -> Self::Root { self.widgets.to_root() }

    fn view(relm: &Relm<Self>, model: Self::Model) -> Self {
        let glade_src = include_str!("launch.glade");
        let widgets = Widgets::from_string(glade_src).expect("glade file broken");

        widgets.init_ui();

        let new_wallet =
            init::<settings::Component>(()).expect("unable to instantiate wallet settings");
        new_wallet.emit(settings::Msg::SetLauncher(relm.stream().clone()));
        let about = init::<about::Component>(()).expect("unable to instantiate about settings");
        about.emit(about::Msg::Response(ResponseType::Close));

        widgets.connect(relm);
        widgets.show(Some(Page::Template));

        Component {
            widgets,
            model,
            wallet_settings: new_wallet,
            wallets: empty!(),
            psbts: empty!(),
            about,
            stream: relm.stream().clone(),
            wallet_count: 1,
            window_count: 0,
        }
    }
}
