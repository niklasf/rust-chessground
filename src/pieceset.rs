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

use cairo;
use resvg;

use shakmaty::{Color, Role, Piece};

struct PieceSetSide {
    pawn: resvg::Document,
    knight: resvg::Document,
    bishop: resvg::Document,
    rook: resvg::Document,
    queen: resvg::Document,
    king: resvg::Document,
}

impl PieceSetSide {
    fn by_role(&self, role: Role) -> &resvg::Document {
        match role {
            Role::Pawn => &self.pawn,
            Role::Knight => &self.knight,
            Role::Bishop => &self.bishop,
            Role::Rook => &self.rook,
            Role::Queen => &self.queen,
            Role::King => &self.king,
        }
    }
}

pub struct PieceSet {
    black: PieceSetSide,
    white: PieceSetSide,
}

impl PieceSet {
    fn by_color(&self, color: Color) -> &PieceSetSide {
        color.fold(&self.white, &self.black)
    }

    fn by_piece(&self, piece: &Piece) -> &resvg::Document {
        self.by_color(piece.color).by_role(piece.role)
    }

    pub fn render_cairo(&self, cr: &cairo::Context, piece: &Piece) {
        cr.scale(1.0 / 177.0, 1.0 / 177.0);
        let rect = resvg::Rect::new(0.0, 0.0, 177.0, 177.0);
        resvg::render_cairo::render_to_canvas(cr, rect, self.by_piece(piece));
    }
}

impl PieceSet {
    pub fn merida() -> PieceSet {
        let opts = resvg::Options::default();

        PieceSet {
            black: PieceSetSide {
                pawn: resvg::parse_doc_from_data(include_str!("merida/bP.svg"), &opts).expect("merida/bP.svg"),
                knight: resvg::parse_doc_from_data(include_str!("merida/bN.svg"), &opts).expect("merida/bN.svg"),
                bishop: resvg::parse_doc_from_data(include_str!("merida/bB.svg"), &opts).expect("merida/bB.svg"),
                rook: resvg::parse_doc_from_data(include_str!("merida/bR.svg"), &opts).expect("merida/bR.svg"),
                queen: resvg::parse_doc_from_data(include_str!("merida/bQ.svg"), &opts).expect("merida/bQ.svg"),
                king: resvg::parse_doc_from_data(include_str!("merida/bK.svg"), &opts).expect("merida/bK.svg"),
            },
            white: PieceSetSide {
                pawn: resvg::parse_doc_from_data(include_str!("merida/wP.svg"), &opts).expect("merida/wP.svg"),
                knight: resvg::parse_doc_from_data(include_str!("merida/wN.svg"), &opts).expect("merida/wN.svg"),
                bishop: resvg::parse_doc_from_data(include_str!("merida/wB.svg"), &opts).expect("merida/wB.svg"),
                rook: resvg::parse_doc_from_data(include_str!("merida/wR.svg"), &opts).expect("merida/wR.svg"),
                queen: resvg::parse_doc_from_data(include_str!("merida/wQ.svg"), &opts).expect("merida/wQ.svg"),
                king: resvg::parse_doc_from_data(include_str!("merida/wK.svg"), &opts).expect("merida/wK.svg"),
            },
        }
    }
}
