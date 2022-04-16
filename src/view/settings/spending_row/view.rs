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
use gtk::{
    glib, Adjustment, Calendar, Label, ListBox, ListBoxRow, Menu, MenuButton, RadioMenuItem,
    SpinButton,
};
use relm::StreamHandle;

use super::Condition;
use crate::view::settings;

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
        let glade_src = include_str!("spending_row.glade");
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
