#![feature(proc_macro)]

extern crate gtk;
extern crate chessground;
#[macro_use]
extern crate relm;
#[macro_use]
extern crate relm_derive;
extern crate shakmaty;
extern crate option_filter;
extern crate rand;
extern crate relm_attributes;

use option_filter::OptionFilterExt;

use rand::distributions::{Range, IndependentSample};

use gtk::prelude::*;
use relm::{Widget};
use relm_attributes::widget;

use shakmaty::{Square, Role, Chess, Position, MoveList, Setup};

use chessground::{Ground, GroundMsg, UserMove};


#[derive(Msg)]
pub enum WinMsg {
    Quit,
    MovePlayed(Square, Square, Option<Role>),
}

use WinMsg::*;

#[widget]
impl Widget for Win {
    fn model() -> Chess {
        Chess::default()
    }

    fn update(&mut self, event: WinMsg) {
        match event {
            Quit => {
                gtk::main_quit()
            },
            MovePlayed(orig, dest, promotion) => {
                let mut legals = MoveList::new();
                self.model.legal_moves(&mut legals);

                let m = legals.drain(..).find(|m| {
                    m.from() == Some(orig) && m.to() == dest &&
                    m.promotion() == promotion
                });

                let last_move = if let Some(m) = m {
                    self.model = self.model.clone().play_unchecked(&m);
                    Some((m.from().unwrap_or_else(|| m.to()), m.to()))
                } else {
                    return;
                };

                legals.clear();
                self.model.legal_moves(&mut legals);

                let last_move = if !legals.is_empty() {
                    // respond with a random move
                    let mut rng = rand::thread_rng();
                    let idx = Range::new(0, legals.len()).ind_sample(&mut rng);
                    let m = &legals[idx];
                    self.model = self.model.clone().play_unchecked(m);
                    Some((m.from().unwrap_or_else(|| m.to()), m.to()))
                } else {
                    last_move
                };

                legals.clear();
                self.model.legal_moves(&mut legals);

                self.ground.emit(GroundMsg::SetPosition {
                    board: self.model.board().clone(),
                    legals,
                    last_move,
                    check: self.model.board().king_of(self.model.turn()).filter(|_| self.model.checkers().any())
                });
            }
        }
    }

    view! {
        gtk::Window {
            #[name="ground"]
            Ground {
                UserMove(orig, dest, promotion) => MovePlayed(orig, dest, promotion),
            },
            delete_event(_, _) => (Quit, Inhibit(false)),
        }
    }
}

fn main() {
    Win::run(()).unwrap();
}
