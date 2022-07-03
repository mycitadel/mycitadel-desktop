use std::str::FromStr;

use bitcoin::util::bip32::{ChildNumber, ExtendedPubKey, Fingerprint};
use bpro::{Error, HardwareList};
use gladis::Gladis;
use glib::subclass::prelude::*;
use gtk::prelude::*;
use gtk::{glib, Button, Dialog, ListBox, MessageDialog};
use relm::{Channel, Relm, Sender, Update, Widget};
use wallet::hd::{Bip43, DerivationStandard, HardenedIndex, SegmentIndexes};
use wallet::hwi;
use wallet::onchain::PublicNetwork;

use super::device_row::{DeviceModel, RowWidgets};
use crate::view::settings;

#[derive(Clone)]
pub struct ViewModel {
    pub(self) scheme: Bip43,
    pub(self) network: PublicNetwork,
    pub(self) devices: DeviceModel,
    pub(self) hwi: HardwareList,
    pub(self) sender: Sender<settings::Msg>,
}

#[derive(Msg)]
pub enum Msg {
    Show(Bip43),
    Refresh,
    Devices(Result<(HardwareList, Vec<Error>), Error>),
    AccountChange(Fingerprint, u32),
    Xpub(Fingerprint, String),
    XpubErr(Fingerprint, hwi::error::Error),
    Add(Fingerprint),
    Close,
}

#[derive(Clone, Gladis)]
pub struct Widgets {
    dialog: Dialog,
    close_btn: Button,
    refresh_btn: Button,
    refresh_dlg: Dialog,
    device_list: ListBox,
    error_dlg: MessageDialog,
}

pub struct Component {
    model: ViewModel,
    sender: Sender<Msg>,
    widgets: Widgets,
}

impl Update for Component {
    // Specify the model used for this widget.
    type Model = ViewModel;
    // Specify the model parameter used to init the model.
    type ModelParam = (Bip43, PublicNetwork, Sender<settings::Msg>);
    // Specify the type of the messages sent to the update function.
    type Msg = Msg;

    fn model(_relm: &Relm<Self>, model: Self::ModelParam) -> Self::Model {
        ViewModel {
            scheme: model.0,
            network: model.1,
            devices: DeviceModel::new(),
            hwi: default!(),
            sender: model.2,
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::Show(bip43) => {
                self.model.scheme = bip43;
                self.model.devices.clear();
                self.widgets.dialog.show();
                self.widgets.refresh_btn.emit_clicked();
            }
            Msg::Refresh => self.widgets.refresh_dlg.show(),
            Msg::Devices(result) => {
                self.widgets.refresh_dlg.hide();
                self.model.hwi = match result {
                    Err(err) => {
                        self.widgets
                            .error_dlg
                            .set_secondary_text(Some(&err.to_string()));
                        self.widgets.error_dlg.show();
                        HardwareList::default()
                    }
                    Ok((devices, log)) if !log.is_empty() => {
                        let err = log.into_iter().fold(s!(""), |mut err, entry| {
                            err.push_str(&entry.to_string());
                            err
                        });
                        self.widgets.error_dlg.set_secondary_text(Some(&err));
                        self.widgets.error_dlg.show();
                        devices
                    }
                    Ok((devices, _)) => devices,
                };
                self.model.devices.refresh(&self.model.hwi);
            }
            Msg::AccountChange(fingerprint, account) => {
                let imp = self.model.devices.imp().0.borrow();
                let model = imp
                    .iter()
                    .find(|device| device.fingerprint() == fingerprint)
                    .expect("device absent in the model");
                model.set_property("updating", true);
                let derivation = self.model.scheme.to_account_derivation(
                    ChildNumber::from_hardened_idx(account).expect("wrong account number"),
                    self.model.network.into(),
                );
                let testnet = self.model.network.is_testnet();
                let sender = self.sender.clone();
                let hwi = self.model.hwi[&fingerprint].device.clone();
                std::thread::spawn(move || {
                    let msg = match hwi.get_xpub(&derivation, testnet) {
                        Ok(xpub) => Msg::Xpub(fingerprint, xpub.xpub.to_string()),
                        Err(err) => Msg::XpubErr(fingerprint, err),
                    };
                    sender.send(msg).expect("message channel");
                });
            }
            Msg::Xpub(fingerprint, xpub) => {
                let imp = self.model.devices.imp().0.borrow();
                let model = imp
                    .iter()
                    .find(|device| device.fingerprint() == fingerprint)
                    .expect("device absent in the model");
                model.set_property("xpub", xpub);
                model.set_property("updating", false);
            }
            Msg::XpubErr(fingerprint, _err) => {
                let imp = self.model.devices.imp().0.borrow();
                let model = imp
                    .iter()
                    .find(|device| device.fingerprint() == fingerprint)
                    .expect("device absent in the model");
                model.set_property("xpub", "error retrieving xpub");
                model.set_property("updating", false);
            }
            Msg::Add(fingerprint) => {
                let imp = self.model.devices.imp().0.borrow();
                let model = imp
                    .iter()
                    .find(|device| device.fingerprint() == fingerprint)
                    .expect("device absent in the model");

                let mut device = self.model.hwi[&fingerprint].clone();
                device.default_account =
                    HardenedIndex::from_index(model.property::<u32>("account"))
                        .expect("wrong account");
                device.default_xpub = ExtendedPubKey::from_str(&model.property::<String>("xpub"))
                    .expect("wrong xpub");

                self.model
                    .sender
                    .send(settings::Msg::SignerAddDevice(fingerprint, device))
                    .expect("communication of devices dialog with settings window");
            }
            Msg::Close => {
                self.widgets.dialog.hide();
            }
        }
    }
}

impl Widget for Component {
    // Specify the type of the root widget.
    type Root = Dialog;

    // Return the root widget.
    fn root(&self) -> Self::Root { self.widgets.dialog.clone() }

    fn view(relm: &Relm<Self>, model: Self::Model) -> Self {
        let glade_src = include_str!("devices.glade");
        let widgets = Widgets::from_string(glade_src).expect("glade file broken");

        connect!(relm, widgets.close_btn, connect_clicked(_), Msg::Close);

        let stream = relm.stream().clone();
        let (_channel, sender) = Channel::new(move |msg| {
            stream.emit(msg);
        });
        let scheme = model.scheme.clone();
        let sender2 = sender.clone();
        widgets.refresh_btn.connect_clicked(move |_| {
            sender2
                .send(Msg::Refresh)
                .expect("broken channel in devices dialog");
            // TODO: This fixes the schema used in the wallet once and forever
            let scheme = scheme.clone();
            let sender = sender2.clone();
            // TODO: move enumeration into Refresh event processing
            std::thread::spawn(move || {
                let result = HardwareList::enumerate(&scheme, model.network, HardenedIndex::zero());
                sender
                    .send(Msg::Devices(result))
                    .expect("broken channel in devices dialog");
            });
        });

        widgets.error_dlg.connect_close(|dlg| dlg.hide());
        widgets.error_dlg.connect_response(|dlg, _ty| dlg.hide());

        let stream = relm.stream().clone();
        widgets
            .device_list
            .bind_model(Some(&model.devices), move |item| {
                RowWidgets::init(stream.clone(), item)
            });

        Component {
            model,
            widgets,
            sender,
        }
    }
}
