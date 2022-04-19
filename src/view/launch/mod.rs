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
use gtk::{ApplicationWindow, Button, ListBox, Notebook, RecentChooserWidget};
use relm::{Relm, Update, Widget};

#[derive(Msg)]
pub enum Msg {
    Action,
    PageChange(u32),
    TemplateSelected,
    ImportSelected,
    OpenSelected,
    RecentSelected,
}

#[derive(Clone, Gladis)]
pub struct Widgets {
    window: ApplicationWindow,
    action_btn: Button,
    pages: Notebook,
    create_box: ListBox,
    import_box: ListBox,
    open_box: ListBox,
    recent: RecentChooserWidget,
}

impl Update for Widgets {
    // Specify the model used for this widget.
    type Model = ();
    // Specify the model parameter used to init the model.
    type ModelParam = ();
    // Specify the type of the messages sent to the update function.
    type Msg = Msg;

    fn model(_relm: &Relm<Self>, _model: Self::ModelParam) -> Self::Model {
        ()
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::Action => {}
            Msg::PageChange(page) => {}
            Msg::TemplateSelected => {}
            Msg::ImportSelected => {}
            Msg::OpenSelected => {}
            Msg::RecentSelected => {}
        }
    }
}

impl Widget for Widgets {
    // Specify the type of the root widget.
    type Root = ApplicationWindow;

    // Return the root widget.
    fn root(&self) -> Self::Root {
        self.window.clone()
    }

    fn view(relm: &Relm<Self>, _model: Self::Model) -> Self {
        let glade_src = include_str!("launch.glade");
        let widgets = Widgets::from_string(glade_src).expect("glade file broken");

        connect!(relm, widgets.action_btn, connect_clicked(_), Msg::Action);
        connect!(
            relm,
            widgets.pages,
            connect_switch_page(_, _, page),
            Msg::PageChange(page)
        );

        connect!(
            relm,
            widgets.create_box,
            connect_row_selected(_, _),
            Msg::TemplateSelected
        );
        connect!(
            relm,
            widgets.import_box,
            connect_row_selected(_, _),
            Msg::ImportSelected
        );
        connect!(
            relm,
            widgets.open_box,
            connect_row_selected(_, _),
            Msg::OpenSelected
        );
        connect!(
            relm,
            widgets.recent,
            connect_selection_changed(_),
            Msg::RecentSelected
        );

        widgets.window.show();

        widgets
    }
}
