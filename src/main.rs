extern crate gtk;
extern crate gdk;
extern crate cairo;
extern crate rsvg;
extern crate shakmaty;

use std::cmp::min;
use std::rc::Rc;
use std::cell::RefCell;
use std::f64::consts::PI;

use shakmaty::square;
use shakmaty::{Square, Color};

use gtk::prelude::*;
use gtk::{Window, WindowType, DrawingArea};
use gdk::{EventMask, EventButton};
use cairo::{Context, Matrix, MatrixTrait};

enum DrawBrush {
    Green,
    Red,
    Blue,
    Yellow,
}

struct DrawShape {
    orig: Square,
    dest: Square,
    brush: DrawBrush,
    stroke: f64,
    opacity: f64,
}

struct Drawable {
    drawing: Option<DrawShape>,
    shapes: Vec<DrawShape>,
    enabled: bool,
    erase_on_click: bool,
}

impl Drawable {
    fn new() -> Drawable {
        Drawable {
            drawing: None,
            shapes: Vec::new(),
            enabled: true,
            erase_on_click: true,
        }
    }

    fn mouse_down(&mut self, widget: &DrawingArea, e: &EventButton) -> Option<Inhibit> {
        if e.get_button() == 3 {
            pos_to_square(widget, e.get_position()).map(|sq| {
                let brush = if e.get_state().contains(gdk::MOD1_MASK | gdk::SHIFT_MASK) {
                    DrawBrush::Yellow
                } else if e.get_state().contains(gdk::MOD1_MASK) {
                    DrawBrush::Blue
                } else if e.get_state().contains(gdk::SHIFT_MASK) {
                    DrawBrush::Red
                } else {
                    DrawBrush::Green
                };

                widget.queue_draw();
            });
        } else if e.get_button() == 1 {
            widget.queue_draw();
        }
        None
    }
}

struct BoardState {
    orientation: Color,
    selected: Option<Square>,
    drawable: Drawable,
}

impl BoardState {
    fn test() -> Self {
        BoardState {
            orientation: Color::White,
            selected: Some(square::E2),
            drawable: Drawable::new(),
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
                    state.drawable.mouse_down(widget, e);

                }
                Inhibit(false)
            });
        }

        {
            let state = Rc::downgrade(&v.state);
            v.widget.connect_button_release_event(move |widget, e| {
                if let Some(state) = state.upgrade() {
                    let mut state = state.borrow_mut();

                    let shape = state.drawable.drawing.take();
                    state.drawable.shapes.extend(shape);

                    widget.queue_draw();
                }
                Inhibit(false)
            });
        }

        {
            let state = Rc::downgrade(&v.state);
            v.widget.connect_motion_notify_event(move |widget, e| {
                if let Some(state) = state.upgrade() {
                    let mut state = state.borrow_mut();
                    if let Some(ref mut drawing) = state.drawable.drawing {
                        drawing.dest = pos_to_square(widget, e.get_position()).unwrap_or(drawing.orig);
                        widget.queue_draw();
                    }
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

fn draw_shape(cr: &Context, shape: &DrawShape) {
    cr.set_line_width(shape.stroke);

    match shape.brush {
        DrawBrush::Green => cr.set_source_rgba(0.08, 0.47, 0.11, shape.opacity),
        DrawBrush::Red => cr.set_source_rgba(0.53, 0.13, 0.13, shape.opacity),
        DrawBrush::Blue => cr.set_source_rgba(0.0, 0.19, 0.53, shape.opacity),
        DrawBrush::Yellow => cr.set_source_rgba(0.90, 0.94, 0.0, shape.opacity),
    }

    let xtail = 0.5 + shape.orig.file() as f64;
    let xhead = 0.5 + shape.dest.file() as f64;
    let ytail = 7.5 - shape.orig.rank() as f64;
    let yhead = 7.5 - shape.dest.rank() as f64;

    if shape.orig == shape.dest {
        // draw circle
        cr.arc(xhead, yhead, 0.5 * (1.0 - shape.stroke), 0.0, 2.0 * PI);
        cr.stroke();
    } else {
        // draw arrow
        let adjacent = xhead - xtail;
        let opposite = yhead - ytail;
        let hypot = adjacent.hypot(opposite);
        let marker_size = 0.75;

        let xbase = xhead - adjacent * marker_size / hypot;
        let ybase = yhead - opposite * marker_size / hypot;

        // line
        cr.move_to(xtail, ytail);
        cr.line_to(xbase, ybase);
        cr.stroke();

        // arrow head
        cr.line_to(xbase - opposite * 0.5 * marker_size / hypot,
                   ybase + adjacent * 0.5 * marker_size / hypot);
        cr.line_to(xhead, yhead);
        cr.line_to(xbase + opposite * 0.5 * marker_size / hypot,
                   ybase - adjacent * 0.5 * marker_size / hypot);
        cr.line_to(xbase, ybase);
        cr.fill();
    }

}

fn draw(widget: &DrawingArea, cr: &Context, state: &BoardState) {
    cr.set_matrix(compute_matrix(widget));

    draw_border(cr);
    draw_board(cr, &state);

    state.drawable.drawing.as_ref().map(|d| draw_shape(cr, d));

    for shape in &state.drawable.shapes {
        draw_shape(cr, shape);
    }

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
