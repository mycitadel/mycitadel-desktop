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
use gtk::glib::Binding;
use gtk::prelude::*;
use gtk::{
    glib, Adjustment, Calendar, Label, ListBoxRow, Menu, MenuButton, Popover, RadioMenuItem,
    SpinButton,
};
use relm::Sender;

use super::Condition;

#[derive(Clone, Gladis)]
pub struct RowWidgets {
    spending_row: ListBoxRow,
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
    calendar_popover: Popover,
}

impl RowWidgets {
    pub fn init(sender: Sender<()>, item: &glib::Object) -> gtk::Widget {
        let glade_src = include_str!("spending_row.glade");
        let row_widgets = RowWidgets::from_string(glade_src).expect("glade file broken");

        let condition = item
            .downcast_ref::<Condition>()
            .expect("Row data is of wrong type");
        row_widgets.bind_model(condition);

        // We need this hack since mpsc::Sender does not implement Sync, as required by the
        // glib event handler. However, since we `move` the object anyway, we do not need it to be
        // Sync, and use workaround with unsafe call.
        unsafe {
            condition.connect_notify_unsafe(None, move |_, _| {
                sender
                    .send(())
                    .expect("channel to settings window is broken");
            })
        };

        let c = condition.clone();
        row_widgets
            .sigs_all_item
            .connect_toggled(move |mi| c.set_property("sigs-all", mi.is_active()));
        let c = condition.clone();
        row_widgets
            .sigs_atleast_item
            .connect_toggled(move |mi| c.set_property("sigs-at-least", mi.is_active()));
        let c = condition.clone();
        row_widgets
            .sigs_any_item
            .connect_toggled(move |mi| c.set_property("sigs-any", mi.is_active()));

        let c = condition.clone();
        row_widgets
            .lock_anytime_item
            .connect_toggled(move |mi| c.set_property("lock-none", mi.is_active()));
        let c = condition.clone();
        row_widgets
            .lock_after_item
            .connect_toggled(move |mi| c.set_property("lock-after", mi.is_active()));
        let c = condition.clone();
        row_widgets
            .lock_older_item
            .connect_toggled(move |mi| c.set_property("lock-older", mi.is_active()));

        let c = condition.clone();
        row_widgets
            .period_years_item
            .connect_toggled(move |mi| c.set_property("period-years", mi.is_active()));
        let c = condition.clone();
        row_widgets
            .period_months_item
            .connect_toggled(move |mi| c.set_property("period-months", mi.is_active()));
        let c = condition.clone();
        row_widgets
            .period_weeks_item
            .connect_toggled(move |mi| c.set_property("period-weeks", mi.is_active()));
        let c = condition.clone();
        row_widgets
            .period_days_item
            .connect_toggled(move |mi| c.set_property("period-days", mi.is_active()));

        let c = condition.clone();
        row_widgets
            .calendar
            .connect_day_selected_double_click(move |cal| {
                c.set_property("after-day", cal.day() as u32);
                c.set_property("after-month", cal.month() as u32 + 1);
                c.set_property("after-year", cal.year() as u32);
                row_widgets.calendar_popover.hide();
            });

        row_widgets.spending_row.upcast::<gtk::Widget>()
    }

