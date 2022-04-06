use gtk::prelude::*;
use gtk::prelude::DialogExt;
use gtk::{Button, Inhibit, Label, Dialog};
use relm::{Relm, Update, Widget, WidgetTest};

use gladis::Gladis;

#[derive(Clone, Default)]
pub(crate) struct Model {
}

#[derive(Msg)]
pub(crate) enum Msg {
}

// Create the structure that holds the widgets used in the view.
#[derive(Clone, Gladis)]
pub(crate) struct Widgets {
    dialog: Dialog,
}

pub(crate) struct Win {
    model: Model,
    widgets: Widgets,
}

impl Update for Win {
    // Specify the model used for this widget.
    type Model = Model;
    // Specify the model parameter used to init the model.
    type ModelParam = Model;
    // Specify the type of the messages sent to the update function.
    type Msg = Msg;

    fn model(_relm: &Relm<Self>, model: Self::ModelParam) -> Model {
        model
    }

    fn update(&mut self, event: Msg) {
    }
}

impl Widget for Win {
    // Specify the type of the root widget.
    type Root = Dialog;

    // Return the root widget.
    fn root(&self) -> Self::Root {
        self.widgets.dialog.clone()
    }

    fn view(relm: &Relm<Self>, model: Self::Model) -> Self {
        let glade_src = include_str!("../res/settings.glade");
        let widgets = Widgets::from_string(glade_src).unwrap();

        /*
        connect!(
            relm,
            widgets.plus_button,
            connect_clicked(_),
            Msg::Increment
        );
        connect!(
            relm,
            widgets.window,
            connect_delete_event(_, _),
            return (Some(Msg::Quit), Inhibit(false))
        );
         */

        widgets.dialog.run();

        Win { model, widgets }
    }
}
