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

use super::{spending_row::Condition, ModelParam, Msg, ViewModel, Widgets};
use crate::model::WalletDescriptor;
use crate::view::{devices, wallet};

pub struct Component {
    model: ViewModel,
    widgets: Widgets,
    devices: relm::Component<devices::Win>,
    parent_stream: Option<StreamHandle<wallet::Msg>>,
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
    type ModelParam = ModelParam;
    // Specify the type of the messages sent to the update function.
    type Msg = Msg;

    fn model(relm: &Relm<Self>, model: Self::ModelParam) -> Self::Model {
        relm.stream().emit(Msg::New);
        ViewModel::from(model)
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::New => self.widgets.reinit_ui(true, &self.model.template),
            Msg::View(descriptor) => {
                self.model = ViewModel::from(descriptor);
                self.widgets.reinit_ui(false, &None)
            }
            Msg::DevicesList => {
                self.devices.emit(devices::Msg::Show);
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
                self.parent_stream.as_ref().map(|stream| {
                    stream.emit(wallet::Msg::Update(descr));
                });
                self.widgets.hide();
            }
            Msg::Hide => {
                self.widgets.hide();
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
            Msg::SetParent(stream) => {
                self.parent_stream = Some(stream);
            }
        }
    }
}

impl Widget for Component {
    // Specify the type of the root widget.
    type Root = Dialog;

    // Return the root widget.
    fn root(&self) -> Self::Root {
        self.widgets.root()
    }

    fn view(relm: &Relm<Self>, model: Self::Model) -> Self {
        let glade_src = include_str!("settings.glade");
        let widgets = Widgets::from_string(glade_src).expect("glade file broken");

        let stream = relm.stream().clone();
        let (_channel, sender) = Channel::new(move |msg| {
            stream.emit(msg);
        });

        let devices = init::<devices::Win>((model.scheme.clone(), model.network, sender))
            .expect("error in devices component");

        widgets.connect(relm);
        widgets.bind_model(relm, &model.spendings);

        Component {
            model,
            widgets,
            devices,
            parent_stream: None,
        }
    }
}
