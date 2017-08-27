extern crate gtk;
extern crate chessground;
#[macro_use]
extern crate relm;
#[macro_use]
extern crate relm_derive;
extern crate shakmaty;
extern crate option_filter;
extern crate rand;

use option_filter::OptionFilterExt;

use rand::distributions::{Range, IndependentSample};

use gtk::prelude::*;
use gtk::{Window, WindowType};
use relm::{Relm, Update, Widget, Component, ContainerWidget};

use shakmaty::{Square, Role, Chess, Position, MoveList, Setup};

use chessground::{Ground, GroundMsg};

#[derive(Msg)]
enum WinMsg {
    Quit,
    UserMove { orig: Square, dest: Square, promotion: Option<Role> },
}

struct Win {
    window: Window,
    ground: Component<Ground>,
    pos: Chess,
}

impl Update for Win {
    type Model = Chess;
    type ModelParam = ();
    type Msg = WinMsg;

    fn model(_: &Relm<Self>, _: ()) -> Self::Model {
        Chess::default()
    }

    fn update(&mut self, event: Self::Msg) {
        match event {
            WinMsg::Quit => {
                gtk::main_quit()
            },
            WinMsg::UserMove { orig, dest, promotion } => {
                let mut legals = MoveList::new();
                self.pos.legal_moves(&mut legals);

                let m = legals.drain(..).find(|m| {
                    m.from() == Some(orig) && m.to() == dest &&
                    m.promotion() == promotion
                });

                let last_move = if let Some(m) = m {
                    self.pos = self.pos.clone().play_unchecked(&m);
                    Some((m.from().unwrap_or_else(|| m.to()), m.to()))
                } else {
                    None
                };

                legals.clear();
                self.pos.legal_moves(&mut legals);

                let last_move = if !legals.is_empty() {
                    // respond with a random move
                    let mut rng = rand::thread_rng();
                    let idx = Range::new(0, legals.len()).ind_sample(&mut rng);
                    let m = &legals[idx];
                    self.pos = self.pos.clone().play_unchecked(m);
                    Some((m.from().unwrap_or_else(|| m.to()), m.to()))
                } else {
                    last_move
                };

                legals.clear();
                self.pos.legal_moves(&mut legals);

                self.ground.emit(GroundMsg::SetPosition {
                    board: self.pos.board().clone(),
                    legals,
                    last_move,
                    check: self.pos.board().king_of(self.pos.turn()).filter(|_| self.pos.checkers().any())
                });
            }
        }
    }
}

impl Widget for Win {
    type Root = Window;

    fn root(&self) -> Self::Root {
        self.window.clone()
    }

    fn view(relm: &Relm<Self>, pos: Self::Model) -> Self {
        let window = Window::new(WindowType::Toplevel);
        let ground = window.add_widget::<Ground, _>(relm, ());

        window.show_all();

        connect!(ground @ GroundMsg::UserMove { orig, dest, promotion }, relm, {
            println!("got user move {} {} {:?}", orig, dest, promotion);
            WinMsg::UserMove { orig, dest, promotion }
        });

        connect!(relm, window, connect_delete_event(_, _), return (Some(WinMsg::Quit), Inhibit(false)));

        Win {
            ground,
            pos,
            window,
        }
    }
}

fn main() {
    Win::run(()).unwrap();
}
