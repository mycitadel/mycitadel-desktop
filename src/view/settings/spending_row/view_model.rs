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

use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::BTreeSet;

use bpro::{Signer, SigsReq, SpendingCondition, TimelockDuration, TimelockReq, TimelockedSigs};
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use glib::subclass::prelude::*;
use gtk::prelude::*;
use gtk::subclass::prelude::ListModelImpl;
use gtk::{gio, glib};

// The actual data structure that stores our values. This is not accessible
// directly from the outside.
#[derive(Debug)]
pub struct ConditionInner {
    sigs_all: RefCell<bool>,
    sigs_at_least: RefCell<bool>,
    sigs_any: RefCell<bool>,
    sigs_no: RefCell<u32>,
    lock_none: RefCell<bool>,
    lock_older: RefCell<bool>,
    lock_after: RefCell<bool>,
    period_years: RefCell<bool>,
    period_months: RefCell<bool>,
    period_weeks: RefCell<bool>,
    period_days: RefCell<bool>,
    period_span: RefCell<u32>,
    after_day: RefCell<u32>,
    after_month: RefCell<u32>,
    after_year: RefCell<u32>,
}

impl Default for ConditionInner {
    fn default() -> Self {
        ConditionInner {
            sigs_all: RefCell::new(true),
            sigs_at_least: RefCell::new(false),
            sigs_any: RefCell::new(false),
            sigs_no: RefCell::new(2),
            lock_none: RefCell::new(true),
            lock_older: RefCell::new(false),
            lock_after: RefCell::new(false),
            period_years: RefCell::new(true),
            period_months: RefCell::new(false),
            period_weeks: RefCell::new(false),
            period_days: RefCell::new(false),
            period_span: RefCell::new(1),
            after_day: RefCell::new(1),
            after_month: RefCell::new(1),
            after_year: RefCell::new(2025),
        }
    }
}

