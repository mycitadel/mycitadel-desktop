use gtk::prelude::GtkWindowExt;
use gtk::{ApplicationWindow};
use relm4::{AppUpdate, Model, RelmApp, Sender, Widgets};
use gladis4::Gladis;

#[derive(Default)]
struct AppModel {
    counter: u8,
}

enum AppMsg {
    Increment,
    Decrement,
}

impl Model for AppModel {
    type Msg = AppMsg;
    type Widgets = AppWidgets;
    type Components = ();
}

impl AppUpdate for AppModel {
    fn update(&mut self, msg: AppMsg, _components: &(), _sender: Sender<AppMsg>) -> bool {
        match msg {
            AppMsg::Increment => {
                self.counter = self.counter.wrapping_add(1);
            }
            AppMsg::Decrement => {
                self.counter = self.counter.wrapping_sub(1);
            }
        }
        true
    }
}

#[derive(Gladis, Clone)]
pub struct AppWidgets {
    pub window: ApplicationWindow,
}

impl Widgets<AppModel, ()> for AppWidgets {
    type Root = ApplicationWindow;

    fn root_widget(&self) -> Self::Root {
        self.window.clone()
    }

    fn init_view(_model: &AppModel, _components: &(), _sender: Sender<AppMsg>) -> Self {
        let ui_string = include_str!("view/wallet.glade");
        let widgets = Self::from_string(ui_string).expect("broken wallet.glade");

        widgets.window.present();

        widgets
    }

    fn view(&mut self, _model: &AppModel, _sender: Sender<AppMsg>) {
        // Update widgets
    }
}

fn main() {
    let model = AppModel::default();
    let app = RelmApp::new(model);
    app.run();
}
