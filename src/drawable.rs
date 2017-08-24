extern crate gtk;
extern crate gdk;
extern crate cairo;
extern crate rsvg;
extern crate shakmaty;

use std::f64::consts::PI;

use shakmaty::Square;

use gtk::prelude::*;
use gtk::DrawingArea;
use gdk::{EventButton, EventMotion};
use cairo::Context;

use util::pos_to_square;

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

impl DrawShape {
    fn render_cairo(&self, cr: &Context) {
        cr.set_line_width(self.stroke);

        match self.brush {
            DrawBrush::Green => cr.set_source_rgba(0.08, 0.47, 0.11, self.opacity),
            DrawBrush::Red => cr.set_source_rgba(0.53, 0.13, 0.13, self.opacity),
            DrawBrush::Blue => cr.set_source_rgba(0.0, 0.19, 0.53, self.opacity),
            DrawBrush::Yellow => cr.set_source_rgba(0.90, 0.94, 0.0, self.opacity),
        }

        let xtail = 0.5 + self.orig.file() as f64;
        let xhead = 0.5 + self.dest.file() as f64;
        let ytail = 7.5 - self.orig.rank() as f64;
        let yhead = 7.5 - self.dest.rank() as f64;

        if self.orig == self.dest {
            // draw circle
            cr.arc(xhead, yhead, 0.5 * (1.0 - self.stroke), 0.0, 2.0 * PI);
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
}

pub struct Drawable {
    drawing: Option<DrawShape>,
    shapes: Vec<DrawShape>,
    enabled: bool,
    erase_on_click: bool,
}

impl Drawable {
    pub fn new() -> Drawable {
        Drawable {
            drawing: None,
            shapes: Vec::new(),
            enabled: true,
            erase_on_click: true,
        }
    }

    pub(crate) fn mouse_down(&mut self, widget: &DrawingArea, e: &EventButton) -> Option<Inhibit> {
        if !self.enabled {
            return None;
        }

        match e.get_button() {
            1 => {
                if self.erase_on_click {
                    self.shapes.clear();
                    widget.queue_draw();
                }
            },
            3 => {
                self.drawing = pos_to_square(widget, e.get_position()).map(|square| {
                    let brush = if e.get_state().contains(gdk::MOD1_MASK | gdk::SHIFT_MASK) {
                        DrawBrush::Yellow
                    } else if e.get_state().contains(gdk::MOD1_MASK) {
                        DrawBrush::Blue
                    } else if e.get_state().contains(gdk::SHIFT_MASK) {
                        DrawBrush::Red
                    } else {
                        DrawBrush::Green
                    };

                    DrawShape {
                        orig: square,
                        dest: square,
                        brush,
                        opacity: 0.4,
                        stroke: 0.2,
                    }
                });

                return Some(Inhibit(false));
            },
            _ => {},
        }

        None
    }

    pub(crate) fn mouse_move(&mut self, widget: &DrawingArea, e: &EventMotion) -> Option<Inhibit> {
        if let Some(ref mut drawing) = self.drawing {
            drawing.dest = pos_to_square(widget, e.get_position()).unwrap_or(drawing.orig);
            widget.queue_draw();
        }

        None
    }

    pub(crate) fn mouse_up(&mut self, widget: &DrawingArea, e: &EventButton) -> Option<Inhibit> {
        if let Some(mut drawing) = self.drawing.take() {
            drawing.dest = pos_to_square(widget, e.get_position()).unwrap_or(drawing.orig);

            // remove or add shape
            let num_shapes = self.shapes.len();
            self.shapes.retain(|s| s.orig != drawing.orig || s.dest != drawing.dest);
            if num_shapes == self.shapes.len() {
                self.shapes.push(drawing);
            }

            widget.queue_draw();
        }

        None
    }

    pub(crate) fn render_cairo(&self, cr: &Context) {
        for shape in &self.shapes {
            shape.render_cairo(cr);
        }

        self.drawing.as_ref().map(|shape| shape.render_cairo(cr));
    }
}
