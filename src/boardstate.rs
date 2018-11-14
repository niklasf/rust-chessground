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

use cairo::prelude::*;
use cairo::{Context, Pattern, RadialGradient};

use shakmaty::{Color, Square, Role, Bitboard, Chess, Position, MoveList};

use pieceset::PieceSet;

pub struct BoardState {
    orientation: Color,
    check: Option<Square>,
    last_move: Option<(Square, Square)>,
    turn: Option<Color>,
    piece_set: PieceSet,
    legals: MoveList,
}

impl BoardState {
    pub fn new() -> Self {
        BoardState::from_position(&Chess::default())
    }

    pub fn from_position<P: Position>(pos: &P) -> Self {
        let mut state = BoardState {
            orientation: pos.turn(),
            check: None,
            last_move: None,
            turn: None,
            piece_set: PieceSet::merida(),
            legals: MoveList::new(),
        };

        state.set_position(pos);
        state
    }

    pub fn set_position<P: Position>(&mut self, pos: &P) {
        self.check = if pos.checkers().any() { pos.board().king_of(pos.turn()) } else { None };
        self.legals = pos.legals();
        self.turn = Some(pos.turn());
    }

    pub fn set_last_move(&mut self, m: Option<(Square, Square)>) {
        self.last_move = m;
    }

    pub fn set_check(&mut self, king: Option<Square>) {
        self.check = king;
    }

    pub fn set_turn(&mut self, turn: Option<Color>) {
        self.turn = turn;
    }

    pub fn turn(&self) -> Option<Color> {
        self.turn
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

    pub fn legals(&self) -> &MoveList {
        &self.legals
    }

    pub fn legals_mut(&mut self) -> &mut MoveList {
        &mut self.legals
    }

    pub fn set_orientation(&mut self, orientation: Color) {
        self.orientation = orientation;
    }

    pub fn orientation(&self) -> Color {
        self.orientation
    }

    pub fn piece_set(&self) -> &PieceSet {
        &self.piece_set
    }

    pub(crate) fn draw(&self, cr: &Context) {
        self.draw_border(cr);
        self.draw_turn(cr);
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

    fn draw_turn(&self, cr: &Context) {
        match self.turn {
            Some(Color::White) => {
                cr.set_source_rgb(1.0, 1.0, 1.0);
                cr.arc(8.25, 8.25, 0.1, 0.0, 2.0 * PI);
                cr.fill();
            },
            Some(Color::Black) => {
                cr.set_source_rgb(0.0, 0.0, 0.0);
                cr.arc(8.25, -0.25, 0.1, 0.0, 2.0 * PI);
                cr.fill();
            }
            None => (),
        }
    }

    fn draw_text(&self, cr: &Context, (x, y): (f64, f64), text: &str) {
        let font = cr.font_extents();
        let e = cr.text_extents(text);

        cr.save();
        cr.translate(x, y);
        cr.rotate(self.orientation.fold(0.0, PI));
        cr.move_to(-0.5 * e.width, 0.5 * font.height - font.descent);
        cr.show_text(text);
        cr.restore();
    }

    fn draw_board(&self, cr: &Context) {
        cr.rectangle(0.0, 0.0, 8.0, 8.0);
        cr.set_source_rgb(0.55, 0.64, 0.68); // dark
        cr.fill();

        cr.set_source_rgb(0.87, 0.89, 0.90); // light

        for square in Bitboard::ALL {
            if square.is_light() {
                cr.rectangle(f64::from(square.file()), 7.0 - f64::from(square.rank()), 1.0, 1.0);
                cr.fill();
            }
        }
    }

    fn draw_last_move(&self, cr: &Context) {
        if let Some((orig, dest)) = self.last_move {
            cr.set_source_rgba(0.61, 0.78, 0.0, 0.41);
            cr.rectangle(f64::from(orig.file()), 7.0 - f64::from(orig.rank()), 1.0, 1.0);
            cr.fill();

            if dest != orig {
                cr.rectangle(f64::from(dest.file()), 7.0 - f64::from(dest.rank()), 1.0, 1.0);
                cr.fill();
            }
        }
    }

    fn draw_check(&self, cr: &Context) {
        if let Some(check) = self.check {
            let cx = 0.5 + f64::from(check.file());
            let cy = 7.5 - f64::from(check.rank());
            let gradient = RadialGradient::new(cx, cy, 0.0, cx, cy, 0.5f64.hypot(0.5));
            gradient.add_color_stop_rgba(0.0, 1.0, 0.0, 0.0, 1.0);
            gradient.add_color_stop_rgba(0.25, 0.91, 0.0, 0.0, 1.0);
            gradient.add_color_stop_rgba(0.89, 0.66, 0.0, 0.0, 0.0);
            cr.set_source(&Pattern::RadialGradient(gradient));
            cr.paint();
        }
    }
}
