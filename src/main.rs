extern crate gtk;
extern crate gdk;
extern crate cairo;
extern crate rsvg;
extern crate shakmaty;
extern crate option_filter;

use std::rc::Rc;
use std::cell::RefCell;
use std::f64::consts::PI;

use shakmaty::{Square, Color, Piece, Board, Bitboard, MoveList, Position, Chess, Setup};
use shakmaty::fen::Fen;

use gtk::prelude::*;
use gtk::{Window, WindowType, DrawingArea};
use gdk::{EventButton, EventMotion};
use cairo::prelude::*;
use cairo::{Context, Matrix, RadialGradient};
use rsvg::HandleExt;

use option_filter::OptionFilterExt;

mod drawable;
mod util;
mod pieceset;

use drawable::Drawable;
use pieceset::PieceSet;

struct BoardState {
    pieces: Board,
    orientation: Color,
    check: Option<Square>,
    selected: Option<Square>,
    drawable: Drawable,
    piece_set: PieceSet,
    legals: MoveList,
    drag: Option<Drag>,
    pos: Chess,
}

impl BoardState {
    fn user_move(&mut self, orig: Square, dest: Square) {
        let m = { self.legals.drain(..).filter(|m| m.from() == Some(orig) && m.to() == dest).next() };
        if let Some(m) = m {
            self.pos = self.pos.clone().play_unchecked(&m);
            self.pieces = self.pos.board().clone();
        }

        self.legals.clear();
        self.pos.legal_moves(&mut self.legals);
        self.check = self.pos.board().king_of(self.pos.turn()).filter(|_| self.pos.checkers().any());
    }
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
        dx.hypot(dy) > 3.0
    }
}

impl BoardState {
    fn test() -> Self {
        let fen: Fen = "2r2rk1/1p3ppp/p1nbb3/q3p3/2PpP3/1P3NP1/PB2QPBP/R3R1K1 w - - 4 16".parse().expect("valid fen");
        let pos: Chess = fen.position().expect("legal position");

        let mut state = BoardState {
            pieces: pos.board().clone(),
            orientation: Color::White,
            check: None,
            selected: None,
            drawable: Drawable::new(),
            piece_set: pieceset::PieceSet::merida(),
            legals: MoveList::new(),
            drag: None,
            pos: pos.clone(),
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
                             gdk::POINTER_MOTION_MASK).bits() as i32);

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
                    drag_mouse_down(&mut state, widget, square, e);
                    state.drawable.mouse_down(widget, square, e);
                }
                Inhibit(false)
            });
        }

        {
            let state = Rc::downgrade(&v.state);
            v.widget.connect_button_release_event(move |widget, e| {
                if let Some(state) = state.upgrade() {
                    let mut state = state.borrow_mut();
                    let square = util::pos_to_square(widget, state.orientation, e.get_position());

                    drag_mouse_up(&mut state, widget, square, e);
                    state.drawable.mouse_up(widget, square);
                }
                Inhibit(false)
            });
        }

        {
            let state = Rc::downgrade(&v.state);
            v.widget.connect_motion_notify_event(move |widget, e| {
                if let Some(state) = state.upgrade() {
                    let mut state = state.borrow_mut();
                    let square = util::pos_to_square(widget, state.orientation, e.get_position());

                    drag_mouse_move(&mut state, widget, square, e);
                    state.drawable.mouse_move(widget, square);
                }
                Inhibit(false)
            });
        }

        v
    }
}

fn selection_mouse_down(state: &mut BoardState, widget: &DrawingArea, e: &EventButton) {
    if e.get_button() == 1 {
        let orig = state.selected.take();
        let dest = util::pos_to_square(widget, state.orientation, e.get_position());

        if let (Some(orig), Some(dest)) =
            (orig, dest.filter(|sq| orig.map_or(false, |o| move_targets(state, o).contains(*sq))))
        {
            state.user_move(orig, dest);
        } else {
            state.selected = dest.filter(|sq| state.pieces.occupied().contains(*sq));
        }
    } else {
        state.selected = None;
    }

    widget.queue_draw();
}

fn drag_mouse_down(state: &mut BoardState, widget: &DrawingArea, square: Option<Square>, e: &EventButton) {
    if e.get_button() == 1 {
        if let Some(square) = square {
            state.drag = state.pieces.piece_at(square).map(|piece| Drag {
                piece,
                orig: square,
                dest: square,
                start: e.get_position(),
                pos: e.get_position(),
            });

            widget.queue_draw();
        }
    }
}

fn drag_mouse_move(state: &mut BoardState, widget: &DrawingArea, square: Option<Square>, e: &EventMotion) {
    if let Some(ref mut drag) = state.drag {
        let matrix = util::compute_matrix(widget, state.orientation);
        let (dx, dy) = matrix.transform_distance(0.5, 0.5);
        let (dx, dy) = (dx.ceil(), dy.ceil());


        widget.queue_draw_area((drag.pos.0 - dx) as i32, (drag.pos.1 - dy) as i32,
                               2 * (dx as i32), 2 * (dy as i32));

        drag.pos = e.get_position();
        drag.dest = square.unwrap_or(drag.orig);

        widget.queue_draw_area((drag.pos.0 - dx) as i32, (drag.pos.1 - dy) as i32,
                               2 * (dx as i32), 2 * (dy as i32));
    }
}

fn drag_mouse_up(state: &mut BoardState, widget: &DrawingArea, square: Option<Square>, e: &EventButton) {
    if let Some(mut drag) = state.drag.take() {
        drag.dest = square.unwrap_or(drag.orig);
        drag.pos = e.get_position();
        println!("drag: {} to {}", drag.orig, drag.dest);
        state.user_move(drag.orig, drag.dest);
        widget.queue_draw();
    }
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

    cr.rectangle(0.0, 0.0, 8.0, 8.0);
    cr.set_source(&dark);
    cr.fill();

    cr.set_source(&light);

    for square in Bitboard::all() {
        if square.is_light() {
            cr.rectangle(square.file() as f64, 7.0 - square.rank() as f64, 1.0, 1.0);
            cr.fill();
        }
    }

    if let Some(square) = state.selected {
        cr.rectangle(square.file() as f64, 7.0 - square.rank() as f64, 1.0, 1.0);
        cr.set_source_rgba(0.08, 0.47, 0.11, 0.5);
        cr.fill();
    }

    let hovered = state.drag.as_ref()
        .filter(|d| d.threshold() && move_targets(state, d.orig).contains(d.dest))
        .map(|d| d.dest);

    if let Some(square) = hovered {
        if hovered != state.selected {
            cr.rectangle(square.file() as f64, 7.0 - square.rank() as f64, 1.0, 1.0);
            cr.set_source_rgba(0.08, 0.47, 0.11, 0.25);
            cr.fill();
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

fn draw_check(cr: &Context, state: &BoardState) {
    if let Some(check) = state.check {
        let cx = 0.5 + check.file() as f64;
        let cy = 7.5 - check.rank() as f64;
        let gradient = RadialGradient::new(cx, cy, 0.0, cx, cy, 0.5f64.hypot(0.5));
        gradient.add_color_stop_rgba(0.0, 1.0, 0.0, 0.0, 1.0);
        gradient.add_color_stop_rgba(0.25, 0.91, 0.0, 0.0, 1.0);
        gradient.add_color_stop_rgba(0.89, 0.66, 0.0, 0.0, 0.0);
        cr.set_source(&gradient);
        cr.paint();
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
    draw_check(cr, &state);
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
