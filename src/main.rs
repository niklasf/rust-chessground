extern crate gtk;
extern crate chessground;
#[macro_use]
extern crate relm;
#[macro_use]
extern crate relm_derive;
extern crate shakmaty;
extern crate option_filter;

use option_filter::OptionFilterExt;

use gtk::prelude::*;
use gtk::{Window, WindowType};
use relm::{Relm, Update, Widget, Component, ContainerWidget};

use shakmaty::{Chess, Position, MoveList, Setup};

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

        connect!(ground @ GroundMsg::UserMove { orig, dest, promotion }, ground, {
            println!("got user move {} {} {:?}", orig, dest, promotion);

            let pos = Chess::default();
            let mut legals = MoveList::new();
            pos.legal_moves(&mut legals);

            let m = legals.drain(..).find(|m| m.from() == Some(orig) && m.to() == dest && m.promotion() == promotion);
            let pos = if let Some(m) = m {
                pos.clone().play_unchecked(&m)
            } else {
                pos
            };

            legals.clear();
            pos.legal_moves(&mut legals);

            GroundMsg::SetPosition {
                board: pos.board().clone(),
                legals,
                last_move: None,
                check: pos.board().king_of(pos.turn()).filter(|_| pos.checkers().any()),
            }

                /* self.pieces.set_board(self.pos.board());
                self.last_move = Some((m.to(), m.from().unwrap_or_else(|| m.to())));

                // respond
                self.legals.clear();
                self.pos.legal_moves(&mut self.legals);
                if !self.legals.is_empty() {
                    let mut rng = rand::thread_rng();
                    let idx = Range::new(0, self.legals.len()).ind_sample(&mut rng);
                    let m = &self.legals[idx];
                    self.pos = self.pos.clone().play_unchecked(m);
                    self.pieces.set_board(self.pos.board());
                    self.last_move = Some((m.to(), m.from().unwrap_or_else(|| m.to())));
                }
            }

            self.legals.clear();
            self.pos.legal_moves(&mut self.legals);
            self.check = self.pos.board().king_of(self.pos.turn()).filter(|_| self.pos.checkers().any()); */

        });

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
