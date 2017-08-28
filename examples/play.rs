#![feature(proc_macro)]

extern crate gtk;
extern crate chessground;
#[macro_use]
extern crate relm;
extern crate relm_attributes;
#[macro_use]
extern crate relm_derive;

extern crate shakmaty;
extern crate rand;

use rand::distributions::{Range, IndependentSample};

use gtk::prelude::*;
use relm::Widget;
use relm_attributes::widget;

use shakmaty::{Square, Role, Chess, Position};
use chessground::{Ground, GroundMsg, UserMove, SetPos, Pos, Flip};

use self::Msg::*;

#[derive(Msg)]
pub enum Msg {
    Quit,
    MovePlayed(Square, Square, Option<Role>),
    ToGround(GroundMsg),
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
                    m
                } else {
                    return;
                };

                let legals = self.model.legals();
                let last_move = if !legals.is_empty() {
                    // respond with a random move
                    let random_index = Range::new(0, legals.len()).ind_sample(&mut rand::thread_rng());
                    let m = &legals[random_index];
                    self.model.play_unchecked(m);
                    m
                } else {
                    last_move
                };

                self.ground.emit(SetPos(Pos::new(&self.model).with_last_move(last_move)));
            },
            ToGround(msg) => {
                self.ground.emit(msg)
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
            key_press_event(_, key) => (ToGround(Flip), Inhibit(false)),
            delete_event(_, _) => (Quit, Inhibit(false)),
        }
    }
}

fn main() {
    Win::run(()).expect("initialized gtk");
}
