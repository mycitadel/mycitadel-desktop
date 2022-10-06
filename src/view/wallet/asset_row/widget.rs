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
use gtk::{glib, Label, ListBoxRow};

use super::AssetInfo;

#[derive(Clone, Gladis)]
pub struct RowWidgets {
    asset_row: ListBoxRow,
    name_lbl: Label,
    amount_lbl: Label,
    ticker_lbl: Label,
    contract_lbl: Label,
}

impl RowWidgets {
    pub fn init(item: &glib::Object) -> gtk::Widget {
        let glade_src = include_str!("asset_row.glade");
        let row_widgets = RowWidgets::from_string(glade_src).expect("glade file broken");

        let asset = item
            .downcast_ref::<AssetInfo>()
            .expect("Row data is of wrong type");
        row_widgets.bind_model(asset);

        row_widgets.asset_row.upcast::<gtk::Widget>()
    }

    fn bind_model(&self, asset: &AssetInfo) {
        let flags = glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE;

        asset
            .bind_property("name", &self.name_lbl, "label")
            .flags(flags)
            .build();

        asset
            .bind_property("ticker", &self.ticker_lbl, "label")
            .flags(flags)
            .build();

        asset
            .bind_property("contract", &self.contract_lbl, "label")
            .flags(flags)
            .build();

        asset
            .bind_property("amount", &self.amount_lbl, "label")
            .flags(flags)
            .build();
    }
}
