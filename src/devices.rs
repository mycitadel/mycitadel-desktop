use std::cell::RefCell;

use bitcoin::secp256k1::PublicKey;
use bitcoin::util::bip32::{ChainCode, ChildNumber, ExtendedPubKey, Fingerprint};
use bitcoin::{secp256k1, Network};

use gladis::Gladis;
use glib::subclass::prelude::*;
use gtk::prelude::*;
use gtk::subclass::prelude::ListModelImpl;
use gtk::{
    gio, glib, Adjustment, Button, Dialog, Label, ListBox, ListBoxRow, MessageDialog, Spinner,
    ToolButton,
};
use relm::{Channel, Relm, Update, Widget};
use wallet::hd::{DerivationScheme, HardenedIndex, SegmentIndexes};

use crate::settings::{Error, HardwareList, PublicNetwork};

// The actual data structure that stores our values. This is not accessible
// directly from the outside.
pub struct DeviceDataInner {
    pub name: RefCell<String>,
    pub fingerprint: RefCell<String>,
    pub xpub: RefCell<String>,
    pub account_no: RefCell<u32>,
}

impl Default for DeviceDataInner {
    fn default() -> Self {
        let public_key = PublicKey::from_secret_key_global(&secp256k1::ONE_KEY);
        DeviceDataInner {
            name: RefCell::new("".to_string()),
            fingerprint: RefCell::new(Default::default()),
            xpub: RefCell::new(
                ExtendedPubKey {
                    network: Network::Bitcoin,
                    depth: 0,
                    parent_fingerprint: Default::default(),
                    child_number: ChildNumber::from_hardened_idx(0)
                        .expect("hardcoded hardened index"),
                    public_key,
                    chain_code: ChainCode::from(&[0u8; 32][..]),
                }
                .to_string(),
            ),
            account_no: RefCell::new(0),
        }
    }
}

// Basic declaration of our type for the GObject type system
#[glib::object_subclass]
impl ObjectSubclass for DeviceDataInner {
    const NAME: &'static str = "Device";
    type Type = DeviceData;
    type ParentType = glib::Object;
}

// The ObjectImpl trait provides the setters/getters for GObject properties.
// Here we need to provide the values that are internally stored back to the
// caller, or store whatever new value the caller is providing.
//
// This maps between the GObject properties and our internal storage of the
// corresponding values of the properties.
impl ObjectImpl for DeviceDataInner {
    fn properties() -> &'static [glib::ParamSpec] {
        use once_cell::sync::Lazy;
        static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
            vec![
                glib::ParamSpecString::new(
                    "name",
                    "Name",
                    "Name",
                    None, // Default value
                    glib::ParamFlags::READWRITE,
                ),
                glib::ParamSpecString::new(
                    "fingerprint",
                    "Fingerprint",
                    "Fingerprint",
                    None,
                    glib::ParamFlags::READWRITE,
                ),
                glib::ParamSpecString::new(
                    "xpub",
                    "XPub",
                    "XPub",
                    None,
                    glib::ParamFlags::READWRITE,
                ),
                glib::ParamSpecUInt::new(
                    "account",
                    "Account",
                    "Account",
                    0,
                    u32::MAX / 2 - 1,
                    0, // Allowed range and default value
                    glib::ParamFlags::READWRITE,
                ),
            ]
        });

        PROPERTIES.as_ref()
    }

    fn set_property(
        &self,
        _obj: &Self::Type,
        _id: usize,
        value: &glib::Value,
        pspec: &glib::ParamSpec,
    ) {
        match pspec.name() {
            "name" => {
                let name = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`");
                self.name.replace(name);
            }
            "fingerprint" => {
                let fingerprint = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`");
                self.fingerprint.replace(fingerprint);
            }
            "xpub" => {
                let xpub = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`");
                self.xpub.replace(xpub);
            }
            "account" => {
                let account_no = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`");
                self.account_no.replace(account_no);
            }
            _ => unimplemented!(),
        }
    }

    fn property(&self, _obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        match pspec.name() {
            "name" => self.name.borrow().to_value(),
            "fingerprint" => self.fingerprint.borrow().to_value(),
            "xpub" => self.xpub.borrow().to_value(),
            "account" => self.account_no.borrow().to_value(),
            _ => unimplemented!(),
        }
    }
}

