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

use std::str::FromStr;

use bitcoin::secp256k1::Secp256k1;
use bitcoin::util::bip32::{ExtendedPrivKey, Fingerprint};
use gladis::Gladis;
use gtk::{MessageDialog, ResponseType};
use relm::{Relm, Sender, Update, Widget};
use wallet::psbt::sign::SignAll;
use wallet::psbt::Psbt;

use super::{Msg, ViewModel, Widgets};
use crate::model::{PublicNetwork, XprivSigner};
use crate::view::psbt;

pub struct Component {
    model: ViewModel,
    widgets: Widgets,
}

impl Component {
    fn process_xpriv(&mut self) {
        let xpriv = self.widgets.xpriv();

        let xpriv = match ExtendedPrivKey::from_str(&xpriv) {
            Ok(xpriv) => {
                self.widgets.hide_message();
                self.model.xpriv = Some(xpriv);
                xpriv
            }
            Err(err) => {
                self.model.xpriv = None;
                self.widgets.show_error(&err.to_string());
                return;
            }
        };

        let network = PublicNetwork::try_from(xpriv.network)
            .expect("xpriv network always either mainnet or testnet");
        if network.is_testnet() != self.model.testnet {
            self.widgets.show_error(
                "network used by the PSBT and network of the provided private key does not match",
            );
            return;
        }

        let signer = XprivSigner {
            xpriv,
            master_fp: self.model.master_fp,
            secp: Secp256k1::new(),
        };

        match self.model.psbt.sign_all(&signer) {
            Ok(0) => {
                self.widgets
                    .show_error("The provided key can't sign any of the transaction inputs");
            }
            Ok(_count) => {
                self.widgets.hide_message();
            }
            Err(err) => {
                self.widgets
                    .show_error(&format!("Unable to sign with the provided key: {}", err));
            }
        }
    }
}

impl Update for Component {
    // Specify the model used for this widget.
    type Model = ViewModel;
    // Specify the model parameter used to init the model.
    type ModelParam = (bool, Psbt, Fingerprint, Sender<psbt::Msg>);
    // Specify the type of the messages sent to the update function.
    type Msg = Msg;

    fn model(_relm: &Relm<Self>, model: Self::ModelParam) -> Self::Model {
        ViewModel::with(model.0, model.1, model.2, model.3)
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::Open(testnet, psbt, master_fp) => {
                self.model.testnet = testnet;
                self.model.psbt = psbt;
                self.model.master_fp = master_fp;
                self.widgets.open();
            }
            Msg::Edit => {
                self.process_xpriv();
            }
            Msg::Error(msg) => self.widgets.show_error(&msg),
            Msg::Warning(msg) => self.widgets.show_warning(&msg),
            Msg::Info(msg) => self.widgets.show_info(&msg),
            Msg::Response(ResponseType::Cancel) | Msg::Response(ResponseType::DeleteEvent) => {
                self.widgets.close();
            }
            Msg::Response(ResponseType::Ok) => {
                self.model
                    .sender
                    .send(psbt::Msg::Signed(self.model.psbt.clone()))
                    .expect("communication of xpriv dialog with PSBT window");
                self.widgets.close();
            }
            Msg::Response(_) => {}
        }
    }
}

impl Widget for Component {
    // Specify the type of the root widget.
    type Root = MessageDialog;

    // Return the root widget.
    fn root(&self) -> Self::Root { self.widgets.to_root() }

    fn view(relm: &Relm<Self>, model: Self::Model) -> Self {
        let glade_src = include_str!("xpriv_dlg.glade");
        let widgets = Widgets::from_string(glade_src).expect("glade file broken");

        widgets.connect(relm);

        Component { model, widgets }
    }
}
