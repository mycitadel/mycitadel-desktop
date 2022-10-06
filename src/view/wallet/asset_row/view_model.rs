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

use glib::subclass::prelude::*;
use gtk::prelude::*;
use gtk::subclass::prelude::ListModelImpl;
use gtk::{gio, glib};

// The actual data structure that stores our values. This is not accessible
// directly from the outside.
#[derive(Default)]
pub struct AssetInner {
    name: RefCell<String>,
    amount: RefCell<String>,
    ticker: RefCell<String>,
    contract: RefCell<String>,
}

// Basic declaration of our type for the GObject type system
#[glib::object_subclass]
impl ObjectSubclass for AssetInner {
    const NAME: &'static str = "AssetInfo";
    type Type = AssetInfo;
    type ParentType = glib::Object;
}

// The ObjectImpl trait provides the setters/getters for GObject properties.
// Here we need to provide the values that are internally stored back to the
// caller, or store whatever new value the caller is providing.
//
// This maps between the GObject properties and our internal storage of the
// corresponding values of the properties.
impl ObjectImpl for AssetInner {
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
                    "amount",
                    "Amount",
                    "Amount",
                    None,
                    glib::ParamFlags::READWRITE,
                ),
                glib::ParamSpecString::new(
                    "ticker",
                    "Ticker",
                    "Ticker",
                    None, // Default value
                    glib::ParamFlags::READWRITE,
                ),
                glib::ParamSpecString::new(
                    "contract",
                    "Contract",
                    "Contract",
                    None, // Default value
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
            "amount" => {
                let amount = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`");
                self.amount.replace(amount);
            }
            "ticker" => {
                let ticker = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`");
                self.ticker.replace(ticker);
            }
            "contract" => {
                let contract = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`");
                self.contract.replace(contract);
            }
            _ => unimplemented!(),
        }
    }

    fn property(&self, _obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        match pspec.name() {
            "name" => self.name.borrow().to_value(),
            "amount" => self.amount.borrow().to_value(),
            "ticker" => self.ticker.borrow().to_value(),
            "contract" => self.contract.borrow().to_value(),
            _ => unimplemented!(),
        }
    }
}

glib::wrapper! {
    pub struct AssetInfo(ObjectSubclass<AssetInner>);
}

impl AssetInfo {
    pub fn with(
        name: &str,
        ticker: &str,
        amount: u64,
        precision: u8,
        contract_name: &str,
    ) -> AssetInfo {
        let precision = precision as u32;
        let amount = amount as f64 / 10_i32.pow(precision) as f64;
        glib::Object::new(&[
            ("name", &name),
            ("amount", &format!("{}", amount)),
            ("ticker", &ticker),
            ("contract", &contract_name),
        ])
        .expect("Failed to create row data")
    }

    pub fn name(&self) -> String { self.property::<String>("name") }

    pub fn ticker(&self) -> String { self.property::<String>("ticker") }

    pub fn contract_name(&self) -> String { self.property::<String>("contract") }

    pub fn amount(&self) -> String { self.property::<String>("amount") }
}

#[derive(Debug, Default)]
pub struct AssetModelInner(pub RefCell<Vec<AssetInfo>>);

/// Basic declaration of our type for the GObject type system
#[glib::object_subclass]
impl ObjectSubclass for AssetModelInner {
    const NAME: &'static str = "AssetModel";
    type Type = AssetModel;
    type ParentType = glib::Object;
    type Interfaces = (gio::ListModel,);
}

impl ObjectImpl for AssetModelInner {}

impl ListModelImpl for AssetModelInner {
    fn item_type(&self, _list_model: &Self::Type) -> glib::Type { AssetInfo::static_type() }
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
    pub struct AssetModel(ObjectSubclass<AssetModelInner>) @implements gio::ListModel;
}

impl AssetModel {
    #[allow(clippy::new_without_default)]
    pub fn new() -> AssetModel { glib::Object::new(&[]).expect("Failed to create AssetModel") }

    pub fn append(&self, obj: &AssetInfo) {
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