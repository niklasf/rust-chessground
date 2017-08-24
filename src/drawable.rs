extern crate gtk;
extern crate gdk;
extern crate cairo;
extern crate rsvg;
extern crate shakmaty;

use std::f64::consts::PI;

use shakmaty::Square;

use gtk::prelude::*;
use gtk::DrawingArea;
use gdk::EventButton;
use cairo::Context;

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

        let orig_x = 0.5 + self.orig.file() as f64;
        let orig_y = 7.5 - self.orig.rank() as f64;
        let dest_x = 0.5 + self.dest.file() as f64;
        let dest_y = 7.5 - self.dest.rank() as f64;

        if self.orig == self.dest {
            // draw circle
            cr.arc(dest_x, dest_y, 0.5 * (1.0 - self.stroke), 0.0, 2.0 * PI);
            cr.stroke();
        } else {
            // draw arrow
            let marker_size = 0.75;
            let margin = 0.1;

            let (dx, dy) = (dest_x - orig_x, dest_y - orig_y);
            let hypot = dx.hypot(dy);

            let shaft_x = dest_x - dx * (marker_size + margin) / hypot;
            let shaft_y = dest_y - dy * (marker_size + margin) / hypot;

            let head_x = dest_x - dx * margin / hypot;
            let head_y = dest_y - dy * margin / hypot;

            // shaft
            cr.move_to(orig_x, orig_y);
            cr.line_to(shaft_x, shaft_y);
            cr.stroke();

            // arrow head
            cr.line_to(shaft_x - dy * 0.5 * marker_size / hypot,
                       shaft_y + dx * 0.5 * marker_size / hypot);
            cr.line_to(head_x, head_y);
            cr.line_to(shaft_x + dy * 0.5 * marker_size / hypot,
                       shaft_y - dx * 0.5 * marker_size / hypot);
            cr.line_to(shaft_x, shaft_y);
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

    pub(crate) fn mouse_down(&mut self, widget: &DrawingArea, square: Option<Square>, e: &EventButton) -> Option<Inhibit> {
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
                self.drawing = square.map(|square| {
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
                        opacity: 0.5,
                        stroke: 0.2,
                    }
                });

                return Some(Inhibit(false));
            },
            _ => {},
        }

        None
    }

    pub(crate) fn mouse_move(&mut self, widget: &DrawingArea, square: Option<Square>) -> Option<Inhibit> {
        if let Some(ref mut drawing) = self.drawing {
            drawing.dest = square.unwrap_or(drawing.orig);
            widget.queue_draw();
        }

        None
    }

    pub(crate) fn mouse_up(&mut self, widget: &DrawingArea, square: Option<Square>) -> Option<Inhibit> {
        if let Some(mut drawing) = self.drawing.take() {
            drawing.dest = square.unwrap_or(drawing.orig);

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
