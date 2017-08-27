extern crate gtk;
extern crate chessground;
#[macro_use]
extern crate relm;
#[macro_use]
extern crate relm_derive;

use gtk::prelude::*;
use gtk::{Window, WindowType};
use relm::{Relm, Update, Widget, Component, ContainerWidget};

use chessground::{Ground, GroundMsg};

#[derive(Msg)]
enum WinMsg {
    Quit,
}

struct Win {
    _ground: Component<Ground>,
    window: Window,
}

impl Update for Win {
    type Model = ();
    type ModelParam = ();
    type Msg = WinMsg;

    fn model(_: &Relm<Self>, _: ()) -> Self::Model {
        ()
    }

    fn update(&mut self, event: Self::Msg) {
        match event {
            WinMsg::Quit => gtk::main_quit(),
        }
    }
}

impl Widget for Win {
    type Root = Window;

    fn root(&self) -> Self::Root {
        self.window.clone()
    }

    fn view(relm: &Relm<Self>, _model: Self::Model) -> Self {
        let window = Window::new(WindowType::Toplevel);
        let ground = window.add_widget::<Ground, _>(relm, ());

        window.show_all();

        connect!(ground@GroundMsg::UserMove { orig, dest }, relm, println!("hello {} {}", orig, dest));
        connect!(relm, window, connect_delete_event(_, _), return (Some(WinMsg::Quit), Inhibit(false)));

        Win {
            _ground: ground,
            window,
        }
    }
}

fn main() {
    Win::run(()).unwrap();
}
