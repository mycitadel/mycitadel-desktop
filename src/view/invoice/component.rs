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
use gtk::{Dialog, ResponseType};
use relm::{Relm, Update, Widget};

use super::{Msg, ViewModel, Widgets};
use crate::model::Wallet;

pub struct Component {
    model: ViewModel,
    widgets: Widgets,
}

impl Component {}

impl Update for Component {
    // Specify the model used for this widget.
    type Model = ViewModel;
    // Specify the model parameter used to init the model.
    type ModelParam = Wallet;
    // Specify the type of the messages sent to the update function.
    type Msg = Msg;

    fn model(_relm: &Relm<Self>, wallet: Self::ModelParam) -> Self::Model {
        ViewModel::with(wallet)
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::Show => {
                self.widgets.update_ui(&self.model);
                self.widgets.show();
            }
            Msg::Response(ResponseType::Ok) => {
                self.widgets.hide();
            }
            Msg::Response(ResponseType::Cancel) => {
                self.widgets.close();
            }
            Msg::Response(_) => unreachable!(),
        }
    }
}

impl Widget for Component {
    // Specify the type of the root widget.
    type Root = Dialog;

    // Return the root widget.
    fn root(&self) -> Self::Root {
        self.widgets.to_root()
    }

    fn view(relm: &Relm<Self>, model: Self::Model) -> Self {
        let glade_src = include_str!("invoice.glade");
        let widgets = Widgets::from_string(glade_src).expect("glade file broken");

        widgets.connect(relm);
        widgets.update_ui(&model);
        widgets.show();

        Component { model, widgets }
    }
}
