use std::f64::consts::PI;

use option_filter::OptionFilterExt;

use cairo::prelude::*;
use cairo::{Context, RadialGradient};

use shakmaty::{Color, Square, Role, Bitboard, Chess, Position, MoveList};

use pieceset::PieceSet;

pub struct BoardState {
    pub(crate) orientation: Color,
    pub(crate) check: Option<Square>,
    pub(crate) last_move: Option<(Square, Square)>,
    pub(crate) piece_set: PieceSet,
    pub(crate) legals: MoveList,
}

impl BoardState {
    pub fn new() -> Self {
        BoardState::from_position(&Chess::default())
    }

    pub fn from_position<P: Position>(pos: &P) -> Self {
        BoardState {
            orientation: pos.turn(),
            check: pos.board().king_of(pos.turn()).filter(|_| pos.checkers().any()),
            last_move: None,
            piece_set: PieceSet::merida(),
            legals: pos.legals(),
        }
    }

    pub fn move_targets(&self, orig: Square) -> Bitboard {
        self.legals.iter().filter(|m| m.from() == Some(orig)).map(|m| m.to()).collect()
    }

    pub fn valid_move(&self, orig: Square, dest: Square) -> bool {
        self.move_targets(orig).contains(dest)
    }

    pub fn legal_move(&self, orig: Square, dest: Square, promotion: Option<Role>) -> bool {
        self.legals.iter().any(|m| {
            m.from() == Some(orig) && m.to() == dest && m.promotion() == promotion
        })
    }

    pub(crate) fn draw(&self, cr: &Context) {
        self.draw_border(cr);
        self.draw_board(cr);
        self.draw_last_move(cr);
        self.draw_check(cr);
    }

    fn draw_border(&self, cr: &Context) {
        cr.set_source_rgb(0.2, 0.2, 0.5);
        cr.rectangle(-0.5, -0.5, 9.0, 9.0);
        cr.fill();

        cr.set_font_size(0.20);
        cr.set_source_rgb(0.8, 0.8, 0.8);

        for (rank, glyph) in ["1", "2", "3", "4", "5", "6", "7", "8"].iter().enumerate() {
            self.draw_text(cr, (-0.25, 7.5 - rank as f64), glyph);
            self.draw_text(cr, (8.25, 7.5 - rank as f64), glyph);
        }

        for (file, glyph) in ["a", "b", "c", "d", "e", "f", "g", "h"].iter().enumerate() {
            self.draw_text(cr, (0.5 + file as f64, -0.25), glyph);
            self.draw_text(cr, (0.5 + file as f64, 8.25), glyph);
        }
    }

    fn draw_text(&self, cr: &Context, (x, y): (f64, f64), text: &str) {
        let font = cr.font_extents();
        let e = cr.text_extents(text);

        cr.save();
        cr.translate(x, y);
        cr.rotate(self.orientation.fold(0.0, PI));
        cr.move_to(-0.5 * e.width, 0.5 * font.ascent);
        cr.show_text(text);
        cr.restore();
    }

    fn draw_board(&self, cr: &Context) {
        cr.rectangle(0.0, 0.0, 8.0, 8.0);
        cr.set_source_rgb(0.55, 0.64, 0.68); // dark
        cr.fill();

        cr.set_source_rgb(0.87, 0.89, 0.90); // light

        for square in Bitboard::all() {
            if square.is_light() {
                cr.rectangle(square.file() as f64, 7.0 - square.rank() as f64, 1.0, 1.0);
                cr.fill();
            }
        }
    }

    fn draw_last_move(&self, cr: &Context) {
        if let Some((orig, dest)) = self.last_move {
            cr.set_source_rgba(0.61, 0.78, 0.0, 0.41);
            cr.rectangle(orig.file() as f64, 7.0 - orig.rank() as f64, 1.0, 1.0);
            cr.fill();

            if dest != orig {
                cr.rectangle(dest.file() as f64, 7.0 - dest.rank() as f64, 1.0, 1.0);
                cr.fill();
            }
        }
    }

    fn draw_check(&self, cr: &Context) {
        if let Some(check) = self.check {
            let cx = 0.5 + check.file() as f64;
            let cy = 7.5 - check.rank() as f64;
            let gradient = RadialGradient::new(cx, cy, 0.0, cx, cy, 0.5f64.hypot(0.5));
            gradient.add_color_stop_rgba(0.0, 1.0, 0.0, 0.0, 1.0);
            gradient.add_color_stop_rgba(0.25, 0.91, 0.0, 0.0, 1.0);
            gradient.add_color_stop_rgba(0.89, 0.66, 0.0, 0.0, 0.0);
            cr.set_source(&gradient);
            cr.paint();
        }
    }
}
