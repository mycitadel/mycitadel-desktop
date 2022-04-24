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

use std::path::Path;
use std::str::FromStr;

use ::wallet::hd::DerivationStandard;
use bitcoin::util::bip32::Fingerprint;
use gladis::Gladis;
use gtk::prelude::*;
use gtk::{Dialog, ResponseType};
use relm::{init, Channel, Relm, StreamHandle, Update, Widget};

use super::{spending_row::Condition, xpub_dlg, Msg, ViewModel, Widgets};
use crate::model::{Signer, WalletDescriptor};
use crate::view::{devices, error_dlg, launch, wallet};

pub struct Component {
    model: ViewModel,
    widgets: Widgets,
    devices: relm::Component<devices::Component>,
    xpub_dlg: relm::Component<xpub_dlg::Component>,
    launcher_stream: Option<StreamHandle<launch::Msg>>,
    wallet_stream: Option<StreamHandle<wallet::Msg>>,
}

impl Component {
    fn close(&self) {
        if let Err(err) = self.model.save() {
            error_dlg(
                self.widgets.as_root(),
                "Error saving new wallet",
                &self.model.filename(),
                Some(&err.to_string()),
            );
        }
        self.widgets.hide();
        if self.model.is_new_wallet() {
            self.launcher_stream
                .as_ref()
                .map(|stream| stream.emit(launch::Msg::Show));
        }
    }

    fn new_wallet_path(&self) -> Option<&Path> {
        if self.model.is_new_wallet() {
            return Some(self.model.path());
        }
        None
    }

    fn replace_signer(&mut self) {
        if let Some(signer) = self.model.active_signer.clone() {
            self.widgets.replace_signer(&signer);
            debug_assert!(self.model.replace_signer(signer));
            self.sync();
        }
    }

    fn condition_selection_change(&mut self) {
        let removable = self.widgets.selected_condition_index().is_some()
            && self.model.spending_model.n_items() > 1;
        self.widgets.set_remove_condition(removable);
    }

