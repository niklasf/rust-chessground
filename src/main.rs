extern crate gtk;
extern crate gdk;
extern crate cairo;
extern crate rsvg;
extern crate shakmaty;
extern crate option_filter;

use std::rc::Rc;
use std::cell::RefCell;

use shakmaty::square;
use shakmaty::{Square, Color, Board};

use gtk::prelude::*;
use gtk::{Window, WindowType, DrawingArea};
use gdk::EventButton;
use cairo::Context;

use option_filter::OptionFilterExt;

mod drawable;
mod util;
mod pieceset;

use drawable::Drawable;
use pieceset::PieceSet;

struct BoardState {
    orientation: Color,
    selected: Option<Square>,
    drawable: Drawable,
    piece_set: PieceSet,
    pieces: Board,
}

impl BoardState {
    fn test() -> Self {
        BoardState {
            orientation: Color::White,
            selected: None,
            drawable: Drawable::new(),
            piece_set: pieceset::PieceSet::merida(),
            pieces: Board::new(),
        }
    }
}

struct BoardView {
    widget: DrawingArea,
    state: Rc<RefCell<BoardState>>,
}

impl BoardView {
    fn new() -> Self {
        let v = BoardView {
            widget: DrawingArea::new(),
            state: Rc::new(RefCell::new(BoardState::test())),
        };

        v.widget.add_events((gdk::BUTTON_PRESS_MASK |
                             gdk::BUTTON_RELEASE_MASK |
                             gdk::BUTTON_MOTION_MASK).bits() as i32);

        {
            let state = Rc::downgrade(&v.state);
            v.widget.connect_draw(move |widget, cr| {
                if let Some(state) = state.upgrade() {
                    draw(widget, cr, &*state.borrow());
                }
                Inhibit(false)
            });
        }

        {
            let state = Rc::downgrade(&v.state);
            v.widget.connect_button_press_event(move |widget, e| {
                if let Some(state) = state.upgrade() {
                    let mut state = state.borrow_mut();
                    selection_mouse_down(&mut state, widget, e);
                    state.drawable.mouse_down(widget, e).unwrap_or(Inhibit(false))
                } else {
                    Inhibit(false)
                }
            });
        }

        {
            let state = Rc::downgrade(&v.state);
            v.widget.connect_button_release_event(move |widget, e| {
                if let Some(state) = state.upgrade() {
                    let mut state = state.borrow_mut();
                    state.drawable.mouse_up(widget, e).unwrap_or(Inhibit(false))
                } else {
                    Inhibit(false)
                }
            });
        }

        {
            let state = Rc::downgrade(&v.state);
            v.widget.connect_motion_notify_event(move |widget, e| {
                if let Some(state) = state.upgrade() {
                    let mut state = state.borrow_mut();
                    state.drawable.mouse_move(widget, e).unwrap_or(Inhibit(false))
                } else {
                    Inhibit(false)
                }
            });
        }

        v
    }
}

fn selection_mouse_down(state: &mut BoardState, widget: &DrawingArea, e: &EventButton) -> Option<Inhibit> {
    if e.get_button() == 1 {
        state.selected =
            util::pos_to_square(widget, e.get_position())
                .filter(|sq| state.pieces.occupied().contains(*sq));
    } else {
        state.selected = None;
    }
    None
}

fn draw_border(cr: &Context) {
    let border = cairo::SolidPattern::from_rgb(0.2, 0.2, 0.5);
    cr.set_source(&border);
    cr.rectangle(-0.5, -0.5, 9.0, 9.0);
    cr.fill();
}

fn draw_board(cr: &Context, state: &BoardState) {
    let light = cairo::SolidPattern::from_rgb(0.87, 0.89, 0.90);
    let dark = cairo::SolidPattern::from_rgb(0.55, 0.64, 0.68);
    let selected = cairo::SolidPattern::from_rgb(0.5, 1.0, 0.5);

    for x in 0..8 {
        for y in 0..8 {
            if state.selected.map_or(false, |sq| sq.file() == x && sq.rank() == 7 - y) {
                cr.set_source(&selected);
            } else if (x + y) % 2 == 0 {
                cr.set_source(&light);
            } else {
                cr.set_source(&dark);
            }

            cr.rectangle(x as f64, y as f64, 1.0, 1.0);
            cr.fill();
        }
    }
}

fn draw_pieces(cr: &Context, state: &BoardState) {
    for square in state.pieces.occupied() {
        cr.save();
        cr.translate(square.file() as f64, 7.0 - square.rank() as f64);
        cr.scale(0.0056, 0.0056);

        let piece = state.pieces.piece_at(square).expect("enumerating");
        state.piece_set.by_piece(&piece).render_cairo(cr);

        cr.restore();
    }
}

fn draw(widget: &DrawingArea, cr: &Context, state: &BoardState) {
    cr.set_matrix(util::compute_matrix(widget));

    draw_border(cr);
    draw_board(cr, &state);
    draw_pieces(cr, &state);

    state.drawable.render_cairo(cr);

    //ctx.rectangle(0.0, 0.0, 50.0, 50.0);
    //ctx.fill();
    //img.render_cairo(ctx);

}

fn main() {
    gtk::init().expect("initialized gtk");

    let window = Window::new(WindowType::Toplevel);
    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    let board = BoardView::new();
    window.add(&board.widget);
    window.show_all();

    gtk::main();
}
