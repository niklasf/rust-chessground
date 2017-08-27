extern crate gtk;
extern crate gdk;
extern crate cairo;
extern crate rsvg;
extern crate shakmaty;
extern crate option_filter;
extern crate time;
extern crate rand;
extern crate relm;
#[macro_use]
extern crate relm_derive;

mod drawable;
mod util;
mod pieceset;
mod ground;

pub use ground::{Ground, GroundMsg};
pub use GroundMsg::*;
