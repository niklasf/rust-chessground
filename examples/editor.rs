extern crate gtk;
extern crate chessground;
#[macro_use]
extern crate relm;
extern crate relm_attributes;
#[macro_use]
extern crate relm_derive;
extern crate shakmaty;

use gtk::prelude::*;
use relm::Widget;
use relm_attributes::widget;

use shakmaty::{Square, Board};
use chessground::{Ground, UserMove, SetBoard};

use self::Msg::*;

#[derive(Msg)]
pub enum Msg {
    Quit,
    PieceMoved(Square, Square),
}

#[widget]
impl Widget for Win {
    fn model() -> Board {
        Board::default()
    }

    fn update(&mut self, event: Msg) {
        match event {
            Quit => gtk::main_quit(),
            PieceMoved(orig, dest) => {
                if let Some(piece) = self.model.remove_piece_at(orig) {
                    self.model.set_piece_at(dest, piece, false);
                    self.ground.emit(SetBoard(self.model.clone()));
                }
            }
        }
    }

    view! {
        gtk::Window {
            title: "Chessground",
            #[name="ground"]
            Ground {
                UserMove(orig, dest, _) => PieceMoved(orig, dest),
            },
            delete_event(_, _) => (Quit, Inhibit(false)),
        }
    }
}

fn main() {
    Win::run(()).expect("initialized gtk");
}
