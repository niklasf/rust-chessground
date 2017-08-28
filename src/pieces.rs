use std::cmp::{min, max};
use std::f64::consts::PI;

use time::SteadyTime;

use gtk::prelude::*;
use gtk::DrawingArea;
use cairo::prelude::*;
use cairo::Context;
use rsvg::HandleExt;

use shakmaty::{Square, Piece, Bitboard, Board};

use util;
use util::ease_in_out_cubic;
use promotable::Promotable;
use ground::{BoardState};

const ANIMATE_DURATION: f64 = 0.2;

pub struct Pieces {
    board: Board,
    figurines: Vec<Figurine>,
}

pub struct Figurine {
    pub(crate) square: Square,
    pub(crate) piece: Piece,
    pub(crate) pos: (f64, f64),
    pub(crate) time: SteadyTime,
    pub(crate) fading: bool,
    replaced: bool,
    pub(crate) dragging: bool,
}

impl Figurine {
    fn pos(&self, now: SteadyTime) -> (f64, f64) {
        let end = util::square_to_inverted(self.square);
        if self.dragging {
            end
        } else if self.fading {
            self.pos
        } else {
            (ease_in_out_cubic(self.pos.0, end.0, self.elapsed(now), ANIMATE_DURATION),
             ease_in_out_cubic(self.pos.1, end.1, self.elapsed(now), ANIMATE_DURATION))
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
            base * ease_in_out_cubic(1.0, 0.0, self.elapsed(now), ANIMATE_DURATION)
        } else {
            base
        }
    }

    fn elapsed(&self, now: SteadyTime) -> f64 {
        (now - self.time).num_milliseconds() as f64 / 1000.0
    }

    fn is_animating(&self, now: SteadyTime) -> bool {
        !self.dragging && self.elapsed(now) <= ANIMATE_DURATION &&
        (self.fading || self.pos != util::square_to_inverted(self.square))
    }

    fn queue_animation(&self, state: &BoardState, widget: &DrawingArea) {
        if self.is_animating(state.now) {
            let matrix = util::compute_matrix(widget, state.orientation);
            let pos = self.pos(state.now);

            let (x1, y1) = matrix.transform_point(pos.0 - 0.5, pos.1 - 0.5);
            let (x2, y2) = matrix.transform_point(pos.0 + 0.5, pos.1 + 0.5);
            let (x3, y3) = matrix.transform_point(self.square.file() as f64, 7.0 - self.square.rank() as f64);
            let (x4, y4) = matrix.transform_point(1.0 + self.square.file() as f64, 8.0 - self.square.rank() as f64);

            let xmin = min(
                min(x1.floor() as i32, x2.floor() as i32),
                min(x3.floor() as i32, x4.floor() as i32));
            let xmax = max(
                max(x1.ceil() as i32, x2.ceil() as i32),
                max(x3.ceil() as i32, x4.ceil() as i32));
            let ymin = min(
                min(y1.floor() as i32, y2.floor() as i32),
                min(y3.floor() as i32, y4.floor() as i32));
            let ymax = max(
                max(y1.ceil() as i32, y2.ceil() as i32),
                max(y3.ceil() as i32, y4.ceil() as i32));

            widget.queue_draw_area(xmin, ymin, xmax - xmin, ymax - ymin);
        }
    }

    fn render(&self, cr: &Context, board_state: &BoardState, promotable: &Promotable) {
        // hide piece while promotion dialog is open
        if promotable.is_promoting(self.square) {
            return;
        }

        cr.push_group();

        let (x, y) = self.pos(board_state.now);
        cr.translate(x, y);
        cr.rotate(board_state.orientation.fold(0.0, PI));
        cr.translate(-0.5, -0.5);
        cr.scale(board_state.piece_set.scale(), board_state.piece_set.scale());

        board_state.piece_set.by_piece(&self.piece).render_cairo(cr);

        cr.pop_group_to_source();
        cr.paint_with_alpha(self.alpha(board_state.now));
    }
}


impl Pieces {
    pub fn new() -> Pieces {
        Pieces::new_from_board(&Board::new())
    }

    pub fn new_from_board(board: &Board) -> Pieces {
        Pieces {
            board: board.clone(),
            figurines: board.pieces().map(|(square, piece)| Figurine {
                square,
                piece,
                pos: (0.5 + square.file() as f64, 7.5 - square.rank() as f64),
                time: SteadyTime::now(),
                fading: false,
                replaced: false,
                dragging: false,
            }).collect()
        }
    }

    pub fn set_board(&mut self, board: Board) {
        let now = SteadyTime::now();

        // clean and freeze previous animation
        self.figurines.retain(|f| f.alpha(now) > 0.0001);
        for figurine in &mut self.figurines {
            if !figurine.fading {
                figurine.pos = figurine.pos(now);
                figurine.time = now;
            }
        }

        // diff
        let mut removed = Bitboard(0);
        let mut added = Vec::new();

        for square in self.board.occupied() | board.occupied() {
            let old = self.board.piece_at(square);
            let new = board.piece_at(square);
            if old != new {
                if old.is_some() {
                    removed.add(square);
                }
                if let Some(new) = new {
                    added.push((square, new));
                }
            }
        }

        // try to match additions and removals
        let mut matched = Vec::new();
        added.retain(|&(square, piece)| {
            let best = removed
                .filter(|sq| self.board.by_piece(piece).contains(*sq))
                .min_by_key(|sq| sq.distance(square));

            if let Some(best) = best {
                removed.remove(best);
                matched.push((best, square));
                false
            } else {
                true
            }
        });

        for square in removed {
            for figurine in &mut self.figurines {
                if !figurine.fading && figurine.square == square {
                    figurine.fading = true;
                    figurine.replaced = board.occupied().contains(square);
                    figurine.time = now;
                }
            }
        }

        for (orig, dest) in matched {
            if let Some(figurine) = self.figurines.iter_mut().find(|f| !f.fading && f.square == orig) {
                figurine.square = dest;
                figurine.time = now;
            }
        }

        for (square, piece) in added {
            self.figurines.push(Figurine {
                square: square,
                piece: piece,
                pos: (0.5 + square.file() as f64, 7.5 - square.rank() as f64),
                time: now,
                fading: false,
                replaced: false,
                dragging: false,
            });
        }

        self.board = board;
    }

    pub fn occupied(&self) -> Bitboard {
        self.board.occupied()
    }

    pub(crate) fn render(&self, cr: &Context, state: &BoardState, promotable: &Promotable) {
        let now = SteadyTime::now();

        for figurine in &self.figurines {
            if figurine.fading {
                figurine.render(cr, state, promotable);
            }
        }

        for figurine in &self.figurines {
            if !figurine.fading && !figurine.is_animating(now) {
                figurine.render(cr, state, promotable);
            }
        }

        for figurine in &self.figurines {
            if !figurine.fading && figurine.is_animating(now) {
                figurine.render(cr, state, promotable);
            }
        }
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

    pub(crate) fn queue_animation(&self, state: &BoardState, widget: &DrawingArea) {
        for figurine in &self.figurines {
            figurine.queue_animation(state, widget);
        }
    }
}
