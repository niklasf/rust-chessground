extern crate gtk;
#[macro_use]
extern crate relm;
#[macro_use]
extern crate relm_derive;
extern crate chessground;

use relm::{Relm, Update, Widget};
use gtk::prelude::*;
use gtk::{Window, WindowType, DrawingArea};
use chessground::BoardView;

struct Model {
}

#[derive(Msg)]
enum Msg {
    Quit,
}

struct Win {
    model: Model,
    window: Window,
    ground: BoardView,
}

impl Update for Win {
    type Model = Model;
    type ModelParam = ();
    type Msg = Msg;

    fn model(_: &Relm<Self>, _: ()) -> Model {
        Model {
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::Quit => gtk::main_quit(),
        }
    }
}

impl Widget for Win {
    type Root = Window;

    fn root(&self) -> Self::Root {
        self.window.clone()
    }

    fn view(relm: &Relm<Self>, model: Self::Model) -> Self {
        let window = Window::new(WindowType::Toplevel);
        let ground = BoardView::new();
        window.add(ground.widget());

        connect!(relm, window, connect_delete_event(_, _), return (Some(Msg::Quit), Inhibit(false)));

        window.show_all();

        Win {
            model,
            window,
            ground,
        }
    }
}

fn main() {
    Win::run(()).unwrap();
}
