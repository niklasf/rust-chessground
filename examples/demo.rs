extern crate gtk;
extern crate rand;
extern crate chessground;

use gtk::prelude::*;
use gtk::{Window, WindowType};

use chessground::BoardView;

fn main() {
    gtk::init().expect("initialized gtk");

    let window = Window::new(WindowType::Toplevel);
    window.set_title("Chessground demo");
    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    let board = BoardView::new();
    window.add(board.widget());
    window.show_all();

    gtk::main();
}
