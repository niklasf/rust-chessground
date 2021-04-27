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

use librsvg::{Loader, SvgHandle};

use shakmaty::{Color, Role, Piece};

struct PieceSetSide {
    pawn: SvgHandle,
    knight: SvgHandle,
    bishop: SvgHandle,
    rook: SvgHandle,
    queen: SvgHandle,
    king: SvgHandle,
}

impl PieceSetSide {
    fn by_role(&self, role: Role) -> &SvgHandle {
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

    pub fn by_piece(&self, piece: &Piece) -> &SvgHandle {
        self.by_color(piece.color).by_role(piece.role)
    }

    pub fn scale(&self) -> f64 {
        1.0 / 177.0
    }
}

impl PieceSet {
    pub fn merida() -> PieceSet {
        PieceSet {
            black: PieceSetSide {
                pawn: Loader::new().read_path("src/merida/bP.svg").expect("merida/bP.svg"),
                knight: Loader::new().read_path("src/merida/bN.svg").expect("merida/bN.svg"),
                bishop: Loader::new().read_path("src/merida/bB.svg").expect("merida/bB.svg"),
                rook: Loader::new().read_path("src/merida/bR.svg").expect("merida/bR.svg"),
                queen: Loader::new().read_path("src/merida/bQ.svg").expect("merida/bQ.svg"),
                king: Loader::new().read_path("src/merida/bK.svg").expect("merida/bK.svg"),
            },
            white: PieceSetSide {
                pawn: Loader::new().read_path("src/merida/wP.svg").expect("merida/wP.svg"),
                knight: Loader::new().read_path("src/merida/wN.svg").expect("merida/wN.svg"),
                bishop: Loader::new().read_path("src/merida/wB.svg").expect("merida/wB.svg"),
                rook: Loader::new().read_path("src/merida/wR.svg").expect("merida/wR.svg"),
                queen: Loader::new().read_path("src/merida/wQ.svg").expect("merida/wQ.svg"),
                king: Loader::new().read_path("src/merida/wK.svg").expect("merida/wK.svg"),
            },
        }
    }
}
