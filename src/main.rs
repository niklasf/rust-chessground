#![feature(proc_macro)]

extern crate gtk;
extern crate chessground;
#[macro_use]
extern crate relm;
extern crate relm_attributes;
#[macro_use]
extern crate relm_derive;

extern crate shakmaty;
extern crate option_filter;
extern crate rand;

use option_filter::OptionFilterExt;
use rand::distributions::{Range, IndependentSample};

use gtk::prelude::*;
use relm::Widget;
use relm_attributes::widget;

use shakmaty::{Square, Role, Chess, Position, Setup};
use chessground::{Ground, GroundMsg, UserMove};

use self::Msg::*;

#[derive(Msg)]
pub enum Msg {
    Quit,
    MovePlayed(Square, Square, Option<Role>),
}

#[widget]
impl Widget for Win {
    fn model() -> Chess {
        Chess::default()
    }

    fn update(&mut self, event: Msg) {
        match event {
            Quit => {
                gtk::main_quit()
            },
            MovePlayed(orig, dest, promotion) => {
                let legals = self.model.legals();
                let m = legals.iter().find(|m| {
                    m.from() == Some(orig) && m.to() == dest &&
                    m.promotion() == promotion
                });

                let last_move = if let Some(m) = m {
                    self.model.play_unchecked(&m);
                    Some((m.from().unwrap_or_else(|| m.to()), m.to()))
                } else {
                    return;
                };

                let last_move = if !self.model.is_game_over() {
                    // respond with a random move
                    let legals = self.model.legals();
                    let random_index = Range::new(0, legals.len()).ind_sample(&mut rand::thread_rng());
                    let m = &legals[random_index];
                    self.model.play_unchecked(m);
                    Some((m.from().unwrap_or_else(|| m.to()), m.to()))
                } else {
                    last_move
                };

                self.ground.emit(GroundMsg::SetPosition {
                    board: self.model.board().clone(),
                    legals: self.model.legals(),
                    last_move,
                    check: self.model.board()
                               .king_of(self.model.turn())
                               .filter(|_| self.model.checkers().any())
                });
            }
        }
    }

    view! {
        gtk::Window {
            gtk::Box {
                #[name="ground"]
                Ground {
                    UserMove(orig, dest, promotion) => MovePlayed(orig, dest, promotion),
                },
            },
            delete_event(_, _) => (Quit, Inhibit(false)),
        }
    }
}

fn main() {
    Win::run(()).expect("initialized gtk");
}
