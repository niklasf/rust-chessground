use std::f64::consts::PI;

use time::SteadyTime;

use gtk::prelude::*;
use gtk::DrawingArea;
use cairo::Context;
use rsvg::HandleExt;

use shakmaty::{Square, Color, Role};

use util;
use util::ease_in_out_cubic;
use ground::{EventContext, BoardState, GroundMsg};

pub struct Promotable {
    promoting: Option<Promoting>,
}

struct Promoting {
    orig: Square,
    dest: Square,
    hover: Option<Square>,
    time: SteadyTime,
}

impl Promoting {
    fn elapsed(&self, now: SteadyTime) -> f64 {
        (now - self.time).num_milliseconds() as f64 / 1000.0
    }

    fn orientation(&self) -> Color {
        Color::from_bool(self.dest.rank() > 4)
    }

    fn draw(&self, cr: &Context, board_state: &BoardState) {
        cr.rectangle(0.0, 0.0, 8.0, 8.0);
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.5);
        cr.fill();

        for (offset, role) in [Role::Queen, Role::Rook, Role::Bishop, Role::Knight, Role::King, Role::Pawn].iter().enumerate() {
            if !board_state.legals.iter().any(|m| {
                m.from() == Some(self.orig) &&
                m.to() == self.dest &&
                m.promotion() == Some(*role)
            }) {
                continue;
            }

            let rank = self.orientation().fold(7 - offset as i8, offset as i8);
            let light = self.dest.file() + rank & 1 == 1;

            cr.save();
            cr.rectangle(self.dest.file() as f64, 7.0 - rank as f64, 1.0, 1.0);
            cr.clip_preserve();

            if light {
                cr.set_source_rgb(0.25, 0.25, 0.25);
            } else {
                cr.set_source_rgb(0.18, 0.18, 0.18);
            }
            cr.fill();

            let radius = match self.hover {
                Some(hover) if hover.file() == self.dest.file() && hover.rank() == rank => {
                    cr.set_source_rgb(
                        ease_in_out_cubic(0.69, 1.0, self.elapsed(board_state.now), 1.0),
                        ease_in_out_cubic(0.69, 0.65, self.elapsed(board_state.now), 1.0),
                        ease_in_out_cubic(0.69, 0.0, self.elapsed(board_state.now), 1.0));

                    ease_in_out_cubic(0.5, 0.5f64.hypot(0.5), self.elapsed(board_state.now), 1.0)
                },
                _ => {
                    cr.set_source_rgb(0.69, 0.69, 0.69);
                    0.5
                },
            };

            cr.arc(0.5 + self.dest.file() as f64, 7.5 - rank as f64, radius, 0.0, 2.0 * PI);
            cr.fill();

            cr.translate(0.5 + self.dest.file() as f64, 7.5 - rank as f64);
            cr.scale(2f64.sqrt() * radius, 2f64.sqrt() * radius);
            cr.translate(-0.5, -0.5);
            cr.scale(board_state.piece_set.scale(), board_state.piece_set.scale());
            board_state.piece_set.by_piece(&role.of(Color::White)).render_cairo(cr);

            cr.restore();
        }
    }
}

impl Promotable {
    pub fn new() -> Promotable {
        Promotable {
            promoting: None,
        }
    }

    pub fn start_promoting(&mut self, orig: Square, dest: Square) {
        self.promoting = Some(Promoting {
            orig,
            dest,
            hover: Some(dest),
            time: SteadyTime::now(),
        });
    }

    pub fn is_promoting(&self, orig: Square) -> bool {
        self.promoting.as_ref().map_or(false, |p| p.orig == orig)
    }

    pub fn is_animating(&self) -> bool {
        if let Some(ref promoting) = self.promoting {
            false
            // TODO: promoting.hover.map_or(false, |h| h.since
        } else {
            false
        }
    }

    pub(crate) fn queue_animation(&self, board_state: &BoardState, drawing_area: &DrawingArea) {
        if let Some(Promoting { hover: Some(square), .. }) = self.promoting {
            // TODO: queue draw square
        }
    }

    pub(crate) fn mouse_move(&mut self, board_state: &BoardState, ctx: &EventContext) -> bool {
        self.queue_animation(board_state, ctx.drawing_area);

        let consume = if let Some(ref mut promoting) = self.promoting {
            if promoting.hover != ctx.square {
                promoting.hover = ctx.square;
                promoting.time = SteadyTime::now();
            }
            true
        } else {
            false
        };

        self.queue_animation(board_state, ctx.drawing_area);
        consume
    }

    pub(crate) fn mouse_down(&mut self, board_state: &mut BoardState, ctx: &EventContext) -> bool {
        if let Some(promoting) = self.promoting.take() {
            ctx.drawing_area.queue_draw();

            // animate the figurine when cancelling
            if let Some(figurine) = board_state.pieces.figurine_at_mut(promoting.orig) {
                figurine.pos = util::square_to_inverted(promoting.dest);
                figurine.time = SteadyTime::now();
            }

            if let Some(square) = ctx.square {
                let side = promoting.orientation();

                if square.file() == promoting.dest.file() {
                    let role = match square.rank() {
                        r if r == side.fold(7, 0) => Some(Role::Queen),
                        r if r == side.fold(6, 1) => Some(Role::Rook),
                        r if r == side.fold(5, 2) => Some(Role::Bishop),
                        r if r == side.fold(4, 3) => Some(Role::Knight),
                        r if r == side.fold(3, 4) => Some(Role::King),
                        r if r == side.fold(2, 5) => Some(Role::Pawn),
                        _ => None,
                    };

                    if role.is_some() {
                        ctx.stream.emit(GroundMsg::UserMove(promoting.orig, promoting.dest, role));
                        return true;
                    }
                }
            }
        }

        false
    }

    pub(crate) fn draw(&self, cr: &Context, board_state: &BoardState) {
        self.promoting.as_ref().map(|p| p.draw(cr, board_state));
    }
}
