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

use gtk::glib::translate::IntoGlib;
use gtk::glib::ToValue;
use gtk::subclass::prelude::{ObjectImpl, ObjectSubclass};
use gtk::{glib, MessageType};
use std::cell::RefCell;
use wallet::slip132::KeyApplication;

struct XpubModelInner {
    xpub: RefCell<String>,
    message: RefCell<String>,
    message_type: RefCell<MessageType>,
}

impl Default for XpubModelInner {
    fn default() -> Self {
        XpubModelInner {
            xpub: RefCell::new("".to_string()),
            message: RefCell::new("".to_string()),
            message_type: RefCell::new(MessageType::Info),
        }
    }
}

// Basic declaration of our type for the GObject type system
#[glib::object_subclass]
impl ObjectSubclass for XpubModelInner {
    const NAME: &'static str = "XpubModel";
    type Type = XpubModel;
    type ParentType = glib::Object;
}

impl ObjectImpl for XpubModelInner {
    fn properties() -> &'static [glib::ParamSpec] {
        use once_cell::sync::Lazy;
        static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
            let flag = glib::ParamFlags::READWRITE;
            vec![
                glib::ParamSpecString::new("xpub", "xpub", "xpub", None, flag),
                glib::ParamSpecString::new("message", "message", "message", None, flag),
                glib::ParamSpecEnum::new(
                    "message-type",
                    "messageType",
                    "messageType",
                    glib::Type::U32,
                    MessageType::Error.into_glib(),
                    flag,
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
            "xpub" => {
                let value = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`");
                self.xpub.replace(value);
            }
            _ => unimplemented!(),
        }
    }

    fn property(&self, _obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        match pspec.name() {
            "xpub" => self.xpub.borrow().to_value(),
            _ => unimplemented!(),
        }
    }
}

glib::wrapper! {
    pub struct XpubModel(ObjectSubclass<XpubModelInner>);
}

impl Default for XpubModel {
    fn default() -> Self {
        glib::Object::new(&[]).expect("Failed to create row data")
    }
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub struct ModelParam {
    pub testnet: bool,
    pub slip_format: KeyApplication,
}

pub struct ViewModel {
    pub config: ModelParam,
    pub xpub_model: XpubModel,
}
