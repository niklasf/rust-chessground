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

use shakmaty::{Square, File, Rank};

pub fn ease(start: f64, end: f64, t: f64) -> f64 {
    // ease in out cubic from https://gist.github.com/gre/1650294
    let t = t.max(0.0).min(1.0);
    let ease = if t < 0.5 {
        4.0 * t * t * t
    } else {
        (t - 1.0) * (2.0 * t - 2.0) * (2.0 * t - 2.0) + 1.0
    };
    start + (end - start) * ease
}

pub fn pos_to_square((x, y): (f64, f64)) -> Option<Square> {
    let (x, y) = (x.floor(), y.floor());
    if 0f64 <= x && x <= 7f64 && 0f64 <= y && y <= 7f64 {
        Some(Square::from_coords(File::new(x as u32), Rank::new(7 - y as u32)))
    } else {
        None
    }
}

pub fn square_to_pos(square: Square) -> (f64, f64) {
    (0.5 + file_to_float(square.file()), 7.5 - rank_to_float(square.rank()))
}

pub fn rank_to_float(rank: Rank) -> f64 {
    f64::from(i8::from(rank))
}

pub fn file_to_float(file: File) -> f64 {
    f64::from(i8::from(file))
}