    fn sync(&mut self) {
        if let Err(err) = self.model.update_descriptor() {
            self.widgets.show_error(&err.to_string());
        }
        self.widgets
            .update_descriptor(self.model.descriptor.as_ref(), self.model.export_lnpbp);
        if let Err(err) = self.model.save() {
            self.widgets.show_error(&err.to_string());
        } else {
            self.widgets.hide_message();
        }
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
        // First, we process events which does not update the state
        let event = match event {
            Msg::New(template, path) => {
                let template = template.unwrap_or_default();
                self.model = match ViewModel::new(template.clone(), path) {
                    Err(err) => {
                        error_dlg(
                            self.widgets.as_root(),
                            "Error saving wallet",
                            &self.model.filename(),
                            Some(&err.to_string()),
                        );
                        // We need this, otherwise self.close() would not work
                        self.model.template = Some(template);
                        self.close();
                        return;
                    }
                    Ok(model) => model,
                };
                self.widgets
                    .reinit_ui(&self.model.template, self.model.path());
                return;
            }
            Msg::View(descriptor, path) => {
                self.model = ViewModel::with(descriptor, path);
                self.widgets
                    .reinit_ui(&self.model.template, &self.model.path());
                return;
            }
            Msg::AddDevices => {
                self.devices.emit(devices::Msg::Show);
                return;
            }
            Msg::AddReadOnly => {
                let testnet = self.model.network.is_testnet();
                let format = self.model.bip43().slip_application();
                self.xpub_dlg.emit(xpub_dlg::Msg::Open(testnet, format));
                return;
            }
            Msg::SignerSelect => {
                let signer = self
                    .widgets
                    .selected_signer_xpub()
                    .and_then(|xpub| self.model.signer_by(xpub));
                self.widgets.update_signer_details(
                    signer.map(|s| (s, self.model.derivation_for(s))),
                    self.model.network,
                );
                self.model.active_signer = signer.cloned();
                return;
            }
            Msg::ExportFormat(lnpbp) => {
                self.model.export_lnpbp = lnpbp;
                self.sync();
                return;
            }
            Msg::ConditionSelect => {
                self.condition_selection_change();
                return;
            }
            Msg::SetWallet(stream) => {
                self.wallet_stream = Some(stream);
                return;
            }
            Msg::SetLauncher(stream) => {
                self.launcher_stream = Some(stream);
                return;
            }
            Msg::Response(ResponseType::Ok) => {
                let descr = match WalletDescriptor::try_from(&self.model) {
                    Err(err) => {
                        error_dlg(
                            self.widgets.as_root(),
                            "Error in wallet settings",
                            &err.to_string(),
                            None,
                        );
                        return;
                    }
                    Ok(descr) => descr,
                };
                if let Some(path) = self.new_wallet_path() {
                    self.launcher_stream.as_ref().map(|stream| {
                        stream.emit(launch::Msg::WalletCreated(path.to_owned()));
                    });
                } else {
                    self.wallet_stream.as_ref().map(|stream| {
                        stream.emit(wallet::Msg::Update(
                            descr.signers().clone(),
                            descr.descriptor_classes().clone(),
                        ));
                    });
                }
                self.widgets.hide();
                return;
            }
            Msg::Response(ResponseType::Cancel) => {
                self.close();
                return;
            }
            _ => event,
        };

        // Than, events which update the state and require saving or descriptor change
        match event {
            Msg::SignerAddDevice(fingerprint, device) => {
                self.model.devices.insert(fingerprint, device);
                self.model.update_signers();
                self.widgets.update_signers(&self.model.signers);
            }
            Msg::SignerAddXpub(xpub) => {
                self.model.signers.push(Signer::with_xpub(
                    xpub,
                    &self.model.bip43(),
                    self.model.network,
                ));
                self.widgets.update_signers(&self.model.signers);
            }
            Msg::SignerFingerprintChange => {
                let fingerprint = match Fingerprint::from_str(&self.widgets.signer_fingerprint()) {
                    Err(_) => {
                        self.widgets.show_error("incorrect fingerprint value");
                        return;
                    }
                    Ok(fingerprint) => {
                        self.widgets.hide_message();
                        fingerprint
                    }
                };
                if let Some(ref mut signer) = self.model.active_signer {
                    if signer.master_fp == fingerprint {
                        return;
                    }
                    signer.master_fp = fingerprint;
                    self.replace_signer();
                }
            }
            Msg::SignerNameChange => {
                if let Some(ref mut signer) = self.model.active_signer {
                    let name = self.widgets.signer_name();
                    if signer.name == name {
                        return;
                    }
                    signer.name = name;
                    self.replace_signer();
                }
            }
            Msg::SignerOwnershipChange => {
                if let Some(ref mut signer) = self.model.active_signer {
                    let ownership = self.widgets.signer_ownership();
                    if signer.ownership == ownership {
                        return;
                    }
                    signer.ownership = ownership;
                    self.replace_signer();
                }
            }
            Msg::SignerOriginUpdate => {}
            Msg::ConditionAdd => {
                self.model.spending_model.append(&Condition::default());
                self.condition_selection_change();
            }
            Msg::ConditionRemove => {
                let index = if let Some(index) = self.widgets.selected_condition_index() {
                    index
                } else {
                    return;
                };
                self.model.spending_model.remove(index as u32);
            }
            Msg::ConditionChange => {}
            Msg::ToggleClass(class) => {
                if self.widgets.should_update_descr_class(class)
                    && self.model.toggle_descr_class(class)
                {
                    self.widgets
                        .update_descr_classes(&self.model.descriptor_classes);
                }
            }
            _ => {}
        }

        self.sync();
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

        let devices = init::<devices::Component>((model.bip43(), model.network, sender.clone()))
            .expect("error in devices component");
        let xpub_dlg = init::<xpub_dlg::Component>((model.bip43().into(), sender))
            .expect("error in xpub dialog component");

        widgets.connect(relm);
        widgets.bind_spending_model(relm, &model.spending_model);

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
