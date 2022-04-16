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
use gtk::prelude::{WidgetExt, *};
use gtk::{ApplicationWindow, Button, Inhibit};
use relm::{init, Relm, Update, Widget};

use super::{Msg, ViewModel};
use crate::model::Wallet;
use crate::view::settings;

// Create the structure that holds the widgets used in the view.
#[derive(Clone, Gladis)]
pub struct Widgets {
    window: ApplicationWindow,
    settings_btn: Button,
}

pub struct Component {
    view_model: ViewModel,
    model: Wallet,
    widgets: Widgets,
    settings: relm::Component<settings::Component>,
}

impl Update for Component {
    // Specify the model used for this widget.
    type Model = Wallet;
    // Specify the model parameter used to init the model.
    type ModelParam = Wallet;
    // Specify the type of the messages sent to the update function.
    type Msg = Msg;

    fn model(_: &Relm<Self>, wallet: Self::ModelParam) -> Self::Model {
        wallet
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::Create => self.settings.emit(settings::Msg::New),
            Msg::Settings => self.settings.emit(settings::Msg::View),
            Msg::Quit => gtk::main_quit(),
            _ => { /* TODO: Implement main window event handling */ }
        }
    }
}

impl Widget for Component {
    // Specify the type of the root widget.
    type Root = ApplicationWindow;

    // Return the root widget.
    fn root(&self) -> Self::Root {
        self.widgets.window.clone()
    }

    fn view(relm: &Relm<Self>, model: Self::Model) -> Self {
        let glade_src = include_str!("wallet.glade");
        let widgets = Widgets::from_string(glade_src).expect("glade file broken");

        let settings =
            init::<settings::Component>(model.descriptor()).expect("error in settings component");

        connect!(relm, widgets.settings_btn, connect_clicked(_), Msg::Create);
        connect!(
            relm,
            widgets.window,
            connect_delete_event(_, _),
            return (Some(Msg::Quit), Inhibit(false))
        );

        widgets.window.show();

        Component {
            view_model: model.clone().into(),
            model,
            widgets,
            settings,
        }
    }
}
