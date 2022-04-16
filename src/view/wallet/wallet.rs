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

use std::sync::{Arc, Mutex};

use gladis::Gladis;
use gtk::prelude::{WidgetExt, *};
use gtk::{ApplicationWindow, Button, Inhibit};
use relm::{init, Component, Relm, Update, Widget};

use crate::view::settings;

#[derive(Default)]
pub struct Model {
    settings: Arc<Mutex<settings::ViewModel>>,
}

#[derive(Msg)]
pub enum Msg {
    New,
    Open,
    Send,
    Receive,
    Refresh,
    Select(usize),
    Settings,
    Quit,
}

// Create the structure that holds the widgets used in the view.
#[derive(Clone, Gladis)]
pub struct Widgets {
    window: ApplicationWindow,
    settings_btn: Button,
}

pub struct Win {
    model: Model,
    widgets: Widgets,
    settings_win: Component<settings::Win>,
}

impl Update for Win {
    // Specify the model used for this widget.
    type Model = Model;
    // Specify the model parameter used to init the model.
    type ModelParam = ();
    // Specify the type of the messages sent to the update function.
    type Msg = Msg;

    fn model(_: &Relm<Self>, _: ()) -> Model { Model::default() }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::Settings => self.settings_win.emit(settings::Msg::Show),
            Msg::Quit => gtk::main_quit(),
            _ => { /* TODO: Implement main window event handling */ }
        }
    }
}

impl Widget for Win {
    // Specify the type of the root widget.
    type Root = ApplicationWindow;

    // Return the root widget.
    fn root(&self) -> Self::Root { self.widgets.window.clone() }

    fn view(relm: &Relm<Self>, model: Self::Model) -> Self {
        let glade_src = include_str!("wallet.glade");
        let widgets = Widgets::from_string(glade_src).expect("glade file broken");

        let settings_win =
            init::<settings::Win>(model.settings.clone()).expect("error in settings dialog");

        connect!(
            relm,
            widgets.settings_btn,
            connect_clicked(_),
            Msg::Settings
        );
        connect!(
            relm,
            widgets.window,
            connect_delete_event(_, _),
            return (Some(Msg::Quit), Inhibit(false))
        );

        widgets.window.show();

        Win {
            model,
            widgets,
            settings_win,
        }
    }
}
