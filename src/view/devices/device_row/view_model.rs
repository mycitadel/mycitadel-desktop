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

use std::cell::RefCell;
use std::str::FromStr;

use bitcoin::secp256k1::PublicKey;
use bitcoin::util::bip32::{ChainCode, ChildNumber, ExtendedPubKey, Fingerprint};
use bitcoin::{secp256k1, Network};
use bpro::HardwareList;
use glib::subclass::prelude::*;
use gtk::prelude::*;
use gtk::subclass::prelude::ListModelImpl;
use gtk::{gio, glib};
use wallet::hd::SegmentIndexes;

// The actual data structure that stores our values. This is not accessible
// directly from the outside.
pub struct DeviceDataInner {
    pub name: RefCell<String>,
    pub fingerprint: RefCell<String>,
    pub xpub: RefCell<String>,
    pub account_no: RefCell<u32>,
    pub updating: RefCell<bool>,
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
            updating: RefCell::new(false),
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
                glib::ParamSpecBoolean::new(
                    "updating",
                    "Updating",
                    "Updating",
                    false,
                    glib::ParamFlags::READWRITE,
                ),
            ]
        });

        PROPERTIES.as_ref()
    }

    fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
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
            "updating" => {
                let updating = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`");
                self.updating.replace(updating);
            }
            _ => unimplemented!(),
        }
    }

    fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        match pspec.name() {
            "name" => self.name.borrow().to_value(),
            "fingerprint" => self.fingerprint.borrow().to_value(),
            "xpub" => self.xpub.borrow().to_value(),
            "account" => self.account_no.borrow().to_value(),
            "updating" => self.updating.borrow().to_value(),
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
    }

    pub fn fingerprint(&self) -> Fingerprint {
        Fingerprint::from_str(&self.property::<String>("fingerprint"))
            .expect("device fingerprint failure")
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
    fn item_type(&self) -> glib::Type { DeviceData::static_type() }
    fn n_items(&self) -> u32 { self.0.borrow().len() as u32 }
    fn item(&self, position: u32) -> Option<glib::Object> {
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
    pub fn new() -> DeviceModel { glib::Object::new(&[]) }

    pub fn refresh(&self, devices: &HardwareList) {
        self.clear();
        for (fingerprint, device) in devices {
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
        self.items_changed(0, n, 0);
    }

    pub fn remove(&self, index: u32) {
        let imp = self.imp();
        imp.0.borrow_mut().remove(index as usize);
        // Emits a signal that 1 item was removed, 0 added at the position index
        self.items_changed(index, 1, 0);
    }
}
