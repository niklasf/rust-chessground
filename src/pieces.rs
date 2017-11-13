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

use option_filter::OptionFilterExt;

use time::SteadyTime;

use gdk::EventButton;
use cairo::prelude::*;
use cairo::Context;
use rsvg::HandleExt;

use shakmaty::{Square, Piece, Bitboard, Board};

use util::{ease, pos_to_square, square_to_pos};
use promotable::Promotable;
use boardstate::BoardState;
use ground::{GroundMsg, EventContext, WidgetContext};

pub struct Pieces {
    figurines: Vec<Figurine>,
    selected: Option<Square>,
    drag: Option<Drag>,
    past: SteadyTime,
}

struct Drag {
    square: Square,
    piece: Piece,
    start: (f64, f64),
    pos: (f64, f64),
    threshold: bool,
}

pub struct Figurine {
    square: Square,
    piece: Piece,
    start: (f64, f64),
    elapsed: f64,
    time: SteadyTime,
    last_drag: SteadyTime,
    fading: bool,
    replaced: bool,
    dragging: bool,
}

impl Pieces {
    pub fn new() -> Pieces {
        Pieces::new_from_board(&Board::new())
    }

    pub fn new_from_board(board: &Board) -> Pieces {
        let now = SteadyTime::now();

        Pieces {
            selected: None,
            drag: None,
            past: now,
            figurines: board.pieces().map(|(square, piece)| Figurine {
                square,
                piece,
                start: (0.5 + f64::from(square.file()), 7.5 - f64::from(square.rank())),
                elapsed: 0.0,
                time: now,
                last_drag: now,
                fading: false,
                replaced: false,
                dragging: false,
            }).collect(),
        }
    }

    pub fn set_board(&mut self, board: &Board) {
        // clean faded figurines
        let now = SteadyTime::now();
        self.figurines.retain(|f| !f.fading || f.alpha() > 0.0001);

        // diff
        let mut added: Vec<_> = board.pieces().filter(|&(sq, piece)| {
            self.figurine_at(sq).map_or(true, |f| f.piece != piece)
        }).collect();

        for figurine in &mut self.figurines {
            if figurine.fading {
                continue;
            }

            // figurine was removed from the square
            if !board.by_piece(figurine.piece).contains(figurine.square) {
                // checkpoint animation
                figurine.start = figurine.pos();
                figurine.elapsed = 0.0;
                figurine.time = now;

                // cancel drag
                if figurine.dragging {
                    figurine.dragging = false;
                    self.drag = None;
                }

                let best = added
                    .iter()
                    .filter(|&&(_, p)| p == figurine.piece)
                    .min_by_key(|&&(sq, _)| figurine.square.distance(sq))
                    .map(|&(sq, _)| sq);

                if let Some(best) = best {
                    // found a close square it could have moved to
                    figurine.square = best;
                    added.retain(|&(sq, _)| sq != best);

                    // snap dragged figurine to square
                    if (now - figurine.last_drag).num_milliseconds() < 200 {
                        figurine.start = square_to_pos(figurine.square);
                    }
                } else {
                    // fade it out
                    figurine.fading = true;
                    figurine.replaced = board.occupied().contains(figurine.square);
                }
            }
        }

        // add new figurines
        for (square, piece) in added {
            self.figurines.push(Figurine {
                square: square,
                piece: piece,
                start: (0.5 + f64::from(square.file()), 7.5 - f64::from(square.rank())),
                elapsed: 0.0,
                time: now,
                last_drag: self.past,
                fading: false,
                replaced: false,
                dragging: false,
            });
        }
    }

    pub fn occupied(&self) -> Bitboard {
        self.figurines.iter().filter(|f| !f.fading).map(|f| f.square).collect()
    }

    pub fn figurine_at(&self, square: Square) -> Option<&Figurine> {
        self.figurines.iter().find(|f| !f.fading && f.square == square)
    }

    pub fn figurine_at_mut(&mut self, square: Square) -> Option<&mut Figurine> {
        self.figurines.iter_mut().find(|f| !f.fading && f.square == square)
    }

    pub fn dragging_mut(&mut self) -> Option<&mut Figurine> {
        self.figurines.iter_mut().find(|f| f.dragging)
    }

