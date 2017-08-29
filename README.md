rust-chessground
================

[![crates.io](https://img.shields.io/crates/v/chessground.svg)](https://crates.io/crates/chessground)

A chessboard widget for [Relm/GTK](https://github.com/antoyo/relm).
Inspired by [chessground.js](https://github.com/ornicar/chessground).

![](https://github.com/niklasf/rust-chessground/blob/master/screenshot.png?raw=true)

Features
--------

* Uses vocabulary from [Shakmaty](https://github.com/niklasf/shakmaty) but is
  chess rule agnostic
* Can show legal move hints
* Check hints
* Move pieces by click
* Move pieces by drag and drop
  - Minimum distance
  - Piece ghosts
* Draw circles and arrows on the board
* Integrated promotion dialog
* Smooth animations

Only a minimum of the features is exposed in the public API. Feel free to
request more.

Documentation
-------------

[Read the documentation](https://docs.rs/chessground)

Example
-------

A board that lets the user freely move pieces. Run with `cargo run --example editor`.

```rust
#![feature(proc_macro)]

extern crate gtk;
extern crate chessground;
#[macro_use]
extern crate relm;
extern crate relm_attributes;
#[macro_use]
extern crate relm_derive;
extern crate shakmaty;

use gtk::prelude::*;
use relm::Widget;
use relm_attributes::widget;

use shakmaty::{Square, Board};
use chessground::{Ground, UserMove, SetBoard};

use self::Msg::*;

#[derive(Msg)]
pub enum Msg {
    Quit,
    PieceMoved(Square, Square),
}

#[widget]
impl Widget for Win {
    fn model() -> Board {
        Board::default()
    }

    fn update(&mut self, event: Msg) {
        match event {
            Quit => gtk::main_quit(),
            PieceMoved(orig, dest) => {
                if let Some(piece) = self.model.remove_piece_at(orig) {
                    self.model.set_piece_at(dest, piece, false);
                    self.ground.emit(SetBoard(self.model.clone()));
                }
            }
        }
    }

    view! {
        gtk::Window {
            title: "Chessground",
            #[name="ground"]
            Ground {
                UserMove(orig, dest, _) => PieceMoved(orig, dest),
            },
            delete_event(_, _) => (Quit, Inhibit(false)),
        }
    }
}

fn main() {
    Win::run(()).expect("initialized gtk");
}
```

Piece sets
----------

Set | Author | License
--- | --- | ---
Merida | Armando Hernandez Marroquin | [GPL-2+](https://www.gnu.org/licenses/gpl-2.0.txt)

License
-------

Chessground is licensed under the GPL-3.0 (or any later version at your
option). See the COPYING file for the full license.
