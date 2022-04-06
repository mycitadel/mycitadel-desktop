use std::sync::{Arc, Mutex};
use gtk::prelude::*;
use gtk::prelude::WidgetExt;
use gtk::{Button, Inhibit, ApplicationWindow};
use relm::{Relm, Update, Widget};

use gladis::Gladis;
use crate::settings;

#[derive(Default)]
pub(crate) struct Model {
    settings: Arc<Mutex<settings::Model>>
}

#[derive(Msg)]
pub(crate) enum Msg {
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
pub(crate) struct Widgets {
    window: ApplicationWindow,
    settings_btn: Button,
}

pub(crate) struct Win {
    model: Model,
    widgets: Widgets,
}

impl Update for Win {
    // Specify the model used for this widget.
    type Model = Model;
    // Specify the model parameter used to init the model.
    type ModelParam = ();
    // Specify the type of the messages sent to the update function.
    type Msg = Msg;

    fn model(_: &Relm<Self>, _: ()) -> Model {
        Model::default()
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::Settings => settings::Win::run(self.model.settings.clone()).expect("error in settings dialog"),
            Msg::Quit => gtk::main_quit(),
            _ => { /* TODO: Implement main window event handling */ }
        }
    }
}

impl Widget for Win {
    // Specify the type of the root widget.
    type Root = ApplicationWindow;

    // Return the root widget.
    fn root(&self) -> Self::Root {
        self.widgets.window.clone()
    }

    fn view(relm: &Relm<Self>, model: Self::Model) -> Self {
        let glade_src = include_str!("../res/wallet.glade");
        let widgets = Widgets::from_string(glade_src).unwrap();

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

        Win { model, widgets }
    }
}
