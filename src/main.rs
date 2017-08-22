extern crate gtk;
extern crate gdk;
extern crate cairo;
extern crate rsvg;
extern crate shakmaty;

use std::cmp::min;
use std::rc::Rc;
use std::cell::RefCell;

use shakmaty::square;
use shakmaty::{Square, Color};

use gtk::prelude::*;
use gtk::{Window, WindowType, DrawingArea};
use gdk::{EventMask, EventButton};
use cairo::{Context, Matrix, MatrixTrait};

struct Drawing {
    orig: Square,
    dest: Square,
}

struct BoardState {
    orientation: Color,
    selected: Option<Square>,
    drawing: Option<Drawing>,
}

impl BoardState {
    fn test() -> Self {
        BoardState {
            orientation: Color::White,
            selected: Some(square::E2),
            drawing: None,
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

        v.widget.add_events(gdk::BUTTON_PRESS_MASK.bits() as i32);

        {
            let state = Rc::downgrade(&v.state);
            v.widget.connect_draw(move |widget, cr| {
                if let Some(state) = state.upgrade() {
                    draw(widget, cr, &*state.borrow());
                } else {
                    println!("failed to draw");
                }
                Inhibit(false)
            });
        }

        {
            let state = Rc::downgrade(&v.state);
            v.widget.connect_button_press_event(move |widget, e| {
                if let Some(state) = state.upgrade() {
                    let mut state = state.borrow_mut();
                    println!("press: {:?} {:?}", e.get_position(), e.get_button());
                    pos_to_square(widget, e.get_position()).map(|sq| {
                        println!("{}", sq);
                        state.drawing = Some(Drawing { orig: sq, dest: sq });
                    });
                }
                Inhibit(false)
            });
        }

        v
    }
}

fn compute_matrix(widget: &DrawingArea) -> Matrix {
    let mut matrix = Matrix::identity();

    let w = widget.get_allocated_width();
    let h = widget.get_allocated_height();
    let size = min(w, h);

    matrix.translate(w as f64 / 2.0, h as f64 / 2.0);
    matrix.scale(size as f64 / 10.0, size as f64 / 10.0);
    matrix.translate(-4.0, -4.0);

    matrix
}

fn pos_to_square(widget: &DrawingArea, (x, y): (f64, f64)) -> Option<Square> {
    let mut matrix = compute_matrix(widget);
    matrix.invert();
    let (x, y) = matrix.transform_point(x, y);
    let (x, y) = (x.floor(), y.floor());
    if 0f64 <= x && x <= 7f64 && 0f64 <= y && y <= 7f64 {
        Square::from_coords(x as i8, 7 - y as i8)
    } else {
        None
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

fn draw_drawing(cr: &Context, drawing: &Drawing) {
    cr.set_line_width(0.2);
    cr.set_source_rgb(0f64, 0f64, 0f64);
    cr.move_to(0.5 + drawing.orig.file() as f64, 7.5 - drawing.orig.rank() as f64);
    cr.line_to(0.5 + drawing.dest.file() as f64, 7.5 - drawing.dest.rank() as f64);
    cr.stroke();
}

fn draw(widget: &DrawingArea, cr: &Context, state: &BoardState) {
    cr.set_matrix(compute_matrix(widget));

    draw_border(cr);
    draw_board(cr, &state);
    state.drawing.as_ref().map(|d| draw_drawing(cr, d));

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
