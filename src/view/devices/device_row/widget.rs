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

use gladis::Gladis;
use gtk::prelude::*;
use gtk::{glib, Adjustment, Button, Label, ListBoxRow, SpinButton, Spinner};
use relm::StreamHandle;

use super::DeviceData;
use crate::view::devices;

#[derive(Clone, Gladis)]
pub struct RowWidgets {
    pub device_row: ListBoxRow,
    name_lbl: Label,
    fingerprint_lbl: Label,
    xpub_lbl: Label,
    spinner: Spinner,
    account_adj: Adjustment,
    account_spin: SpinButton,
    add_btn: Button,
}

impl RowWidgets {
    pub fn init(stream_: StreamHandle<devices::Msg>, item: &glib::Object) -> gtk::Widget {
        let glade_src = include_str!("device_row.glade");
        let row_widgets = RowWidgets::from_string(glade_src).expect("glade file broken");

        let device = item
            .downcast_ref::<DeviceData>()
            .expect("Row data is of wrong type");
        let fingerprint = device.fingerprint();
        row_widgets.bind_model(device);

        let stream = stream_.clone();
        row_widgets.account_adj.connect_value_changed(move |adj| {
            let account = adj.value() as u32;
            stream.emit(devices::Msg::AccountChange(fingerprint, account))
        });

        let stream = stream_.clone();
        row_widgets.add_btn.connect_clicked(move |_| {
            stream.emit(devices::Msg::Add(fingerprint));
        });

        row_widgets.device_row.upcast::<gtk::Widget>()
    }

    fn bind_model(&self, device: &DeviceData) {
        device
            .bind_property("name", &self.name_lbl, "label")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .build();
        device
            .bind_property("fingerprint", &self.fingerprint_lbl, "label")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .build();
        device
            .bind_property("xpub", &self.xpub_lbl, "label")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .build();
        device
            .bind_property("account", &self.account_adj, "value")
            .flags(
                glib::BindingFlags::DEFAULT
                    | glib::BindingFlags::SYNC_CREATE
                    | glib::BindingFlags::BIDIRECTIONAL,
            )
            .build();
        device
            .bind_property("updating", &self.xpub_lbl, "visible")
            .flags(
                glib::BindingFlags::DEFAULT
                    | glib::BindingFlags::SYNC_CREATE
                    | glib::BindingFlags::INVERT_BOOLEAN,
            )
            .build();
        device
            .bind_property("updating", &self.spinner, "active")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .build();
        device
            .bind_property("updating", &self.account_spin, "sensitive")
            .flags(
                glib::BindingFlags::DEFAULT
                    | glib::BindingFlags::SYNC_CREATE
                    | glib::BindingFlags::INVERT_BOOLEAN,
            )
            .build();
        device
            .bind_property("updating", &self.add_btn, "sensitive")
            .flags(
                glib::BindingFlags::DEFAULT
                    | glib::BindingFlags::SYNC_CREATE
                    | glib::BindingFlags::INVERT_BOOLEAN,
            )
            .build();
    }
}