    fn bind_model(&self, condition: &Condition) {
        let flags_ro = glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE;
        let flags_rw = glib::BindingFlags::DEFAULT
            | glib::BindingFlags::SYNC_CREATE
            | glib::BindingFlags::BIDIRECTIONAL;
        condition
            .bind_property("sigs-all", &self.sigs_all_item, "active")
            .flags(flags_ro)
            .build();
        condition
            .bind_property("sigs-any", &self.sigs_any_item, "active")
            .flags(flags_ro)
            .build();
        condition
            .bind_property("sigs-at-least", &self.sigs_atleast_item, "active")
            .flags(flags_ro)
            .build();
        condition
            .bind_property("sigs-no", &self.sigs_adj, "value")
            .flags(flags_rw)
            .build();
        condition
            .bind_property("sigs-at-least", &self.sigs_spin, "visible")
            .flags(flags_ro)
            .build();
        condition
            .bind_property("sigs-at-least", &self.sigtext_lbl, "visible")
            .flags(flags_ro)
            .build();
        condition
            .bind_property("sigs-all", &self.sig_lbl, "label")
            .flags(flags_ro)
            .transform_to(|_, val| {
                if val.get().unwrap() {
                    Some("All signatures".to_value())
                } else {
                    None
                }
            })
            .build();
        condition
            .bind_property("sigs-any", &self.sig_lbl, "label")
            .flags(flags_ro)
            .transform_to(|_, val| {
                if val.get().unwrap() {
                    Some("Any signature".to_value())
                } else {
                    None
                }
            })
            .build();
        condition
            .bind_property("sigs-at-least", &self.sig_lbl, "label")
            .flags(flags_ro)
            .transform_to(|_, val| {
                if val.get().unwrap() {
                    Some("At least".to_value())
                } else {
                    None
                }
            })
            .build();

        condition
            .bind_property("lock-none", &self.lock_anytime_item, "active")
            .flags(flags_ro)
            .build();
        condition
            .bind_property("lock-after", &self.lock_after_item, "active")
            .flags(flags_ro)
            .build();
        condition
            .bind_property("lock-older", &self.lock_older_item, "active")
            .flags(flags_ro)
            .build();
        condition
            .bind_property("lock-older", &self.calendar_mbt, "visible")
            .flags(flags_ro)
            .build();
        condition
            .bind_property("lock-after", &self.date_spin, "visible")
            .flags(flags_ro)
            .build();
        condition
            .bind_property("lock-after", &self.period_mbt, "visible")
            .flags(flags_ro)
            .build();
        condition
            .bind_property("lock-none", &self.lock_lbl, "label")
            .flags(flags_ro)
            .transform_to(|_, val| {
                if val.get().unwrap() {
                    Some("at any time".to_value())
                } else {
                    None
                }
            })
            .build();
        condition
            .bind_property("lock-after", &self.lock_lbl, "label")
            .flags(flags_ro)
            .transform_to(|_, val| {
                if val.get().unwrap() {
                    Some("after".to_value())
                } else {
                    None
                }
            })
            .build();
        condition
            .bind_property("lock-older", &self.lock_lbl, "label")
            .flags(flags_ro)
            .transform_to(|_, val| {
                if val.get().unwrap() {
                    Some("starting from".to_value())
                } else {
                    None
                }
            })
            .build();

        condition
            .bind_property("period-years", &self.period_years_item, "active")
            .flags(flags_ro)
            .build();
        condition
            .bind_property("period-months", &self.period_months_item, "active")
            .flags(flags_ro)
            .build();
        condition
            .bind_property("period-weeks", &self.period_weeks_item, "active")
            .flags(flags_ro)
            .build();
        condition
            .bind_property("period-days", &self.period_days_item, "active")
            .flags(flags_ro)
            .build();
        condition
            .bind_property("period-span", &self.date_adj, "value")
            .flags(flags_rw)
            .build();
        condition
            .bind_property("period-years", &self.period_lbl, "label")
            .flags(flags_ro)
            .transform_to(|_, val| {
                if val.get().unwrap() {
                    Some("year(s)".to_value())
                } else {
                    None
                }
            })
            .build();
        condition
            .bind_property("period-weeks", &self.period_lbl, "label")
            .flags(flags_ro)
            .transform_to(|_, val| {
                if val.get().unwrap() {
                    Some("week(s)".to_value())
                } else {
                    None
                }
            })
            .build();
        condition
            .bind_property("period-months", &self.period_lbl, "label")
            .flags(flags_ro)
            .transform_to(|_, val| {
                if val.get().unwrap() {
                    Some("month(s)".to_value())
                } else {
                    None
                }
            })
            .build();
        condition
            .bind_property("period-days", &self.period_lbl, "label")
            .flags(flags_ro)
            .transform_to(|_, val| {
                if val.get().unwrap() {
                    Some("day(s)".to_value())
                } else {
                    None
                }
            })
            .build();

        condition
            .bind_property("after-day", &self.calendar, "day")
            .flags(flags_ro)
            .build();
        condition
            .bind_property("after-month", &self.calendar, "month")
            .transform_to(|_, value| {
                let month: u32 = value.get().unwrap();
                let month = month as i32 - 1;
                Some(month.to_value())
            })
            .flags(flags_ro)
            .build();
        condition
            .bind_property("after-year", &self.calendar, "year")
            .flags(flags_ro)
            .build();

        let fmtdate = |binding: &Binding, _: &glib::Value| -> Option<glib::Value> {
            let year: u32 = binding.source().unwrap().property("after-year");
            let month: u32 = binding.source().unwrap().property("after-month");
            let day: u32 = binding.source().unwrap().property("after-day");
            Some(format!("{}/{}/{}", year, month, day).to_value())
        };

        condition
            .bind_property("after-day", &self.calendar_lbl, "label")
            .flags(flags_ro)
            .transform_to(fmtdate.clone())
            .build();
        condition
            .bind_property("after-month", &self.calendar_lbl, "label")
            .flags(flags_ro)
            .transform_to(fmtdate.clone())
            .build();
        condition
            .bind_property("after-year", &self.calendar_lbl, "label")
            .flags(flags_ro)
            .transform_to(fmtdate)
            .build();
    }
}
