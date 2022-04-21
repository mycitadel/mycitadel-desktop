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
use gtk::prelude::*;
use gtk::Dialog;
use relm::{init, Channel, Relm, StreamHandle, Update, Widget};

use super::{spending_row::Condition, xpub_dlg, Msg, ViewModel, Widgets};
use crate::model::{Bip43, WalletDescriptor};
use crate::view::{devices, launch, wallet};

pub struct Component {
    model: ViewModel,
    widgets: Widgets,
    devices: relm::Component<devices::Component>,
    xpub_dlg: relm::Component<xpub_dlg::Component>,
    launcher_stream: Option<StreamHandle<launch::Msg>>,
    wallet_stream: Option<StreamHandle<wallet::Msg>>,
}

impl Component {
    fn update_descriptor(&mut self) {
        self.model.update_descriptor();
        self.widgets
            .update_descriptor(self.model.descriptor.as_ref(), self.model.format_lnpbp);
    }

    fn condition_selection_change(&mut self) {
        let removable =
            self.widgets.selected_condition_index().is_some() && self.model.spendings.n_items() > 1;
        self.widgets.set_remove_condition(removable);
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
        ViewModel::default()
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::New(template) => {
                self.model = match template {
                    Some(template) => template.into(),
                    None => ViewModel::default(),
                };
                self.model.new_wallet = true;
                self.widgets
                    .reinit_ui(self.model.new_wallet, &self.model.template)
            }
            Msg::View(descriptor) => {
                self.model = ViewModel::from(descriptor);
                self.model.new_wallet = false;
                self.widgets.reinit_ui(self.model.new_wallet, &None)
            }
            Msg::AddDevices => {
                self.devices.emit(devices::Msg::Show);
            }
            Msg::AddReadOnly => {
                let testnet = self.model.network.is_testnet();
                let format = Bip43::try_from(&self.model.scheme)
                    .ok()
                    .and_then(Bip43::slip_application);
                self.xpub_dlg.emit(xpub_dlg::Msg::Open(testnet, format));
            }
            Msg::SignerAddDevice(fingerprint, device) => {
                self.model.devices.insert(fingerprint, device);
                self.model.update_signers();
                self.widgets.update_signers(&self.model.signers);
                self.widgets
                    .update_descriptor(self.model.descriptor.as_ref(), self.model.format_lnpbp);
            }
            Msg::SignerSelect => {
                let signer = self
                    .widgets
                    .selected_signer_xpub()
                    .and_then(|xpub| self.model.signer_by(xpub));
                self.widgets
                    .update_signer_details(signer.map(|s| (s, self.model.derivation_for(s))));
            }
            Msg::AddXpub(xpub) => {
                self.model.signers.insert(xpub.into());
                self.widgets.update_signers(&self.model.signers);
                self.widgets
                    .update_descriptor(self.model.descriptor.as_ref(), self.model.format_lnpbp);
            }
            Msg::ToggleClass(class) => {
                if self.widgets.should_update_descr_class(class)
                    && self.model.toggle_descr_class(class)
                {
                    self.widgets.update_descr_class(self.model.class);
                    self.widgets
                        .update_descriptor(self.model.descriptor.as_ref(), self.model.format_lnpbp);
                }
            }
            Msg::ExportFormat(lnpbp) => {
                self.model.format_lnpbp = lnpbp;
                self.widgets
                    .update_descriptor(self.model.descriptor.as_ref(), self.model.format_lnpbp);
            }
            Msg::Update => {
                let descr = WalletDescriptor::from(&self.model);
                self.wallet_stream.as_ref().map(|stream| {
                    stream.emit(wallet::Msg::Update(descr));
                });
                self.widgets.hide();
            }
            Msg::Close => {
                self.widgets.hide();
                if self.model.new_wallet {
                    self.launcher_stream
                        .as_ref()
                        .map(|stream| stream.emit(launch::Msg::Show));
                }
            }
            Msg::ConditionAdd => {
                self.model.spendings.append(&Condition::default());
                self.condition_selection_change();
                self.update_descriptor();
            }
            Msg::ConditionRemove => {
                let index = if let Some(index) = self.widgets.selected_condition_index() {
                    index
                } else {
                    return;
                };
                self.model.spendings.remove(index as u32);
                self.update_descriptor();
            }
            Msg::ConditionSelect => {
                self.condition_selection_change();
            }
            Msg::ConditionChange => {
                self.update_descriptor();
            }
            Msg::SetWallet(stream) => {
                self.wallet_stream = Some(stream);
            }
            Msg::SetLauncher(stream) => {
                self.launcher_stream = Some(stream);
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
        let glade_src = include_str!("settings.glade");
        let widgets = Widgets::from_string(glade_src).expect("glade file broken");

        let stream = relm.stream().clone();
        let (_channel, sender) = Channel::new(move |msg| {
            stream.emit(msg);
        });

        let devices =
            init::<devices::Component>((model.scheme.clone(), model.network, sender.clone()))
                .expect("error in devices component");
        let xpub_dlg =
            init::<xpub_dlg::Component>((sender,)).expect("error in xpub dialog component");

        widgets.connect(relm);
        widgets.bind_model(relm, &model.spendings);

        Component {
            model,
            widgets,
            devices,
            xpub_dlg,
            launcher_stream: None,
            wallet_stream: None,
        }
    }
}
