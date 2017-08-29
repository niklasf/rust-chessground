#![feature(proc_macro)]

extern crate gdk;
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

use gdk::enums::key::Key;
use gtk::prelude::*;
use relm::Widget;
use relm_attributes::widget;

use shakmaty::{Square, Role, Move, Chess, Position};
use chessground::{Ground, UserMove, SetPos, Pos, Flip};

use self::Msg::*;

#[derive(Msg)]
pub enum Msg {
    Quit,
    MovePlayed(Square, Square, Option<Role>),
    KeyPressed(Key),
}

#[derive(Default)]
pub struct Model {
    stack: Vec<Move>,
    switchyard: Vec<Move>,
    position: Chess,
}

impl Model {
    fn push(&mut self, m: &Move) {
        self.position.play_unchecked(m);
        self.stack.push(m.clone());
        self.switchyard.clear();
    }

    fn undo(&mut self) {
        self.stack.pop().map(|m| self.switchyard.push(m));
        self.replay();
    }

    fn redo(&mut self) {
        self.switchyard.pop().map(|m| {
            self.position.play_unchecked(&m);
            self.stack.push(m);
        });
    }

    fn replay(&mut self) {
        // replay
        self.position = Chess::default();
        for m in &self.stack {
            self.position.play_unchecked(m);
        }
    }

    fn pos(&self) -> Pos {
        let mut pos = Pos::new(&self.position);
        pos.set_last_move(self.stack.iter().last());
        pos
    }
}

#[widget]
impl Widget for Win {
    fn model() -> Model {
        Model::default()
    }

    fn update(&mut self, event: Msg) {
        match event {
            Quit => {
                gtk::main_quit()
            },
            MovePlayed(orig, dest, promotion) => {
                let legals = self.model.position.legals();
                let m = legals.iter().find(|m| {
                    m.from() == Some(orig) && m.to() == dest &&
                    m.promotion() == promotion
                });

                if let Some(m) = m {
                    self.model.push(m);
                } else {
                    return;
                };

                /* if !self.model.position.is_game_over() {
                    // respond with a random move
                    let legals = self.model.position.legals();
                    let random_index = Range::new(0, legals.len()).ind_sample(&mut rand::thread_rng());
                    self.model.push(&legals[random_index]);
                } */

                self.ground.emit(SetPos(self.model.pos()));
            },
            KeyPressed(key) if key == 'f' as Key => {
                self.ground.emit(Flip)
            },
            KeyPressed(key) if key == 'j' as Key => {
                self.model.undo();
                self.ground.emit(SetPos(self.model.pos()));
            },
            KeyPressed(key) if key == 'k' as Key => {
                self.model.redo();
                self.ground.emit(SetPos(self.model.pos()));
            },
            KeyPressed(_) => {},
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
            key_press_event(_, e) => (KeyPressed(e.get_keyval()), Inhibit(false)),
            delete_event(_, _) => (Quit, Inhibit(false)),
        }
    }
}

fn main() {
    Win::run(()).expect("initialized gtk");
}
