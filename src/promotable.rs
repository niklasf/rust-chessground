// This file is part of the chessground library.
// Copyright (C) 2017 Niklas Fiekas <niklas.fiekas@backscattering.de>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use std::f64::consts::PI;

use time::SteadyTime;

use gtk::prelude::*;
use cairo::Context;
use rsvg::HandleExt;

use shakmaty::{Square, Rank, Color, Role, MoveList};

use util::{ease, square_to_pos};
use pieces::Pieces;
use boardstate::BoardState;
use ground::{WidgetContext, EventContext, GroundMsg};

pub struct Promotable {
    promoting: Option<Promoting>,
}

struct Promoting {
    color: Color,
    orig: Square,
    dest: Square,
    hover: Option<Hover>,
}

struct Hover {
    square: Square,
    since: SteadyTime,
    elapsed: f64,
}

impl Promotable {
    pub fn new() -> Promotable {
        Promotable {
            promoting: None,
        }
    }

    pub fn start(&mut self, color: Color, orig: Square, dest: Square) {
        self.promoting = Some(Promoting {
            color,
            orig,
            dest,
            hover: Some(Hover {
                square: dest,
                since: SteadyTime::now(),
                elapsed: 0.0,
            }),
        });
    }

    pub fn cancel(&mut self) {
        self.promoting = None;
    }

    pub fn update(&mut self, legals: &MoveList) {
        let cancel = if let Some(ref promoting) = self.promoting {
            !legals.iter().any(|m| {
                m.from() == Some(promoting.orig) && m.to() == promoting.dest &&
                m.promotion().is_some()
            })
        } else {
            false
        };

        if cancel {
            self.cancel();
        }
    }

    pub fn is_promoting(&self, orig: Square) -> bool {
        self.promoting.as_ref().map_or(false, |p| p.orig == orig)
    }

    pub(crate) fn queue_animation(&mut self, ctx: &WidgetContext) {
        if let Some(Promoting { hover: Some(ref mut hover), .. }) = self.promoting {
            if hover.elapsed < 1.0 {
                ctx.queue_draw_square(hover.square);
            }

            hover.elapsed = ((SteadyTime::now() - hover.since).num_milliseconds() as f64 / 1000.0).min(1.0);
        }
    }

    pub(crate) fn mouse_move(&mut self, ctx: &EventContext) {
        if let Some(ref mut promoting) = self.promoting {
            let previous = promoting.hover.as_ref().map(|h| h.square);
            let square = ctx.square().filter(|sq| sq.file() == promoting.dest.file());

            if square != previous {
                if let Some(sq) = previous {
                    ctx.widget().queue_draw_square(sq);
                }
                if let Some(sq) = square {
                    ctx.widget().queue_draw_square(sq);
                }

                promoting.hover = square.map(|square| Hover {
                    square,
                    since: SteadyTime::now(),
                    elapsed: 0.0,
                });
            }
        }
    }

    pub(crate) fn mouse_down(&mut self, pieces: &mut Pieces, ctx: &EventContext) -> Inhibit {
        if let Some(promoting) = self.promoting.take() {
            ctx.widget().queue_draw();

            if let Some(figurine) = pieces.figurine_at_mut(promoting.orig) {
                // animate the figurine when cancelling
                figurine.set_pos(square_to_pos(promoting.dest));
            }

            if let Some(square) = ctx.square() {
                let side = promoting.orientation();
                let base = i8::from(promoting.dest.rank());

                if square.file() == promoting.dest.file() {
                    let role = match i8::from(square.rank()) {
                        r if r == base => Some(Role::Queen),
                        r if r == base + side.fold(-1, 1) => Some(Role::Rook),
                        r if r == base + side.fold(-2, 2) => Some(Role::Bishop),
                        r if r == base + side.fold(-3, 3) => Some(Role::Knight),
                        r if r == base + side.fold(-4, 4) => Some(Role::King),
                        r if r == base + side.fold(-5, 5) => Some(Role::Pawn),
                        _ => None,
                    };

                    if role.is_some() {
                        ctx.stream().emit(GroundMsg::UserMove(promoting.orig, promoting.dest, role));
                        return Inhibit(true);
                    }
                }
            }
        }

        Inhibit(false)
    }

    pub(crate) fn draw(&self, cr: &Context, state: &BoardState) -> Result<(), cairo::Error> {
        if let Some(ref p) = self.promoting {
            p.draw(cr, state)?;
        }

        Ok(())
    }
}

impl Promoting {
    fn orientation(&self) -> Color {
        Color::from_white(self.dest.rank() > Rank::Fourth)
    }

    fn draw(&self, cr: &Context, state: &BoardState) -> Result<(), cairo::Error> {
        // make the board darker
        cr.rectangle(0.0, 0.0, 8.0, 8.0);
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.5);
        cr.fill()?;

        for (offset, role) in [Role::Queen, Role::Rook, Role::Bishop, Role::Knight, Role::King, Role::Pawn].iter().enumerate() {
            if !state.legal_move(self.orig, self.dest, Some(*role)) {
                continue;
            }

            let rank = i8::from(self.dest.rank()) - self.orientation().fold(offset as i8, -(offset as i8));
            let light = (i8::from(self.dest.file()) + rank) & 1 == 1;

            cr.save()?;
            cr.rectangle(f64::from(self.dest.file()), 7.0 - f64::from(rank), 1.0, 1.0);

            // draw background
            if light {
                cr.set_source_rgb(0.25, 0.25, 0.25);
            } else {
                cr.set_source_rgb(0.18, 0.18, 0.18);
            }
            cr.fill_preserve()?;
            cr.clip();

            // draw piece
            let radius = match self.hover {
                Some(ref hover) if i8::from(hover.square.rank()) == rank => {
                    cr.set_source_rgb(ease(0.69, 1.0, hover.elapsed),
                                      ease(0.69, 0.65, hover.elapsed),
                                      ease(0.69, 0.0, hover.elapsed));

                    ease(0.5, 0.5f64.hypot(0.5), hover.elapsed)
                },
                _ => {
                    cr.set_source_rgb(0.69, 0.69, 0.69);
                    0.5
                },
            };

            cr.arc(0.5 + f64::from(self.dest.file()), 7.5 - f64::from(rank), radius, 0.0, 2.0 * PI);
            cr.fill()?;

            cr.translate(0.5 + f64::from(self.dest.file()), 7.5 - f64::from(rank));
            cr.scale(2f64.sqrt() * radius, 2f64.sqrt() * radius);
            cr.rotate(state.orientation().fold(0.0, PI));
            cr.translate(-0.5, -0.5);
            cr.scale(state.piece_set().scale(), state.piece_set().scale());
            state.piece_set().by_piece(&role.of(self.color)).render_cairo(cr);

            cr.restore()?;
        }

        Ok(())
    }
}
