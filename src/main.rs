use gtk::{
    prelude::BuilderExtManual, prelude::ButtonExt, prelude::LabelExt, prelude::WidgetExt, Button,
    Inhibit, Label, Window,
};
use relm::{connect, Relm, Update, Widget, WidgetTest};
use relm_derive::Msg;

use gladis::Gladis;

struct Model {
    counter: i32,
}

#[derive(Msg)]
enum Msg {
    Decrement,
    Increment,
    Quit,
}

// Create the structure that holds the widgets used in the view.
#[derive(Clone, Gladis)]
struct Widgets {
    /*
    counter_label: Label,
    minus_button: Button,
    plus_button: Button,
     */
    window: Window,
}

struct Win {
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
        Model { counter: 0 }
    }

    fn update(&mut self, event: Msg) {
        /*
        let label = &self.widgets.counter_label;

        match event {
            Msg::Decrement => {
                self.model.counter -= 1;
                // Manually update the view.
                label.set_text(&self.model.counter.to_string());
            }
            Msg::Increment => {
                self.model.counter += 1;
                label.set_text(&self.model.counter.to_string());
            }
            Msg::Quit => gtk::main_quit(),
        }
         */
    }
}

impl Widget for Win {
    // Specify the type of the root widget.
    type Root = Window;

    // Return the root widget.
    fn root(&self) -> Self::Root {
        self.widgets.window.clone()
    }

    fn view(relm: &Relm<Self>, model: Self::Model) -> Self {
        let glade_src = include_str!("../res/wallet.glade");
        let widgets = Widgets::from_string(glade_src).unwrap();

        widgets.window.show();

        /*
        connect!(
            relm,
            widgets.plus_button,
            connect_clicked(_),
            Msg::Increment
        );
        connect!(
            relm,
            widgets.minus_button,
            connect_clicked(_),
            Msg::Decrement
        );
        connect!(
            relm,
            widgets.window,
            connect_delete_event(_, _),
            return (Some(Msg::Quit), Inhibit(false))
        );
         */

        Win { model, widgets }
    }
}

impl WidgetTest for Win {
    type Streams = ();

    fn get_streams(&self) -> Self::Streams {}

    type Widgets = Widgets;

    fn get_widgets(&self) -> Self::Widgets {
        self.widgets.clone()
    }
}

fn main() {
    Win::run(()).expect("Win::run failed");
}

#[cfg(test)]
mod tests {
    use gtk::prelude::LabelExt;

    use gtk_test::assert_text;
    use relm_test::click;

    use crate::Win;

    #[test]
    #[ignore]
    fn label_change() {
        let (_component, _, widgets) = relm::init_test::<Win>(()).expect("init_test failed");
        let plus_button = &widgets.plus_button;
        let minus_button = &widgets.minus_button;
        let label = &widgets.counter_label;

        assert_text!(label, 0);
        click(plus_button);
        assert_text!(label, 1);
        click(plus_button);
        assert_text!(label, 2);
        click(plus_button);
        assert_text!(label, 3);
        click(plus_button);
        assert_text!(label, 4);

        click(minus_button);
        assert_text!(label, 3);
        click(minus_button);
        assert_text!(label, 2);
        click(minus_button);
        assert_text!(label, 1);
        click(minus_button);
        assert_text!(label, 0);
        click(minus_button);
        assert_text!(label, -1);
    }
}
