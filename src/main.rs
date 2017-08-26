extern crate gtk;
extern crate gdk;
extern crate cairo;
extern crate rsvg;
extern crate shakmaty;
extern crate option_filter;
extern crate time;
extern crate rand;

use gtk::prelude::*;
use gtk::{Window, WindowType};

mod drawable;
mod util;
mod pieceset;
mod ground;

use ground::BoardView;

fn main() {
    gtk::init().expect("initialized gtk");

    let window = Window::new(WindowType::Toplevel);
    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    let board = BoardView::new();
    window.add(board.widget());
    window.show_all();

    gtk::main();
}
