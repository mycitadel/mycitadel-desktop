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

use gladis::Gladis;
use gtk::{MessageDialog, ResponseType};
use relm::{Relm, Sender, Update, Widget};
use std::str::FromStr;

use super::{Msg, ViewModel, Widgets};
use crate::model::{WalletStandard, XpubDescriptor, XpubParseError, XpubRequirementError};
use crate::view::settings;

pub struct Component {
    model: ViewModel,
    widgets: Widgets,
}

impl Component {
    fn process_xpub(&mut self) {
        let xpub = self.widgets.xpub();
        match XpubDescriptor::from_str_checked(
            &xpub,
            self.model.testnet,
            Some(self.model.standard.clone()),
        ) {
            Ok(xpub) => {
                self.widgets.hide_message();
                self.model.xpub = Some(xpub)
            }
            Err(XpubParseError::Inconsistency(
                err @ XpubRequirementError::TestnetMismatch { .. },
            )) => {
                self.model.xpub = None;
                self.widgets.show_error(&err.to_string())
            }
            Err(XpubParseError::Inconsistency(err)) => {
                self.model.xpub = XpubDescriptor::from_str(&xpub).ok();
                self.widgets.show_warning(&err.to_string())
            }
            Err(err) => {
                self.model.xpub = None;
                self.widgets.show_error(&err.to_string())
            }
        }
    }
}

impl Update for Component {
    // Specify the model used for this widget.
    type Model = ViewModel;
    // Specify the model parameter used to init the model.
    type ModelParam = (WalletStandard, Sender<settings::Msg>);
    // Specify the type of the messages sent to the update function.
    type Msg = Msg;

    fn model(_relm: &Relm<Self>, model: Self::ModelParam) -> Self::Model {
        ViewModel::with(model.0, model.1)
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::Open(testnet, format) => {
                self.model.testnet = testnet;
                self.model.slip_format = format;
                self.widgets.open();
            }
            Msg::Edit => {
                self.process_xpub();
            }
            Msg::Error(msg) => self.widgets.show_error(&msg),
            Msg::Warning(msg) => self.widgets.show_warning(&msg),
            Msg::Info(msg) => self.widgets.show_info(&msg),
            Msg::Response(ResponseType::Cancel) | Msg::Response(ResponseType::DeleteEvent) => {
                self.widgets.close();
            }
            Msg::Response(ResponseType::Ok) => {
                if let Some(ref xpub) = self.model.xpub {
                    self.model
                        .sender
                        .send(settings::Msg::SignerAddXpub(xpub.into()))
                        .expect("communication of xpub dialog with settings window");
                    self.widgets.close();
                } else {
                    self.widgets.show_notification();
                }
            }
            Msg::Response(resp) => {}
        }
    }
}

impl Widget for Component {
    // Specify the type of the root widget.
    type Root = MessageDialog;

    // Return the root widget.
    fn root(&self) -> Self::Root {
        self.widgets.to_root()
    }

    fn view(relm: &Relm<Self>, model: Self::Model) -> Self {
        let glade_src = include_str!("xpub_dlg.glade");
        let widgets = Widgets::from_string(glade_src).expect("glade file broken");

        widgets.connect(relm);

        Component { model, widgets }
    }
}
