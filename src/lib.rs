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

//! A chessboard widget for Relm/GTK.

#![doc(html_root_url = "https://docs.rs/chessground/0.2.0")]

#![warn(missing_debug_implementations)]

extern crate gtk;
extern crate gdk;
extern crate cairo;
extern crate rsvg;
extern crate shakmaty;
extern crate option_filter;
extern crate time;
extern crate relm;
#[macro_use]
extern crate relm_derive;

mod ground;
mod boardstate;
mod pieceset;
mod pieces;
mod promotable;
mod drawable;
mod util;

pub use ground::{Ground, GroundMsg, Pos};
pub use GroundMsg::*;
pub use drawable::{DrawBrush, DrawShape};