glib::wrapper! {
    pub struct DeviceData(ObjectSubclass<DeviceDataInner>);
}

impl DeviceData {
    pub fn new(
        name: &str,
        fingerprint: &Fingerprint,
        xpub: &ExtendedPubKey,
        account: u32,
    ) -> DeviceData {
        glib::Object::new(&[
            ("name", &name),
            ("fingerprint", &fingerprint.to_string()),
            ("xpub", &xpub.to_string()),
            ("account", &account),
        ])
        .expect("Failed to create row data")
    }
}

#[derive(Debug, Default)]
pub struct DeviceModelInner(pub RefCell<Vec<DeviceData>>);

/// Basic declaration of our type for the GObject type system
#[glib::object_subclass]
impl ObjectSubclass for DeviceModelInner {
    const NAME: &'static str = "DeviceModel";
    type Type = DeviceModel;
    type ParentType = glib::Object;
    type Interfaces = (gio::ListModel,);
}

impl ObjectImpl for DeviceModelInner {}

impl ListModelImpl for DeviceModelInner {
    fn item_type(&self, _list_model: &Self::Type) -> glib::Type {
        DeviceData::static_type()
    }
    fn n_items(&self, _list_model: &Self::Type) -> u32 {
        self.0.borrow().len() as u32
    }
    fn item(&self, _list_model: &Self::Type, position: u32) -> Option<glib::Object> {
        self.0
            .borrow()
            .get(position as usize)
            .map(|o| o.clone().upcast::<glib::Object>())
    }
}

// Public part of the Model type.
glib::wrapper! {
    pub struct DeviceModel(ObjectSubclass<DeviceModelInner>) @implements gio::ListModel;
}

impl DeviceModel {
    #[allow(clippy::new_without_default)]
    pub fn new() -> DeviceModel {
        glib::Object::new(&[]).expect("Failed to create DeviceModel")
    }

    pub fn refresh(&self, devices: HardwareList) {
        self.clear();
        for (fingerprint, device) in &devices {
            let data = DeviceData::new(
                &device.model,
                fingerprint,
                &device.default_xpub,
                device.default_account.first_index(),
            );
            self.append(&data);
        }
    }

    pub fn append(&self, obj: &DeviceData) {
        let imp = self.imp();
        let index = {
            // Borrow the data only once and ensure the borrow guard is dropped
            // before we emit the items_changed signal because the view
            // could call get_item / get_n_item from the signal handler to update its state
            let mut data = imp.0.borrow_mut();
            data.push(obj.clone());
            data.len() - 1
        };
        // Emits a signal that 1 item was added, 0 removed at the position index
        self.items_changed(index as u32, 0, 1);
    }

    pub fn clear(&self) {
        let imp = self.imp();
        let n = self.n_items();
        imp.0.borrow_mut().clear();
        // Emits a signal that 1 item was removed, 0 added at the position index
        for index in 0..n {
            self.items_changed(index, 1, 0);
        }
    }

