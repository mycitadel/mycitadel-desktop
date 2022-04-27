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

use bitcoin::util::address;
use bitcoin::Address;
use std::cell::RefCell;
use std::str::FromStr;

use glib::subclass::prelude::*;
use gtk::prelude::*;
use gtk::subclass::prelude::ListModelImpl;
use gtk::{gio, glib};
use wallet::scripts::address::AddressCompat;

// The actual data structure that stores our values. This is not accessible
// directly from the outside.
#[derive(Default)]
pub struct BeneficiaryInner {
    address: RefCell<String>,
    amount: RefCell<u64>,
}

// Basic declaration of our type for the GObject type system
#[glib::object_subclass]
impl ObjectSubclass for BeneficiaryInner {
    const NAME: &'static str = "Beneficiary";
    type Type = Beneficiary;
    type ParentType = glib::Object;
}

// The ObjectImpl trait provides the setters/getters for GObject properties.
// Here we need to provide the values that are internally stored back to the
// caller, or store whatever new value the caller is providing.
//
// This maps between the GObject properties and our internal storage of the
// corresponding values of the properties.
impl ObjectImpl for BeneficiaryInner {
    fn properties() -> &'static [glib::ParamSpec] {
        use once_cell::sync::Lazy;
        static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
            vec![
                glib::ParamSpecString::new(
                    "address",
                    "Address",
                    "Address",
                    None, // Default value
                    glib::ParamFlags::READWRITE,
                ),
                glib::ParamSpecUInt64::new(
                    "amount",
                    "Amount",
                    "Amount",
                    0,
                    21_000_000 * 100_000_00,
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
            "address" => {
                let address = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`");
                self.address.replace(address);
            }
            "amount" => {
                let amount = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`");
                self.amount.replace(amount);
            }
            _ => unimplemented!(),
        }
    }

    fn property(&self, _obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        match pspec.name() {
            "address" => self.address.borrow().to_value(),
            "amount" => self.amount.borrow().to_value(),
            _ => unimplemented!(),
        }
    }
}

glib::wrapper! {
    pub struct Beneficiary(ObjectSubclass<BeneficiaryInner>);
}

impl Beneficiary {
    pub fn new() -> Beneficiary {
        glib::Object::new(&[("address", &""), ("amount", &0u64)])
            .expect("Failed to create row data")
    }

    pub fn with(address: AddressCompat, amount: u64) -> Beneficiary {
        glib::Object::new(&[("address", &address.to_string()), ("amount", &amount)])
            .expect("Failed to create row data")
    }

    pub fn address(&self) -> Result<Address, address::Error> {
        Address::from_str(&self.property::<String>("address"))
    }

    pub fn address_compat(&self) -> Result<AddressCompat, address::Error> {
        AddressCompat::from_str(&self.property::<String>("address"))
    }
}

#[derive(Debug, Default)]
pub struct BeneficiaryModelInner(pub RefCell<Vec<Beneficiary>>);

/// Basic declaration of our type for the GObject type system
#[glib::object_subclass]
impl ObjectSubclass for BeneficiaryModelInner {
    const NAME: &'static str = "BeneficiaryModel";
    type Type = BeneficiaryModel;
    type ParentType = glib::Object;
    type Interfaces = (gio::ListModel,);
}

impl ObjectImpl for BeneficiaryModelInner {}

impl ListModelImpl for BeneficiaryModelInner {
    fn item_type(&self, _list_model: &Self::Type) -> glib::Type {
        Beneficiary::static_type()
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
    pub struct BeneficiaryModel(ObjectSubclass<BeneficiaryModelInner>) @implements gio::ListModel;
}

impl BeneficiaryModel {
    #[allow(clippy::new_without_default)]
    pub fn new() -> BeneficiaryModel {
        glib::Object::new(&[]).expect("Failed to create BeneficiaryModel")
    }

    pub fn append(&self, obj: &Beneficiary) {
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
