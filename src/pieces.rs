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

const ANIMATE_DURATION: f64 = 0.2;

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
}

impl Pieces {
    pub fn new() -> Pieces {
        Pieces::new_from_board(&Board::new())
    }

    pub fn new_from_board(board: &Board) -> Pieces {
        let now = SteadyTime::now();

        Pieces {
            selected: None,
            drag_start: None,
            past: now,
            figurines: board.pieces().map(|(square, piece)| Figurine {
                square,
                piece,
                pos: (0.5 + square.file() as f64, 7.5 - square.rank() as f64),
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
        self.figurines.retain(|f| !f.fading || f.alpha(now) > 0.0001);

        // diff
        let mut added: Vec<_> = board.pieces().filter(|&(sq, piece)| {
            self.figurine_at(sq).map_or(true, |f| f.piece != piece)
        }).collect();

        for figurine in &mut self.figurines {
            if figurine.fading {
                continue;
            }

            // checkpoint animation
            figurine.pos = figurine.pos(now);
            figurine.time = now;

            // figurine was removed from the square
            if !board.by_piece(figurine.piece).contains(figurine.square) {
                let best = added
                    .iter()
                    .filter(|&&(_, p)| p == figurine.piece)
                    .min_by_key(|&&(sq, _)| figurine.square.distance(sq))
                    .map(|&(sq, _)| sq);

                if let Some(best) = best {
                    // found a close square it could have moved to
                    figurine.square = best;
                    figurine.time = now;
                    added.retain(|&(sq, _)| sq != best);

                    // snap dragged figurine to square
                    if (now - figurine.last_drag).num_milliseconds() < 200 {
                        figurine.pos = square_to_pos(figurine.square);
                    }
                } else {
                    // fade it out
                    figurine.fading = true;
                    figurine.replaced = board.occupied().contains(figurine.square);
                    figurine.time = now;
                }
            }
        }

        // add new figurines
        for (square, piece) in added {
            self.figurines.push(Figurine {
                square: square,
                piece: piece,
                pos: (0.5 + square.file() as f64, 7.5 - square.rank() as f64),
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

    pub fn dragging(&self) -> Option<&Figurine> {
        self.figurines.iter().find(|f| f.dragging)
    }

    pub fn dragging_mut(&mut self) -> Option<&mut Figurine> {
        self.figurines.iter_mut().find(|f| f.dragging)
    }

    pub fn is_animating(&self, now: SteadyTime) -> bool {
        self.figurines.iter().any(|f| f.is_animating(now))
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
                if self.occupied().contains(square) {
                    self.drag_start = Some(DragStart {
                        pos: ctx.pos(),
                        square,
                    });
                }
            }
        }
    }

    pub(crate) fn drag_mouse_move(&mut self, ctx: &EventContext) {
        let dragging = if let Some(ref start) = self.drag_start {
            let pos = ctx.pos();
            let (dx, dy) = (start.pos.0 - pos.0, start.pos.1 - pos.1);
            let (pdx, pdy) = ctx.widget().matrix().transform_distance(dx, dy);
            Some(start.square).filter(|_| {
                dx.hypot(dy) >= 0.1 || pdx.hypot(pdy) >= 4.0
            })
        } else {
            None
        };

        if let Some(square) = dragging {
            // mark figurine as beeing dragged to show the shadow
            if let Some(figurine) = self.figurine_at_mut(square) {
                figurine.dragging = true;
            }

            // ensure orig square is selected
            if self.selected != dragging {
                self.selected = dragging;
                ctx.widget().queue_draw();
            }
        }

        if let Some(dragging) = self.dragging_mut() {
            // invalidate previous
            ctx.widget().queue_draw_rect(dragging.pos.0 - 0.5, dragging.pos.1 - 0.5, 1.0, 1.0);
            ctx.widget().queue_draw_square(dragging.square);
            if let Some(sq) = pos_to_square(dragging.pos) {
                ctx.widget().queue_draw_square(sq);
            }

            // update position
            dragging.pos = ctx.pos();
            dragging.time = SteadyTime::now();

            // invalidate new
            ctx.widget().queue_draw_rect(dragging.pos.0 - 0.5, dragging.pos.1 - 0.5, 1.0, 1.0);
            if let Some(sq) = ctx.square() {
                ctx.widget().queue_draw_square(sq);
            }
        }
    }

    pub(crate) fn drag_mouse_up(&mut self, ctx: &EventContext) {
        self.drag_start = None;

        let (orig, dest) = if let Some(dragging) = self.dragging_mut() {
            ctx.widget().queue_draw();

            let dest = ctx.square().unwrap_or(dragging.square);

            let now = SteadyTime::now();
            dragging.time = now;
            dragging.last_drag = now;
            dragging.pos = square_to_pos(dragging.square);
            dragging.dragging = false;

            if dragging.square != dest && !dragging.fading {
                (dragging.square, dest)
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

    pub(crate) fn queue_animation(&self, ctx: &WidgetContext) {
        for figurine in &self.figurines {
            figurine.queue_animation(ctx);
        }
    }

    pub(crate) fn draw(&self, cr: &Context, now: SteadyTime, state: &BoardState, promotable: &Promotable) {
        self.draw_selection(cr, state);
        self.draw_move_hints(cr, state);

        for figurine in &self.figurines {
            if figurine.fading {
                figurine.draw(cr, now, state, promotable);
            }
        }

        for figurine in &self.figurines {
            if !figurine.fading && !figurine.is_animating(now) {
                figurine.draw(cr, now, state, promotable);
            }
        }

        for figurine in &self.figurines {
            if !figurine.fading && figurine.is_animating(now) {
                figurine.draw(cr, now, state, promotable);
            }
        }
    }

    fn draw_selection(&self, cr: &Context, state: &BoardState) {
        if let Some(selected) = self.selected {
            cr.rectangle(selected.file() as f64, 7.0 - selected.rank() as f64, 1.0, 1.0);
            cr.set_source_rgba(0.08, 0.47, 0.11, 0.5);
            cr.fill();

            if let Some(hovered) = self.dragging().and_then(|d| pos_to_square(d.pos)) {
                if state.valid_move(selected, hovered) {
                    cr.rectangle(hovered.file() as f64, 7.0 - hovered.rank() as f64, 1.0, 1.0);
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
                    cr.move_to(square.file() as f64, 7.0 - square.rank() as f64);
                    cr.rel_line_to(corner, 0.0);
                    cr.rel_line_to(-corner, corner);
                    cr.rel_line_to(0.0, -corner);
                    cr.fill();

                    cr.move_to(1.0 + square.file() as f64, 7.0 - square.rank() as f64);
                    cr.rel_line_to(0.0, corner);
                    cr.rel_line_to(-corner, -corner);
                    cr.rel_line_to(corner, 0.0);
                    cr.fill();

                    cr.move_to(square.file() as f64, 8.0 - square.rank() as f64);
                    cr.rel_line_to(corner, 0.0);
                    cr.rel_line_to(-corner, -corner);
                    cr.rel_line_to(0.0, corner);
                    cr.fill();

                    cr.move_to(1.0 + square.file() as f64, 8.0 - square.rank() as f64);
                    cr.rel_line_to(-corner, 0.0);
                    cr.rel_line_to(corner, -corner);
                    cr.rel_line_to(0.0, corner);
                    cr.fill();
                } else {
                    cr.arc(0.5 + square.file() as f64,
                           7.5 - square.rank() as f64,
                           radius, 0.0, 2.0 * PI);
                    cr.fill();
                }
            }
        }
    }

    pub(crate) fn draw_drag(&self, cr: &Context, now: SteadyTime, state: &BoardState) {
        if let Some(dragging) = self.dragging() {
            cr.push_group();
            cr.translate(dragging.pos.0, dragging.pos.1);
            cr.rotate(state.orientation().fold(0.0, PI));
            cr.translate(-0.5, -0.5);
            cr.scale(state.piece_set().scale(), state.piece_set().scale());
            state.piece_set().by_piece(&dragging.piece).render_cairo(cr);
            cr.pop_group_to_source();
            cr.paint_with_alpha(dragging.drag_alpha(now));
        }
    }
}

impl Figurine {
    pub fn set_pos(&mut self, pos: (f64, f64)) {
        self.pos = pos;
        self.time = SteadyTime::now();
    }

    fn pos(&self, now: SteadyTime) -> (f64, f64) {
        let end = square_to_pos(self.square);
        if self.dragging {
            end
        } else if self.fading {
            self.pos
        } else {
            (ease(self.pos.0, end.0, self.elapsed(now) / ANIMATE_DURATION),
             ease(self.pos.1, end.1, self.elapsed(now) / ANIMATE_DURATION))
        }
    }

    fn alpha(&self, now: SteadyTime) -> f64 {
        if self.dragging {
            0.2 * self.alpha_easing(1.0, now)
        } else {
            self.drag_alpha(now)
        }
    }

    pub fn drag_alpha(&self, now: SteadyTime) -> f64 {
        let base = if self.fading && self.replaced { 0.5 } else { 1.0 };
        self.alpha_easing(base, now)
    }

    fn alpha_easing(&self, base: f64, now: SteadyTime) -> f64 {
        if self.fading {
            base * ease(1.0, 0.0, self.elapsed(now) / ANIMATE_DURATION)
        } else {
            base
        }
    }

    fn elapsed(&self, now: SteadyTime) -> f64 {
        (now - self.time).num_milliseconds() as f64 / 1000.0
    }

    fn is_animating(&self, now: SteadyTime) -> bool {
        !self.dragging && self.elapsed(now) <= ANIMATE_DURATION &&
        (self.fading || self.pos != square_to_pos(self.square))
    }

    fn queue_animation(&self, ctx: &WidgetContext) {
        if self.is_animating(ctx.last_render()) {
            let pos = self.pos(ctx.last_render());
            ctx.queue_draw_rect(pos.0 - 0.5, pos.1 - 0.5, 1.0, 1.0);

            let pos = self.pos(ctx.now());
            ctx.queue_draw_rect(pos.0 - 0.5, pos.1 - 0.5, 1.0, 1.0);
        }
    }

    fn draw(&self, cr: &Context, now: SteadyTime, board_state: &BoardState, promotable: &Promotable) {
        // hide piece while promotion dialog is open
        if promotable.is_promoting(self.square) {
            return;
        }

        cr.push_group();

        let (x, y) = self.pos(now);
        cr.translate(x, y);
        cr.rotate(board_state.orientation().fold(0.0, PI));
        cr.translate(-0.5, -0.5);
        cr.scale(board_state.piece_set().scale(), board_state.piece_set().scale());

        board_state.piece_set().by_piece(&self.piece).render_cairo(cr);

        cr.pop_group_to_source();
        cr.paint_with_alpha(self.alpha(now));
    }
}
