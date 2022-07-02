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
use gtk::Dialog;
use relm::{Relm, Update, Widget};

use super::{Msg, ViewModel, Widgets};

pub struct Component {
    model: ViewModel,
    widgets: Widgets,
}

impl Component {}

impl Update for Component {
    // Specify the model used for this widget.
    type Model = ViewModel;
    // Specify the model parameter used to init the model.
    type ModelParam = ();
    // Specify the type of the messages sent to the update function.
    type Msg = Msg;

    fn model(_relm: &Relm<Self>, _param: Self::ModelParam) -> Self::Model { ViewModel::default() }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::Show => {
                self.widgets.init_ui(&self.model);
                self.widgets.show();
            }
            Msg::Response(_) => {
                self.widgets.hide();
            }
        }
    }
}

impl Widget for Component {
    // Specify the type of the root widget.
    type Root = Dialog;

    // Return the root widget.
    fn root(&self) -> Self::Root { self.widgets.to_root() }

    fn view(relm: &Relm<Self>, model: Self::Model) -> Self {
        let glade_src = include_str!("about.glade");
        let widgets = Widgets::from_string(glade_src).expect("glade file broken");

        widgets.connect(relm);
        widgets.init_ui(&model);
        widgets.show();

        Component { model, widgets }
    }
}