    pub fn remove(&self, index: u32) {
        let imp = self.imp();
        imp.0.borrow_mut().remove(index as usize);
        // Emits a signal that 1 item was removed, 0 added at the position index
        self.items_changed(index, 1, 0);
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct Model {
    pub scheme: DerivationScheme,
    pub network: PublicNetwork,
    pub devices: DeviceModel,
}

impl Model {}

#[derive(Msg, Debug)]
pub(crate) enum Msg {
    Show,
    Refresh,
    Devices(Result<(HardwareList, Vec<Error>), Error>),
    AccountChange(u32),
    Add,
    Close,
}

#[derive(Clone, Gladis)]
struct RowWidgets {
    pub device_list: ListBox,
    pub device_row: ListBoxRow,
    name_lbl: Label,
    fingerprint_lbl: Label,
    xpub_lbl: Label,
    spinner: Spinner,
    account_adj: Adjustment,
    add_btn: Button,
}

impl RowWidgets {
    pub fn set_device(&self, device: &DeviceData) {
        device
            .bind_property("name", &self.name_lbl, "label")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .build();
        device
            .bind_property("fingerprint", &self.fingerprint_lbl, "label")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .build();
        device
            .bind_property("xpub", &self.xpub_lbl, "label")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .build();
    }
}

#[derive(Clone, Gladis)]
pub(crate) struct Widgets {
    dialog: Dialog,
    close_btn: Button,
    refresh_btn: Button,
    refresh_dlg: Dialog,
    device_list: ListBox,
    error_dlg: MessageDialog,
}

pub(crate) struct Win {
    model: Model,
    channel: Channel<Msg>,
    widgets: Widgets,
}

impl Update for Win {
    // Specify the model used for this widget.
    type Model = Model;
    // Specify the model parameter used to init the model.
    type ModelParam = (DerivationScheme, PublicNetwork);
    // Specify the type of the messages sent to the update function.
    type Msg = Msg;

    fn model(_relm: &Relm<Self>, model: Self::ModelParam) -> Self::Model {
        Model {
            scheme: model.0,
            network: model.1,
            devices: DeviceModel::new(),
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::Show => self.widgets.dialog.show(),
            Msg::Refresh => self.widgets.refresh_dlg.show(),
            Msg::Devices(result) => {
                self.widgets.refresh_dlg.hide();
                let devices = match result {
                    Err(err) => {
                        self.widgets
                            .error_dlg
                            .set_secondary_text(Some(&err.to_string()));
                        self.widgets.error_dlg.show();
                        HardwareList::default()
                    }
                    Ok((devices, log)) if !log.is_empty() => {
                        // TODO: Display log and do not hide the window
                        devices
                    }
                    Ok((devices, _log)) if devices.is_empty() => {
                        // TODO: Display message to user
                        devices
                    }
                    Ok((devices, _)) => devices,
                };
                self.model.devices.refresh(devices);
            }
            Msg::AccountChange(_) => {}
            Msg::Add => {}
            Msg::Close => {
                self.widgets.dialog.hide();
            }
        }
    }
}

impl Widget for Win {
    // Specify the type of the root widget.
    type Root = Dialog;

    // Return the root widget.
    fn root(&self) -> Self::Root {
        self.widgets.dialog.clone()
    }

    fn view(relm: &Relm<Self>, model: Self::Model) -> Self {
        let glade_src = include_str!("../res/devices.glade");
        let widgets = Widgets::from_string(glade_src).expect("glade file broken");

        connect!(relm, widgets.close_btn, connect_clicked(_), Msg::Close);

        let stream = relm.stream().clone();
        let (channel, sender) = Channel::new(move |msg| {
            stream.emit(msg);
        });
        let scheme = model.scheme.clone();
        widgets.refresh_btn.connect_clicked(move |_| {
            sender
                .send(Msg::Refresh)
                .expect("broken channel in devices dialog");
            // TODO: This fixes the schema used in the wallet once and forever
            let scheme = scheme.clone();
            let sender = sender.clone();
            std::thread::spawn(move || {
                let result = HardwareList::enumerate(&scheme, model.network, HardenedIndex::zero());
                sender
                    .send(Msg::Devices(result))
                    .expect("broken channel in devices dialog");
            });
        });

        widgets
            .device_list
            .bind_model(Some(&model.devices), move |item| {
                let glade_src = include_str!("../res/device_row.glade");
                let device_row = RowWidgets::from_string(glade_src).expect("glade file broken");
                let device = item
                    .downcast_ref::<DeviceData>()
                    .expect("Row data is of wrong type");
                device_row.set_device(device);
                device_row.device_list.remove(&device_row.device_row);
                device_row.device_row.upcast::<gtk::Widget>()
            });

        Win {
            model,
            widgets,
            channel,
        }
    }
}
