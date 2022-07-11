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

use rsvg::Handle;

use shakmaty::{Color, Role, Piece};

struct PieceSetSide {
    pawn: Handle,
    knight: Handle,
    bishop: Handle,
    rook: Handle,
    queen: Handle,
    king: Handle,
}

impl PieceSetSide {
    fn by_role(&self, role: Role) -> &Handle {
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
        color.fold_wb(&self.white, &self.black)
    }

    pub fn by_piece(&self, piece: &Piece) -> &Handle {
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
                pawn: Handle::from_data(include_bytes!("merida/bP.svg")).expect("merida/bP.svg"),
                knight: Handle::from_data(include_bytes!("merida/bN.svg")).expect("merida/bN.svg"),
                bishop: Handle::from_data(include_bytes!("merida/bB.svg")).expect("merida/bB.svg"),
                rook: Handle::from_data(include_bytes!("merida/bR.svg")).expect("merida/bR.svg"),
                queen: Handle::from_data(include_bytes!("merida/bQ.svg")).expect("merida/bQ.svg"),
                king: Handle::from_data(include_bytes!("merida/bK.svg")).expect("merida/bK.svg"),
            },
            white: PieceSetSide {
                pawn: Handle::from_data(include_bytes!("merida/wP.svg")).expect("merida/wP.svg"),
                knight: Handle::from_data(include_bytes!("merida/wN.svg")).expect("merida/wN.svg"),
                bishop: Handle::from_data(include_bytes!("merida/wB.svg")).expect("merida/wB.svg"),
                rook: Handle::from_data(include_bytes!("merida/wR.svg")).expect("merida/wR.svg"),
                queen: Handle::from_data(include_bytes!("merida/wQ.svg")).expect("merida/wQ.svg"),
                king: Handle::from_data(include_bytes!("merida/wK.svg")).expect("merida/wK.svg"),
            },
        }
    }
}