impl From<&ConditionInner> for TimelockReq {
    fn from(inner: &ConditionInner) -> Self {
        match (
            *inner.lock_none.borrow(),
            *inner.lock_after.borrow(),
            *inner.lock_older.borrow(),
        ) {
            (true, false, false) => TimelockReq::Anytime,
            (_, true, false) => {
                let date = NaiveDate::from_ymd(
                    *inner.after_year.borrow() as i32,
                    *inner.after_month.borrow(),
                    *inner.after_day.borrow(),
                );
                TimelockReq::AfterDate(DateTime::from_utc(date.and_hms(0, 0, 0), Utc))
            }
            (_, _, true) => {
                let offset = *inner.period_span.borrow();
                let duration = match (
                    *inner.period_years.borrow(),
                    *inner.period_months.borrow(),
                    *inner.period_weeks.borrow(),
                    *inner.period_days.borrow(),
                ) {
                    (true, false, false, false) => TimelockDuration::Years(offset as u8),
                    (_, true, false, false) => TimelockDuration::Months(offset as u8),
                    (_, _, true, false) => TimelockDuration::Weeks(offset as u8),
                    (_, _, _, true) => TimelockDuration::Days(offset as u8),
                    _ => unreachable!(
                        "ConditionInner internal inconsistency in relative timelock \
                         requirements\n{:#?}",
                        inner
                    ),
                };
                TimelockReq::AfterPeriod(duration)
            }
            _ => unreachable!(
                "ConditionInner internal inconsistency in timelock requirements\n{:#?}",
                inner
            ),
        }
    }
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
        static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
            let flag = glib::ParamFlags::READWRITE;
            vec![
                glib::ParamSpecBoolean::new("sigs-all", "SigsAll", "SigsAll", true, flag),
                glib::ParamSpecBoolean::new(
                    "sigs-at-least",
                    "SigsAtLeast",
                    "SigsAtLeast",
                    false,
                    flag,
                ),
                glib::ParamSpecBoolean::new("sigs-any", "SigsAny", "SigsAny", false, flag),
                glib::ParamSpecUInt::new("sigs-no", "SigsNo", "SigsNo", 1, 100, 2, flag),
                glib::ParamSpecBoolean::new("lock-none", "LockNone", "LockNone", true, flag),
                glib::ParamSpecBoolean::new("lock-after", "LockAfter", "LockAfter", false, flag),
                glib::ParamSpecBoolean::new("lock-older", "LockOlder", "LockOlder", false, flag),
                glib::ParamSpecBoolean::new(
                    "period-years",
                    "PeriodYears",
                    "PeriodYears",
                    true,
                    flag,
                ),
                glib::ParamSpecBoolean::new(
                    "period-months",
                    "PeriodMonths",
                    "PeriodMonths",
                    false,
                    flag,
                ),
                glib::ParamSpecBoolean::new(
                    "period-weeks",
                    "PeriodWeeks",
                    "PeriodWeeks",
                    false,
                    flag,
                ),
                glib::ParamSpecBoolean::new("period-days", "PeriodDays", "PeriodDays", false, flag),
                glib::ParamSpecUInt::new(
                    "period-span",
                    "PeriodSpan",
                    "PeriodSpan",
                    1,
                    100,
                    1,
                    flag,
                ),
                glib::ParamSpecUInt::new("after-day", "AfterDay", "AfterDay", 1, 31, 1, flag),
                glib::ParamSpecUInt::new("after-month", "AfterMonth", "AfterMonth", 1, 12, 1, flag),
                glib::ParamSpecUInt::new(
                    "after-year",
                    "AfterYear",
                    "AfterYear",
                    2022,
                    2222,
                    2025,
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
            "sigs-all" => {
                let value = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`");
                self.sigs_all.replace(value);
            }
            "sigs-at-least" => {
                let value = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`");
                self.sigs_at_least.replace(value);
            }
            "sigs-any" => {
                let value = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`");
                self.sigs_any.replace(value);
            }
            "sigs-no" => {
                let value = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`");
                self.sigs_no.replace(value);
            }
            "lock-none" => {
                let value = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`");
                self.lock_none.replace(value);
            }
            "lock-after" => {
                let value = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`");
                self.lock_after.replace(value);
            }
            "lock-older" => {
                let value = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`");
                self.lock_older.replace(value);
            }
            "period-years" => {
                let value = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`");
                self.period_years.replace(value);
            }
            "period-months" => {
                let value = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`");
                self.period_months.replace(value);
            }
            "period-weeks" => {
                let value = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`");
                self.period_weeks.replace(value);
            }
            "period-days" => {
                let value = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`");
                self.period_days.replace(value);
            }
            "period-span" => {
                let value = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`");
                self.period_span.replace(value);
            }
            "after-day" => {
                let value = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`");
                self.after_day.replace(value);
            }
            "after-month" => {
                let value = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`");
                self.after_month.replace(value);
            }
            "after-year" => {
                let value = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`");
                self.after_year.replace(value);
            }
            _ => unimplemented!(),
        }
    }

    fn property(&self, _obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        match pspec.name() {
            "sigs-all" => self.sigs_all.borrow().to_value(),
            "sigs-at-least" => self.sigs_at_least.borrow().to_value(),
            "sigs-any" => self.sigs_any.borrow().to_value(),
            "sigs-no" => self.sigs_no.borrow().to_value(),
            "lock-none" => self.lock_none.borrow().to_value(),
            "lock-after" => self.lock_after.borrow().to_value(),
            "lock-older" => self.lock_older.borrow().to_value(),
            "period-years" => self.period_years.borrow().to_value(),
            "period-months" => self.period_months.borrow().to_value(),
            "period-weeks" => self.period_weeks.borrow().to_value(),
            "period-days" => self.period_days.borrow().to_value(),
            "period-span" => self.period_span.borrow().to_value(),
            "after-day" => self.after_day.borrow().to_value(),
            "after-month" => self.after_month.borrow().to_value(),
            "after-year" => self.after_year.borrow().to_value(),
            _ => unimplemented!(),
        }
    }
}

impl ConditionInner {
    pub fn sigs_req(&self) -> SigsReq {
        if *self.sigs_all.borrow() {
            SigsReq::All
        } else if *self.sigs_any.borrow() {
            SigsReq::Any
        } else {
            SigsReq::AtLeast(*self.sigs_no.borrow() as u16)
        }
    }
}

glib::wrapper! {
    pub struct Condition(ObjectSubclass<ConditionInner>);
}

impl Default for Condition {
    fn default() -> Self { glib::Object::new(&[]).expect("Failed to create row data") }
}

impl From<&Condition> for SpendingCondition {
    fn from(condition: &Condition) -> Self {
        let condition = condition.imp().borrow();
        SpendingCondition::Sigs(TimelockedSigs {
            sigs: condition.sigs_req(),
            timelock: TimelockReq::from(condition),
        })
    }
}

impl Condition {
    pub fn sigs_req(&self) -> SigsReq {
        if self.property("sigs-all") {
            SigsReq::All
        } else if self.property("sigs-any") {
            SigsReq::Any
        } else {
            SigsReq::AtLeast(self.property::<u32>("sigs-no") as u16)
        }
    }
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
        glib::Object::new(&[]).expect("Failed to create SpendingModel")
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
        self.items_changed(0, n, 0);
    }

    pub fn remove(&self, index: u32) {
        let imp = self.imp();
        imp.conditions.borrow_mut().remove(index as usize);
        // Emits a signal that 1 item was removed, 0 added at the position index
        self.items_changed(index, 1, 0);
    }

    pub fn spending_conditions(&self) -> Vec<(u8, SpendingCondition)> {
        let imp = self.imp();
        imp.conditions
            .borrow()
            .iter()
            .map(SpendingCondition::from)
            .enumerate()
            .map(|(d, c)| (d as u8 + 1, c))
            .collect()
    }

    pub fn push_condition(&self, sc: &SpendingCondition) {
        let cond = Condition::default();

        cond.set_property(
            "sigs-all",
            matches!(
                sc,
                SpendingCondition::Sigs(TimelockedSigs {
                    sigs: SigsReq::All,
                    ..
                })
            ),
        );
        cond.set_property(
            "sigs-at-least",
            matches!(
                sc,
                SpendingCondition::Sigs(TimelockedSigs {
                    sigs: SigsReq::AtLeast(_),
                    ..
                })
            ),
        );
        cond.set_property(
            "sigs-any",
            matches!(
                sc,
                SpendingCondition::Sigs(TimelockedSigs {
                    sigs: SigsReq::Any,
                    ..
                })
            ),
        );
        match sc {
            SpendingCondition::Sigs(TimelockedSigs {
                sigs: SigsReq::AtLeast(no),
                ..
            }) => Some(no),
            _ => None,
        }
        .map(|no| cond.set_property("sigs-no", *no as u32));

        cond.set_property(
            "lock-none",
            matches!(
                sc,
                SpendingCondition::Sigs(TimelockedSigs {
                    timelock: TimelockReq::Anytime,
                    ..
                })
            ),
        );
        cond.set_property(
            "lock-older",
            matches!(
                sc,
                SpendingCondition::Sigs(TimelockedSigs {
                    timelock: TimelockReq::AfterPeriod(_),
                    ..
                })
            ),
        );
        cond.set_property(
            "lock-after",
            matches!(
                sc,
                SpendingCondition::Sigs(TimelockedSigs {
                    timelock: TimelockReq::AfterDate(_),
                    ..
                })
            ),
        );
        match sc {
            SpendingCondition::Sigs(TimelockedSigs {
                timelock: TimelockReq::AfterPeriod(datetime),
                ..
            }) => Some(datetime),
            _ => None,
        }
        .map(|duration| {
            let span = match duration {
                TimelockDuration::Days(span)
                | TimelockDuration::Weeks(span)
                | TimelockDuration::Months(span)
                | TimelockDuration::Years(span) => *span,
            };
            cond.set_property("period-span", span as u32);
            cond.set_property(
                "period-years",
                matches!(duration, TimelockDuration::Years(_)),
            );
            cond.set_property(
                "period-months",
                matches!(duration, TimelockDuration::Months(_)),
            );
            cond.set_property(
                "period-weeks",
                matches!(duration, TimelockDuration::Weeks(_)),
            );
            cond.set_property("period-days", matches!(duration, TimelockDuration::Days(_)));
        });
        match sc {
            SpendingCondition::Sigs(TimelockedSigs {
                timelock: TimelockReq::AfterDate(datetime),
                ..
            }) => Some(datetime),
            _ => None,
        }
        .map(|datetime| {
            cond.set_property("after-day", datetime.day());
            cond.set_property("after-month", datetime.month());
            cond.set_property("after-year", datetime.year() as u32);
        });

        self.append(&cond);
    }

    pub fn reset_conditions(&self, conditions: &BTreeSet<(u8, SpendingCondition)>) {
        self.clear();
        // TODO: Support script prioritisation
        for (_, condition) in conditions {
            self.push_condition(condition);
        }
    }
}