    pub(crate) fn selection_mouse_down(&mut self, ctx: &EventContext, e: &EventButton) {
        let orig = self.selected.take();

        if e.get_button() == 1 {
            let dest = ctx.square();
            self.selected = dest.filter(|sq| self.occupied().contains(*sq));

            if let (Some(orig), Some(dest)) = (orig, dest) {
                self.selected = None;
                if orig != dest {
                    ctx.stream().emit(GroundMsg::UserMove(orig, dest, None));
                }
            }
        }

        ctx.widget().queue_draw();
    }

    pub(crate) fn drag_mouse_down(&mut self, ctx: &EventContext, e: &EventButton) {
        if e.get_button() == 1 {
            if let Some(square) = ctx.square() {
                let piece = if let Some(figurine) = self.figurine_at_mut(square) {
                    figurine.dragging = true;
                    figurine.piece
                } else {
                    return;
                };

                self.drag = Some(Drag {
                    square,
                    piece,
                    start: ctx.pos(),
                    pos: ctx.pos(),
                    threshold: false,
                });
            }
        }
    }

    pub(crate) fn drag_mouse_move(&mut self, ctx: &EventContext) {
        if let Some(ref mut drag) = self.drag {
            ctx.widget().queue_draw_rect(drag.pos.0 - 0.5, drag.pos.1 - 0.5, 1.0, 1.0);
            pos_to_square(drag.pos).map(|sq| ctx.widget().queue_draw_square(sq));
            drag.pos = ctx.pos();
            ctx.widget().queue_draw_rect(drag.pos.0 - 0.5, drag.pos.1 - 0.5, 1.0, 1.0);
            pos_to_square(drag.pos).map(|sq| ctx.widget().queue_draw_square(sq));

            let (dx, dy) = (drag.start.0 - drag.pos.0, drag.start.1 - drag.pos.1);
            let (pdx, pdy) = ctx.widget().matrix().transform_distance(dx, dy);
            drag.threshold |= dx.hypot(dy) >= 0.1 || pdx.hypot(pdy) >= 4.0;

            if drag.threshold {
                // ensure orig square is selected
                if self.selected != Some(drag.square) {
                  self.selected = Some(drag.square);
                  ctx.widget().queue_draw();
                } else {
                  ctx.widget().queue_draw_square(drag.square);
                }
            }
        }
    }

    pub(crate) fn drag_mouse_up(&mut self, ctx: &EventContext) {
        let (orig, dest) = if let Some(drag) = self.drag.take() {
            ctx.widget().queue_draw();

            if let Some(ref mut figurine) = self.dragging_mut() {
                figurine.last_drag = SteadyTime::now();
                figurine.dragging = false;
            }

            let dest = ctx.square().unwrap_or(drag.square);

            if drag.square != dest {
                (drag.square, dest)
            } else {
                return;
            }
        } else {
            return;
        };

        self.selected = None;

        if orig != dest {
            ctx.stream().emit(GroundMsg::UserMove(orig, dest, None));
        }
    }

    pub(crate) fn queue_animation(&mut self, ctx: &WidgetContext) {
        for figurine in &mut self.figurines {
            figurine.queue_animation(ctx);
        }
    }

    pub(crate) fn draw(&self, cr: &Context, state: &BoardState, promotable: &Promotable) {
        self.draw_selection(cr, state);
        self.draw_move_hints(cr, state);

        for figurine in &self.figurines {
            if figurine.fading {
                self.draw_figurine(cr, figurine, state, promotable);
            }
        }

        for figurine in &self.figurines {
            if !figurine.fading && figurine.elapsed >= 1.0 {
                self.draw_figurine(cr, figurine, state, promotable);
            }
        }

        for figurine in &self.figurines {
            if !figurine.fading && figurine.elapsed < 1.0 {
                self.draw_figurine(cr, figurine, state, promotable);
            }
        }
    }

    fn draw_figurine(&self, cr: &Context, figurine: &Figurine, state: &BoardState, promotable: &Promotable) {
        // hide piece while promotion dialog is open
        if promotable.is_promoting(figurine.square) {
            return;
        }

        // draw ghost when dragging
        let dragging =
            figurine.dragging &&
            self.drag.as_ref().map_or(false, |d| d.threshold && d.square == figurine.square);

        cr.push_group();

        let (x, y) = figurine.pos();
        cr.translate(x, y);
        cr.rotate(state.orientation().fold(0.0, PI));
        cr.translate(-0.5, -0.5);
        cr.scale(state.piece_set().scale(), state.piece_set().scale());

        state.piece_set().by_piece(&figurine.piece).render_cairo(cr);

        cr.pop_group_to_source();

        cr.paint_with_alpha(if dragging { 0.2 } else { figurine.alpha() });
    }

