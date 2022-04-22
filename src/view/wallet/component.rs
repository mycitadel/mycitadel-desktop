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
use relm::{init, Relm, StreamHandle, Update, Widget};

use super::{Msg, ViewModel};
use crate::model::{Wallet, WalletDescriptor};
use crate::view::{launch, settings};

// Create the structure that holds the widgets used in the view.
#[derive(Clone, Gladis)]
pub struct Widgets {
    window: ApplicationWindow,
    new_btn: Button,
    open_btn: Button,
    settings_btn: Button,
}

pub struct Component {
    view_model: ViewModel,
    model: Wallet,
    widgets: Widgets,
    settings: relm::Component<settings::Component>,
    launcher_stream: Option<StreamHandle<launch::Msg>>,
}

impl Update for Component {
    // Specify the model used for this widget.
    type Model = Wallet;
    // Specify the model parameter used to init the model.
    type ModelParam = WalletDescriptor;
    // Specify the type of the messages sent to the update function.
    type Msg = Msg;

    fn model(_relm: &Relm<Self>, descriptor: Self::ModelParam) -> Self::Model {
        Wallet::with(descriptor)
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::New => {
                self.launcher_stream
                    .as_ref()
                    .map(|stream| stream.emit(launch::Msg::Show));
            }
            Msg::Settings => self
                .settings
                .emit(settings::Msg::View(self.model.to_descriptor())),
            Msg::Update(descr) => {
                self.model.set_descriptor(descr);
                self.widgets.window.show();
            }
            Msg::RegisterLauncher(stream) => {
                self.launcher_stream = Some(stream);
            }
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

        let settings = init::<settings::Component>(()).expect("error in settings component");
        settings.emit(settings::Msg::SetWallet(relm.stream().clone()));

        connect!(relm, widgets.new_btn, connect_clicked(_), Msg::New);
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
            return (Some(Msg::Close), Inhibit(false))
        );

        widgets.window.show();

        Component {
            view_model: model.clone().into(),
            model,
            widgets,
            settings,
            launcher_stream: None,
        }
    }
}
