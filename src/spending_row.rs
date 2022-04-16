use std::cell::RefCell;
use std::collections::BTreeSet;

use gladis::Gladis;
use glib::subclass::prelude::*;
use gtk::prelude::*;
use gtk::subclass::prelude::ListModelImpl;
use gtk::{
    gio, glib, Adjustment, Calendar, Label, ListBox, ListBoxRow, Menu, MenuButton, RadioMenuItem,
    SpinButton,
};
use relm::StreamHandle;

use crate::types::{Signer, SigsReq, TimelockReq};
use crate::{devices, settings};

// The actual data structure that stores our values. This is not accessible
// directly from the outside.
pub struct ConditionInner {}

impl Default for ConditionInner {
    fn default() -> Self { ConditionInner {} }
}

// Basic declaration of our type for the GObject type system
#[glib::object_subclass]
impl ObjectSubclass for ConditionInner {
    const NAME: &'static str = "Condition";
    type Type = Condition;
    type ParentType = glib::Object;
}

// The ObjectImpl trait provides the setters/getters for GObject properties.
// Here we need to provide the values that are internally stored back to the
// caller, or store whatever new value the caller is providing.
//
// This maps between the GObject properties and our internal storage of the
// corresponding values of the properties.
impl ObjectImpl for ConditionInner {
    fn properties() -> &'static [glib::ParamSpec] {
        use once_cell::sync::Lazy;
        static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| vec![]);

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
            _ => unimplemented!(),
        }
    }

    fn property(&self, _obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        match pspec.name() {
            _ => unimplemented!(),
        }
    }
}

glib::wrapper! {
    pub struct Condition(ObjectSubclass<ConditionInner>);
}

impl Default for Condition {
    fn default() -> Self { glib::Object::new(&[]).expect("Failed to create row data") }
}

#[derive(Debug, Default)]
pub struct SpendingModelInner {
    pub conditions: RefCell<Vec<Condition>>,
    pub max_sigs: RefCell<u16>,
}

/// Basic declaration of our type for the GObject type system
#[glib::object_subclass]
impl ObjectSubclass for SpendingModelInner {
    const NAME: &'static str = "SpendingModel";
    type Type = SpendingModel;
    type ParentType = glib::Object;
    type Interfaces = (gio::ListModel,);
}

impl ObjectImpl for SpendingModelInner {}

impl ListModelImpl for SpendingModelInner {
    fn item_type(&self, _list_model: &Self::Type) -> glib::Type { Condition::static_type() }
    fn n_items(&self, _list_model: &Self::Type) -> u32 { self.conditions.borrow().len() as u32 }
    fn item(&self, _list_model: &Self::Type, position: u32) -> Option<glib::Object> {
        self.conditions
            .borrow()
            .get(position as usize)
            .map(|o| o.clone().upcast::<glib::Object>())
    }
}

// Public part of the Model type.
glib::wrapper! {
    pub struct SpendingModel(ObjectSubclass<SpendingModelInner>) @implements gio::ListModel;
}

impl SpendingModel {
    #[allow(clippy::new_without_default)]
    pub fn new() -> SpendingModel {
        let mut model: SpendingModel =
            glib::Object::new(&[]).expect("Failed to create SpendingModel");
        model.append(&Condition::default());
        model
    }

    pub fn refresh(&self, signers: BTreeSet<Signer>) {
        let imp = self.imp();
        *imp.max_sigs.borrow_mut() = signers.len() as u16;
        // TODO: Update specific conditions
    }

    pub fn append(&self, obj: &Condition) {
        let imp = self.imp();
        let index = {
            // Borrow the data only once and ensure the borrow guard is dropped
            // before we emit the items_changed signal because the view
            // could call get_item / get_n_item from the signal handler to update its state
            let mut data = imp.conditions.borrow_mut();
            data.push(obj.clone());
            data.len() - 1
        };
        // Emits a signal that 1 item was added, 0 removed at the position index
        self.items_changed(index as u32, 0, 1);
    }

    pub fn clear(&self) {
        let imp = self.imp();
        let n = self.n_items();
        imp.conditions.borrow_mut().clear();
        // Emits a signal that 1 item was removed, 0 added at the position index
        for index in 0..n {
            self.items_changed(index, 1, 0);
        }
    }

    pub fn remove(&self, index: u32) {
        let imp = self.imp();
        imp.conditions.borrow_mut().remove(index as usize);
        // Emits a signal that 1 item was removed, 0 added at the position index
        self.items_changed(index, 1, 0);
    }
}

#[derive(Clone, Gladis)]
pub struct RowWidgets {
    pub spending_list: ListBox,
    pub spending_row: ListBoxRow,
    sig_lbl: Label,
    sigs_menu: Menu,
    sigs_all_item: RadioMenuItem,
    sigs_atleast_item: RadioMenuItem,
    sigs_any_item: RadioMenuItem,
    sigs_spin: SpinButton,
    sigs_adj: Adjustment,
    sigtext_lbl: Label,
    lock_mbt: MenuButton,
    lock_menu: Menu,
    lock_anytime_item: RadioMenuItem,
    lock_after_item: RadioMenuItem,
    lock_older_item: RadioMenuItem,
    lock_lbl: Label,
    date_spin: SpinButton,
    date_adj: Adjustment,
    period_mbt: MenuButton,
    period_menu: Menu,
    period_days_item: RadioMenuItem,
    period_weeks_item: RadioMenuItem,
    period_months_item: RadioMenuItem,
    period_years_item: RadioMenuItem,
    period_lbl: Label,
    calendar_mbt: MenuButton,
    calendar_lbl: Label,
    calendar: Calendar,
}

impl RowWidgets {
    pub fn init(stream_: StreamHandle<settings::Msg>, item: &glib::Object) -> gtk::Widget {
        let glade_src = include_str!("../res/spending_row.glade");
        let row_widgets = RowWidgets::from_string(glade_src).expect("glade file broken");

        let condition = item
            .downcast_ref::<Condition>()
            .expect("Row data is of wrong type");
        row_widgets.set_condition(condition);

        /*
        let stream = stream_.clone();
        row_widgets.account_adj.connect_value_changed(move |adj| {
            let account = adj.value() as u32;
            stream.emit(devices::Msg::AccountChange(fingerprint, account))
        });

        let stream = stream_.clone();
        row_widgets.add_btn.connect_clicked(move |_| {
            stream.emit(devices::Msg::Add(fingerprint));
        });
         */

        row_widgets.spending_list.remove(&row_widgets.spending_row);
        row_widgets.spending_row.upcast::<gtk::Widget>()
    }

    pub fn set_condition(&self, condition: &Condition) {
        /*
        device
            .bind_property("name", &self.name_lbl, "label")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .build();
         */
    }
}
