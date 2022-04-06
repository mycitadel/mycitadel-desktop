use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};
use gtk::prelude::*;
use gtk::prelude::DialogExt;
use gtk::{Button, Dialog};
use relm::{Relm, Update, Widget};

use gladis::Gladis;

#[derive(Clone, Default)]
pub(crate) struct Model {
}

#[derive(Msg)]
pub(crate) enum Msg {
    Init(Arc<Mutex<Model>>),
    Save,
    Cancel,
}

// Create the structure that holds the widgets used in the view.
#[derive(Clone, Gladis)]
pub(crate) struct Widgets {
    dialog: Dialog,
    save_btn: Button,
    cancel_btn: Button,
}

pub(crate) struct Win {
    model: Model,
    origin_model: Option<Arc<Mutex<Model>>>,
    widgets: Widgets,
}

impl Update for Win {
    // Specify the model used for this widget.
    type Model = Model;
    // Specify the model parameter used to init the model.
    type ModelParam = Arc<Mutex<Model>>;
    // Specify the type of the messages sent to the update function.
    type Msg = Msg;

    fn model(relm: &Relm<Self>, model: Self::ModelParam) -> Self::Model {
        relm.stream().emit(Msg::Init(model.clone()));
        model.lock().expect("wallet model locked").deref().clone()
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::Init(origin_model) => {
                self.origin_model = Some(origin_model);
            }
            Msg::Save => {
                self.origin_model.as_ref().map(|model| {
                    *(model.lock().expect("wallet model locked").deref_mut()) = self.model.clone();
                });
                self.widgets.dialog.close();
            }
            Msg::Cancel => {
                self.widgets.dialog.close();
            }
        }
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

        connect!(
            relm,
            widgets.save_btn,
            connect_clicked(_),
            Msg::Save
        );
        connect!(
            relm,
            widgets.cancel_btn,
            connect_clicked(_),
            Msg::Cancel
        );

        widgets.dialog.run();

        Win { model, widgets, origin_model: None }
    }
}
