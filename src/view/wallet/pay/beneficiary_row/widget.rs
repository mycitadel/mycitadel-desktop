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

use std::str::FromStr;

use ::wallet::address::AddressCompat;
use bitcoin::Address;
use gladis::Gladis;
use gtk::prelude::*;
use gtk::{glib, Entry, ListBoxRow};
use relm::Relm;

use super::Beneficiary;
use crate::model::PublicNetwork;
use crate::view::wallet::{self, pay};

#[derive(Clone, Gladis)]
pub struct RowWidgets {
    beneficiary_row: ListBoxRow,
    address_fld: Entry,
    amount_fld: Entry,
}

impl RowWidgets {
    pub fn init(
        relm: Relm<wallet::Component>,
        item: &glib::Object,
        network: PublicNetwork,
    ) -> gtk::Widget {
        let glade_src = include_str!("beneficiary_row.glade");
        let row_widgets = RowWidgets::from_string(glade_src).expect("glade file broken");

        let beneficiary = item
            .downcast_ref::<Beneficiary>()
            .expect("Row data is of wrong type");
        row_widgets.bind_model(beneficiary, network);

        let row = row_widgets.beneficiary_row.clone();
        connect!(
            relm,
            row_widgets.address_fld,
            connect_changed(_),
            wallet::Msg::Pay(pay::Msg::BeneficiaryEdit(row.index() as u32))
        );
        let row = row_widgets.beneficiary_row.clone();
        connect!(
            relm,
            row_widgets.amount_fld,
            connect_changed(_),
            wallet::Msg::Pay(pay::Msg::BeneficiaryEdit(row.index() as u32))
        );

        row_widgets.beneficiary_row.upcast::<gtk::Widget>()
    }

    fn bind_model(&self, beneficiary: &Beneficiary, network: PublicNetwork) {
        let flags = glib::BindingFlags::DEFAULT
            | glib::BindingFlags::SYNC_CREATE
            | glib::BindingFlags::BIDIRECTIONAL;

        self.address_fld
            .bind_property("text", beneficiary, "address")
            .transform_to(move |binding, value| {
                let address_fld: Entry = binding.source().unwrap().downcast().unwrap();

                let addr_str = address_fld.text();
                let addr_str = addr_str.as_str();
                let (icon, msg) = match (Address::from_str(addr_str), AddressCompat::from_str(addr_str)) {
                    (_, _) if addr_str.is_empty() => (None, None),
                    (Err(err), _) => (Some("dialog-error-symbolic"), Some(format!("Invalid address: {}", err))),
                    (_, Err(_)) => (Some("dialog-warning-symbolic"), Some(s!("Address belongs to the future witness version and can be spent only if a soft-fork will happen"))),
                    (Ok(addr), _) if !addr.is_valid_for_network(network.into()) => {
                        (Some("dialog-error-symbolic"), Some(format!("Address belongs to {} network, which does not match the wallet network", addr.network)))
                    }
                    (Ok(_), _) => (Some("emblem-ok-symbolic"), Some(s!("Address is valid"))),
                };
                address_fld.set_secondary_icon_name(icon);
                address_fld.set_secondary_icon_tooltip_text(msg.as_deref());
                Some(value.clone())
            })
            .flags(flags)
            .build();

        self.amount_fld
            .bind_property("text", beneficiary, "amount")
            .transform_to(move |binding, _value| {
                let item: Beneficiary = binding.target().unwrap().downcast().unwrap();
                let amount_fld: Entry = binding.source().unwrap().downcast().unwrap();

                let amount_str = amount_fld.text();
                let amount_str = amount_str.as_str();
                let addr_str: String = item.property("address");
                let (icon, msg, amount) = match f64::from_str(amount_str) {
                    _ if amount_str.is_empty() => (None, None, 0u64),
                    Err(err) => (
                        Some("dialog-error-symbolic"),
                        Some(format!("Invalid amount: {}", err)),
                        0u64,
                    ),
                    Ok(amount) if addr_str.is_empty() => (
                        Some("dialog-error-symbolic"),
                        Some(format!("Payment without beneficiary address")),
                        (amount * 100_000_000.0) as u64,
                    ),
                    Ok(amount) => {
                        let sats = (amount * 100_000_000.0) as u64;
                        if sats as f64 / 100_000_000.0 != amount {
                            (
                                Some("dialog-warning-symbolic"),
                                Some(s!("Subsatoshi amount")),
                                sats,
                            )
                        } else {
                            (
                                Some("emblem-ok-symbolic"),
                                Some(s!("Amount is valid")),
                                sats,
                            )
                        }
                    }
                };
                amount_fld.set_secondary_icon_name(icon);
                amount_fld.set_secondary_icon_tooltip_text(msg.as_deref());
                Some(amount.to_value())
            })
            .flags(flags)
            .build();
    }
}
