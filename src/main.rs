extern crate gtk;
extern crate gdk;
extern crate cairo;
extern crate rsvg;
extern crate shakmaty;
extern crate option_filter;

use std::rc::Rc;
use std::cell::RefCell;
use std::f64::consts::PI;

use shakmaty::{Square, Color, Piece, Board, Bitboard, Move, MoveList, Position, Chess, Setup};
use shakmaty::fen::Fen;

use gtk::prelude::*;
use gtk::{Window, WindowType, DrawingArea};
use gdk::{EventButton, EventMotion};
use cairo::prelude::*;
use cairo::{Context, Matrix};
use rsvg::HandleExt;

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
    legals: MoveList,
    drag: Option<Drag>,
}

struct Drag {
    piece: Piece,
    orig: Square,
    dest: Square,
    start: (f64, f64),
    pos: (f64, f64),
}

impl Drag {
    fn threshold(&self) -> bool {
        let dx = self.start.0 - self.pos.0;
        let dy = self.start.1 - self.pos.1;
        dx.hypot(dy) > 10.0
    }
}

impl BoardState {
    fn test() -> Self {
        let fen: Fen = "2r2rk1/1p3ppp/p1nbb3/q3p3/2PpP3/1P3NP1/PB2QPBP/R3R1K1 w - - 4 16".parse().expect("valid fen");
        let pos: Chess = fen.position().expect("legal position");

        let mut state = BoardState {
            orientation: Color::White,
            selected: None,
            drawable: Drawable::new(),
            piece_set: pieceset::PieceSet::merida(),
            pieces: pos.board().clone(),
            legals: MoveList::new(),
            drag: None,
        };

        pos.legal_moves(&mut state.legals);

        state
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
                    let square = util::pos_to_square(widget, state.orientation, e.get_position());
                    selection_mouse_down(&mut state, widget, e);
                    drag_mouse_down(&mut state, square, e);
                    state.drawable.mouse_down(widget, square, e).unwrap_or(Inhibit(false))
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
                    let square = util::pos_to_square(widget, state.orientation, e.get_position());
                    drag_mouse_up(&mut state, widget, square, e);
                    state.drawable.mouse_up(widget, square).unwrap_or(Inhibit(false))
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
                    let square = util::pos_to_square(widget, state.orientation, e.get_position());
                    drag_mouse_move(&mut state, widget, square, e);
                    state.drawable.mouse_move(widget, square).unwrap_or(Inhibit(false))
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
            util::pos_to_square(widget, state.orientation, e.get_position())
                .filter(|sq| state.pieces.occupied().contains(*sq));
    } else {
        state.selected = None;
    }
    None
}

fn drag_mouse_down(state: &mut BoardState, square: Option<Square>, e: &EventButton) -> Option<Inhibit> {
    if let Some(square) = square {
        state.drag = state.pieces.piece_at(square).map(|piece| Drag {
            piece,
            orig: square,
            dest: square,
            start: e.get_position(),
            pos: e.get_position(),
        });
    }
    None
}

fn drag_mouse_move(state: &mut BoardState, widget: &DrawingArea, square: Option<Square>, e: &EventMotion) -> Option<Inhibit> {
    if let Some(ref mut drag) = state.drag {
        drag.dest = square.unwrap_or(drag.orig);
        drag.pos = e.get_position();
        widget.queue_draw();
    }
    None
}

