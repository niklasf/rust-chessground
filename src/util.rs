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

use shakmaty::Square;

pub fn fmin(a: f64, b: f64) -> f64 {
    if a < b { a } else { b }
}

pub fn fmax(a: f64, b: f64) -> f64 {
    if a > b { a } else { b }
}

pub fn ease(start: f64, end: f64, t: f64) -> f64 {
    // ease in out cubic from https://gist.github.com/gre/1650294
    let t = fmax(0.0, fmin(1.0, t));
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
        Square::from_coords(x as i8, 7 - y as i8)
    } else {
        None
    }
}

pub fn square_to_pos(square: Square) -> (f64, f64) {
    (0.5 + f64::from(square.file()), 7.5 - f64::from(square.rank()))
}
