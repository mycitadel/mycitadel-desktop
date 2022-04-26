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
use gtk::{glib, Entry, ListBoxRow};
use relm::Relm;

use super::Beneficiary;
use crate::view::pay;

#[derive(Clone, Gladis)]
pub struct RowWidgets {
    beneficiary_row: ListBoxRow,
    address_fld: Entry,
    amount_fld: Entry,
}

impl RowWidgets {
    pub fn init(relm: Relm<pay::Component>, item: &glib::Object) -> gtk::Widget {
        let glade_src = include_str!("beneficiary_row.glade");
        let row_widgets = RowWidgets::from_string(glade_src).expect("glade file broken");

        let beneficiary = item
            .downcast_ref::<Beneficiary>()
            .expect("Row data is of wrong type");
        row_widgets.bind_model(beneficiary);

        connect!(
            relm,
            row_widgets.address_fld,
            connect_changed(_),
            pay::Msg::BeneficiaryChange
        );
        connect!(
            relm,
            row_widgets.amount_fld,
            connect_changed(_),
            pay::Msg::BeneficiaryChange
        );

        row_widgets.beneficiary_row.upcast::<gtk::Widget>()
    }

    fn bind_model(&self, beneficiary: &Beneficiary) {
        let flags_ro = glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE;

        self.address_fld
            .bind_property("text", beneficiary, "address")
            .flags(flags_ro)
            .build();
        self.amount_fld
            .bind_property("text", beneficiary, "amount")
            .flags(flags_ro)
            .build();
    }
}