fn drag_mouse_up(state: &mut BoardState, widget: &DrawingArea, square: Option<Square>, e: &EventButton) -> Option<Inhibit> {
    if let Some(mut drag) = state.drag.take() {
        drag.dest = square.unwrap_or(drag.orig);
        drag.pos = e.get_position();
        println!("drag: {} to {}", drag.orig, drag.dest);
        widget.queue_draw();
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

    let hovered = state.drag.as_ref()
        .filter(|d| d.threshold() && move_targets(state, d.orig).contains(d.dest))
        .map(|d| d.dest);

    for square in Bitboard::all() {
        cr.set_source(if square.is_light() { &light } else { &dark });
        cr.rectangle(square.file() as f64, 7.0 - square.rank() as f64, 1.0, 1.0);
        cr.fill_preserve();

       if state.selected.map_or(false, |sq| sq == square) {
           cr.set_source_rgba(0.08, 0.47, 0.11, 0.5);
           cr.fill();
        } else if Some(square) == hovered {
           cr.set_source_rgba(0.08, 0.47, 0.11, 0.25);
           cr.fill();
        } else {
            cr.new_path();
        }
    }
}

fn draw_pieces(cr: &Context, state: &BoardState) {
    for square in state.pieces.occupied() {
        cr.push_group();
        cr.translate(square.file() as f64, 7.0 - square.rank() as f64);

        cr.translate(0.5, 0.5);
        cr.rotate(state.orientation.fold(0.0, PI));
        cr.translate(-0.5, -0.5);

        cr.scale(0.0056, 0.0056);

        let piece = state.pieces.piece_at(square).expect("enumerating");
        state.piece_set.by_piece(&piece).render_cairo(cr);

        cr.pop_group_to_source();

        if state.drag.as_ref().map_or(false, |d| d.threshold() && d.orig == square) {
            cr.paint_with_alpha(0.2);
        } else {
            cr.paint();
        }
    }
}

fn move_targets(state: &BoardState, orig: Square) -> Bitboard {
    state.legals.iter().filter(|m| m.from() == Some(orig)).map(|m| m.to()).collect()
}

fn draw_move_hints(cr: &Context, state: &BoardState) {
    if let Some(selected) = state.selected {
        cr.set_source_rgba(0.08, 0.47, 0.11, 0.5);

        let radius = 0.12;
        let corner = 1.8 * radius;

        for square in move_targets(state, selected) {
            if state.pieces.occupied().contains(square) {
                cr.move_to(square.file() as f64, 7.0 - square.rank() as f64);
                cr.rel_line_to(corner, 0.0);
                cr.rel_line_to(-corner, corner);
                cr.rel_line_to(0.0, -corner);
                cr.fill();

                cr.move_to(1.0 + square.file() as f64, 7.0 - square.rank() as f64);
                cr.rel_line_to(0.0, corner);
                cr.rel_line_to(-corner, -corner);
                cr.rel_line_to(corner, 0.0);
                cr.fill();

                cr.move_to(square.file() as f64, 8.0 - square.rank() as f64);
                cr.rel_line_to(corner, 0.0);
                cr.rel_line_to(-corner, -corner);
                cr.rel_line_to(0.0, corner);
                cr.fill();

                cr.move_to(1.0 + square.file() as f64, 8.0 - square.rank() as f64);
                cr.rel_line_to(-corner, 0.0);
                cr.rel_line_to(corner, -corner);
                cr.rel_line_to(0.0, corner);
                cr.fill();
            } else {
                cr.arc(0.5 + square.file() as f64,
                       7.5 - square.rank() as f64,
                       radius, 0.0, 2.0 * PI);
                cr.fill();
            }
        }
    }
}

fn draw_drag(cr: &Context, mut matrix: Matrix, state: &BoardState) {
    if let Some(drag) = state.drag.as_ref().filter(|d| d.threshold()) {
        matrix.invert();
        let (x, y) = matrix.transform_point(drag.pos.0, drag.pos.1);
        cr.translate(x, y);
        cr.rotate(state.orientation.fold(0.0, PI));
        cr.translate(-0.5, -0.5);

        cr.scale(0.0056, 0.0056);

        state.piece_set.by_piece(&drag.piece).render_cairo(cr);
    }
}

fn draw(widget: &DrawingArea, cr: &Context, state: &BoardState) {
    let matrix = util::compute_matrix(widget, state.orientation);
    cr.set_matrix(matrix);

    draw_border(cr);
    draw_board(cr, &state);
    draw_pieces(cr, &state);

    state.drawable.render_cairo(cr);

    draw_move_hints(cr, &state);

    draw_drag(cr, matrix, state);

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
