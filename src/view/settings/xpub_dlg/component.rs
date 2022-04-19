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
use gtk::MessageDialog;
use relm::{Relm, Update, Widget};

use super::{ModelParam, Msg, ViewModel, Widgets, XpubModel};

pub struct Component {
    model: ViewModel,
    widgets: Widgets,
}

impl Update for Component {
    // Specify the model used for this widget.
    type Model = ViewModel;
    // Specify the model parameter used to init the model.
    type ModelParam = ModelParam;
    // Specify the type of the messages sent to the update function.
    type Msg = Msg;

    fn model(_relm: &Relm<Self>, config: Self::ModelParam) -> Self::Model {
        ViewModel {
            config,
            xpub_model: XpubModel::default(),
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::Open => {}
            Msg::Edit => {
                // TODO: parse xpub
            }
            Msg::Close => {}
            Msg::Ok => {}
        }
    }
}

impl Widget for Component {
    // Specify the type of the root widget.
    type Root = MessageDialog;

    // Return the root widget.
    fn root(&self) -> Self::Root {
        self.widgets.to_root()
    }

    fn view(relm: &Relm<Self>, model: Self::Model) -> Self {
        let glade_src = include_str!("xpub_dlg.glade");
        let widgets = Widgets::from_string(glade_src).expect("glade file broken");

        // connect!(relm, model.xpub_model, connect_notify("xpub"), Msg::Edit);

        let stream = relm.stream().clone();
        model
            .xpub_model
            .connect_notify(Some("xpub"), move |_, _| stream.emit(Msg::Edit));

        widgets.bind_model(&model.xpub_model);

        Component { model, widgets }
    }
}
