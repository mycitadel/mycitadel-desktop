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

use bitcoin::util::bip32::Fingerprint;
use glib::subclass::prelude::*;
use gtk::prelude::*;
use gtk::subclass::prelude::ListModelImpl;
use gtk::{gio, glib};

// The actual data structure that stores our values. This is not accessible
// directly from the outside.
#[derive(Default)]
pub struct SigningInner {
    name: RefCell<String>,
    master_fp: RefCell<String>,
    status: RefCell<String>,
    sigs_present: RefCell<u32>,
    sigs_required: RefCell<u32>,
    signable: RefCell<bool>,
}

// Basic declaration of our type for the GObject type system
#[glib::object_subclass]
impl ObjectSubclass for SigningInner {
    const NAME: &'static str = "Signing";
    type Type = Signing;
    type ParentType = glib::Object;
}

// The ObjectImpl trait provides the setters/getters for GObject properties.
// Here we need to provide the values that are internally stored back to the
// caller, or store whatever new value the caller is providing.
//
// This maps between the GObject properties and our internal storage of the
// corresponding values of the properties.
impl ObjectImpl for SigningInner {
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
                    "master-fp",
                    "MasterFingerprint",
                    "Fingerprint of the extended master pubkey",
                    None, // Default value
                    glib::ParamFlags::READWRITE,
                ),
                glib::ParamSpecString::new(
                    "status",
                    "Status",
                    "Signing status",
                    Some("unsigned"), // Default value
                    glib::ParamFlags::READWRITE,
                ),
                glib::ParamSpecUInt::new(
                    "sigs-present",
                    "SigsPresent",
                    "Number of signatures present",
                    0,
                    u32::MAX,
                    0, // Allowed range and default value
                    glib::ParamFlags::READWRITE,
                ),
                glib::ParamSpecUInt::new(
                    "sigs-required",
                    "SigsRequired",
                    "Number of signatures required",
                    0,
                    u32::MAX,
                    0, // Allowed range and default value
                    glib::ParamFlags::READWRITE,
                ),
                glib::ParamSpecBoolean::new(
                    "signable",
                    "Signable",
                    "Indicates if the output can be signed",
                    false,
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
                let value = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`");
                self.name.replace(value);
            }
            "master-fp" => {
                let value = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`");
                self.master_fp.replace(value);
            }
            "status" => {
                let value = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`");
                self.status.replace(value);
            }
            "sigs-present" => {
                let value = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`");
                self.sigs_present.replace(value);
            }
            "sigs-required" => {
                let value = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`");
                self.sigs_required.replace(value);
            }
            "signable" => {
                let value = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`");
                self.signable.replace(value);
            }
            _ => unimplemented!(),
        }
    }

    fn property(&self, _obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        match pspec.name() {
            "name" => self.name.borrow().to_value(),
            "master-fp" => self.master_fp.borrow().to_value(),
            "status" => self.status.borrow().to_value(),
            "sigs-present" => self.sigs_present.borrow().to_value(),
            "sigs-required" => self.sigs_required.borrow().to_value(),
            "signable" => self.signable.borrow().to_value(),
            _ => unimplemented!(),
        }
    }
}

glib::wrapper! {
    pub struct Signing(ObjectSubclass<SigningInner>);
}

impl Signing {
    pub fn with(
        name: &str,
        master_fp: Fingerprint,
        sigs_present: u32,
        sigs_required: u32,
    ) -> Signing {
        let status = if sigs_present == 0 {
            s!("unsigned")
        } else if sigs_required >= sigs_present {
            s!("fully signed")
        } else {
            format!("{} of {} signatures", sigs_required, sigs_present)
        };
        glib::Object::new(&[
            ("name", &name),
            ("master-fp", &format!("{}", master_fp)),
            ("status", &status),
            ("sigs-present", &sigs_present),
            ("sigs-required", &sigs_required),
            ("signable", &(sigs_present < sigs_required)),
        ])
        .expect("Failed to create row data")
    }

    pub fn master_fp(&self) -> Fingerprint {
        let fingerprint: String = self.property("master-fp");
        Fingerprint::from_str(&fingerprint).expect("broken fingerprint")
    }

    pub fn name(&self) -> String { self.property("name") }
}

#[derive(Debug, Default)]
pub struct SigningModelInner(pub RefCell<Vec<Signing>>);

/// Basic declaration of our type for the GObject type system
#[glib::object_subclass]
impl ObjectSubclass for SigningModelInner {
    const NAME: &'static str = "SigningModel";
    type Type = SigningModel;
    type ParentType = glib::Object;
    type Interfaces = (gio::ListModel,);
}

impl ObjectImpl for SigningModelInner {}

impl ListModelImpl for SigningModelInner {
    fn item_type(&self, _list_model: &Self::Type) -> glib::Type { Signing::static_type() }
    fn n_items(&self, _list_model: &Self::Type) -> u32 { self.0.borrow().len() as u32 }
    fn item(&self, _list_model: &Self::Type, position: u32) -> Option<glib::Object> {
        self.0
            .borrow()
            .get(position as usize)
            .map(|o| o.clone().upcast::<glib::Object>())
    }
}

// Public part of the Model type.
glib::wrapper! {
    pub struct SigningModel(ObjectSubclass<SigningModelInner>) @implements gio::ListModel;
}

impl Default for SigningModel {
    fn default() -> Self { SigningModel::new() }
}

impl SigningModel {
    pub fn new() -> SigningModel { glib::Object::new(&[]).expect("Failed to create SigningModel") }

    pub fn append(&self, obj: &Signing) {
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