    fn draw_selection(&self, cr: &Context, state: &BoardState) {
        if let Some(selected) = self.selected {
            cr.rectangle(f64::from(selected.file()), 7.0 - f64::from(selected.rank()), 1.0, 1.0);
            cr.set_source_rgba(0.08, 0.47, 0.11, 0.5);
            cr.fill();

            if let Some(hovered) = self.drag.as_ref().and_then(|d| pos_to_square(d.pos)) {
                if state.valid_move(selected, hovered) {
                    cr.rectangle(f64::from(hovered.file()), 7.0 - f64::from(hovered.rank()), 1.0, 1.0);
                    cr.set_source_rgba(0.08, 0.47, 0.11, 0.25);
                    cr.fill();
                }
            }
        }
    }

    fn draw_move_hints(&self, cr: &Context, state: &BoardState) {
        if let Some(selected) = self.selected {
            cr.set_source_rgba(0.08, 0.47, 0.11, 0.5);

            let radius = 0.12;
            let corner = 1.8 * radius;

            for square in state.move_targets(selected) {
                if self.occupied().contains(square) {
                    cr.move_to(f64::from(square.file()), 7.0 - f64::from(square.rank()));
                    cr.rel_line_to(corner, 0.0);
                    cr.rel_line_to(-corner, corner);
                    cr.rel_line_to(0.0, -corner);
                    cr.fill();

                    cr.move_to(1.0 + f64::from(square.file()), 7.0 - f64::from(square.rank()));
                    cr.rel_line_to(0.0, corner);
                    cr.rel_line_to(-corner, -corner);
                    cr.rel_line_to(corner, 0.0);
                    cr.fill();

                    cr.move_to(f64::from(square.file()), 8.0 - f64::from(square.rank()));
                    cr.rel_line_to(corner, 0.0);
                    cr.rel_line_to(-corner, -corner);
                    cr.rel_line_to(0.0, corner);
                    cr.fill();

                    cr.move_to(1.0 + f64::from(square.file()), 8.0 - f64::from(square.rank()));
                    cr.rel_line_to(-corner, 0.0);
                    cr.rel_line_to(corner, -corner);
                    cr.rel_line_to(0.0, corner);
                    cr.fill();
                } else {
                    cr.arc(0.5 + f64::from(square.file()),
                           7.5 - f64::from(square.rank()),
                           radius, 0.0, 2.0 * PI);
                    cr.fill();
                }
            }
        }
    }

    pub(crate) fn draw_drag(&self, cr: &Context, state: &BoardState) {
        match self.drag {
            Some(ref drag) if drag.threshold => {
                cr.push_group();
                cr.translate(drag.pos.0, drag.pos.1);
                cr.rotate(state.orientation().fold(0.0, PI));
                cr.translate(-0.5, -0.5);
                cr.scale(state.piece_set().scale(), state.piece_set().scale());
                state.piece_set().by_piece(&drag.piece).render_cairo(cr);
                cr.pop_group_to_source();
                cr.paint();
            }
            _ => {}
        }
    }
}

impl Figurine {
    pub fn piece(&self) -> &Piece {
        &self.piece
    }

    pub fn set_pos(&mut self, pos: (f64, f64)) {
        self.start = pos;
        self.time = SteadyTime::now();
        self.elapsed = 0.0;
    }

    fn pos(&self) -> (f64, f64) {
        if self.fading {
            self.start
        } else {
            let end = square_to_pos(self.square);
            (ease(self.start.0, end.0, self.elapsed), ease(self.start.1, end.1, self.elapsed))
        }
    }

    fn alpha(&self) -> f64 {
        if self.replaced {
            ease(0.5, 0.0, self.elapsed)
        } else if self.fading {
            ease(1.0, 0.0, self.elapsed)
        } else {
            1.0
        }
    }

    fn queue_animation(&mut self, ctx: &WidgetContext) {
        if self.elapsed < 1.0 {
            let pos = self.pos();
            ctx.queue_draw_rect(pos.0 - 0.5, pos.1 - 0.5, 1.0, 1.0);

            let now = SteadyTime::now();
            self.elapsed = ((now - self.time).num_milliseconds() as f64 / 300.0).min(1.0);

            let pos = self.pos();
            ctx.queue_draw_rect(pos.0 - 0.5, pos.1 - 0.5, 1.0, 1.0);
        }
    }
}
