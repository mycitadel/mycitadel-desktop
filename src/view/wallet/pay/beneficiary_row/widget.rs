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

use std::str::FromStr;
use std::sync::{Arc, Mutex};

use ::wallet::onchain::PublicNetwork;
use bitcoin::Address;
use bitcoin_scripts::address::AddressCompat;
use gladis::Gladis;
use gtk::prelude::*;
use gtk::{glib, Entry, ListBoxRow, ToggleButton};
use relm::Relm;

use super::Beneficiary;
use crate::view::wallet::{self, pay};

#[derive(Clone, Gladis)]
pub struct RowWidgets {
    beneficiary_row: ListBoxRow,
    address_fld: Entry,
    amount_fld: Entry,
    max_btn: ToggleButton,
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
        let row = row_widgets.beneficiary_row.clone();
        connect!(
            relm,
            row_widgets.max_btn,
            connect_toggled(_),
            wallet::Msg::Pay(pay::Msg::BeneficiaryEdit(row.index() as u32))
        );

        row_widgets.beneficiary_row.upcast::<gtk::Widget>()
    }

    fn bind_model(&self, beneficiary: &Beneficiary, network: PublicNetwork) {
        let ro_flags = glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE;
        let rw_flags = glib::BindingFlags::DEFAULT
            | glib::BindingFlags::SYNC_CREATE
            | glib::BindingFlags::BIDIRECTIONAL;

        beneficiary
            .bind_property("amount", &self.address_fld, "primary_icon_name")
            .transform_to(move |_binding, value| {
                if value.get::<u64>().unwrap() == 0 {
                    Some("dialog-error-symbolic".to_value())
                } else {
                    None
                }
            })
            .flags(ro_flags)
            .build();
        beneficiary
            .bind_property("amount", &self.address_fld, "primary_icon_tooltip_text")
            .transform_to(move |_binding, value| {
                if value.get::<u64>().unwrap() == 0 {
                    Some("Payment without beneficiary address".to_value())
                } else {
                    None
                }
            })
            .flags(ro_flags)
            .build();
        self.address_fld
            .bind_property("text", beneficiary, "address")
            .transform_to(move |binding, value| {
                let address_fld: Entry = binding.source().unwrap().downcast().unwrap();

                let addr_str = address_fld.text();
                let addr_str = addr_str.as_str();
                let (icon, msg) = match (
                    Address::from_str(addr_str),
                    AddressCompat::from_str(addr_str),
                ) {
                    (_, _) if addr_str.is_empty() => (None, None),
                    (Err(err), _) => (
                        Some("dialog-error-symbolic"),
                        Some(format!("Invalid address: {}", err)),
                    ),
                    (_, Err(_)) => (
                        Some("dialog-warning-symbolic"),
                        Some(s!("Address belongs to the future witness version and can \
                                 be spent only if a soft-fork will happen")),
                    ),
                    (Ok(addr), _) if !addr.is_valid_for_network(network.into()) => (
                        Some("dialog-error-symbolic"),
                        Some(format!(
                            "Address belongs to {} network, which does not match the wallet \
                             network",
                            addr.network
                        )),
                    ),
                    (Ok(_), _) => (Some("emblem-ok-symbolic"), Some(s!("Address is valid"))),
                };
                address_fld.set_primary_icon_name(icon);
                address_fld.set_primary_icon_tooltip_text(msg.as_deref());
                Some(value.clone())
            })
            .flags(rw_flags)
            .build();

        self.amount_fld
            .bind_property("text", beneficiary, "amount")
            .transform_to(move |binding, value| {
                let amount_fld: Entry = binding.source().unwrap().downcast().unwrap();
                let amount_str = value.get::<&str>().unwrap();
                let (icon, msg, amount) = match f64::from_str(amount_str) {
                    _ if amount_str.is_empty() => (None, None, 0u64),
                    Err(err) => (
                        Some("dialog-error-symbolic"),
                        Some(format!("Invalid amount: {}", err)),
                        0u64,
                    ),
                    Ok(amount) => {
                        let s = format!("{}", amount);
                        let s = s.split_once('.');
                        if s.map(|(_, r)| r.len()).unwrap_or(0) > 8 {
                            (
                                Some("dialog-warning-symbolic"),
                                Some(s!("Sub-satoshi amount")),
                                (amount * 100_000_000.0) as u64,
                            )
                        } else {
                            (
                                Some("emblem-ok-symbolic"),
                                Some(s!("Amount is valid")),
                                (amount * 100_000_000.0) as u64,
                            )
                        }
                    }
                };
                amount_fld.set_primary_icon_name(icon);
                amount_fld.set_primary_icon_tooltip_text(msg.as_deref());
                Some(amount.to_value())
            })
            .transform_from(move |_binding, value| {
                let btc = value.get::<u64>().unwrap();
                if btc == 0 {
                    Some("".to_value())
                } else {
                    Some(format!("{:.8}", btc as f64 / 100_000_000.0).to_value())
                }
            })
            .flags(rw_flags)
            .build();

        let saved_amount = Arc::new(Mutex::new(0u64));
        beneficiary
            .bind_property("max", &self.amount_fld, "editable")
            .transform_to(move |binding, value| {
                let active = value.get::<bool>().unwrap();
                let amount_fld: Entry = binding.target().unwrap().downcast().unwrap();
                if active {
                    amount_fld.set_primary_icon_name(None);
                }
                amount_fld.set_sensitive(!active);
                Some((!active).to_value())
            })
            .flags(ro_flags)
            .build();
        self.max_btn
            .bind_property("active", beneficiary, "max")
            .transform_to(move |binding, value| {
                let active = value
                    .get::<bool>()
                    .expect("toggle button value is not bool");

                let item: Beneficiary = binding.target().unwrap().downcast().unwrap();
                if active {
                    *saved_amount.lock().unwrap() = item.property::<u64>("amount");
                    item.set_property("amount", 0u64);
                } else {
                    item.set_property("amount", *saved_amount.lock().unwrap());
                }
                Some(value.clone())
            })
            .flags(rw_flags)
            .build();
    }
}
