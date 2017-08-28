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

use std::f64::consts::PI;

use gdk;
use gdk::EventButton;
use cairo::Context;

use shakmaty::Square;

use ground::{EventContext, GroundMsg};

/// Shape colors.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum DrawBrush {
    Green,
    Red,
    Blue,
    Yellow,
}

/// An arrow or circle drawn on the board.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct DrawShape {
    orig: Square,
    dest: Square,
    brush: DrawBrush,
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

    pub(crate) fn mouse_down(&mut self, ctx: &EventContext, e: &EventButton) {
        if !self.enabled {
            return;
        }

        match e.get_button() {
            1 => {
                if self.erase_on_click && !self.shapes.is_empty() {
                    self.shapes.clear();
                    ctx.stream().emit(GroundMsg::ShapesChanged(self.shapes.clone()));
                    ctx.widget().queue_draw();
                }
            }
            3 => {
                self.drawing = ctx.square().map(|square| {
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
                    }
                });

                ctx.widget().queue_draw();
            }
            _ => {}
        }
    }

    pub(crate) fn mouse_move(&mut self, ctx: &EventContext) {
        if let Some(ref mut drawing) = self.drawing {
            let dest = ctx.square().unwrap_or(drawing.orig);
            if drawing.dest != dest {
                ctx.widget().queue_draw();
            }
            drawing.dest = dest;
        }
    }

    pub(crate) fn mouse_up(&mut self, ctx: &EventContext) {
        if let Some(mut drawing) = self.drawing.take() {
            if self.enabled {
                drawing.dest = ctx.square().unwrap_or(drawing.orig);

                // remove or add shape
                let num_shapes = self.shapes.len();
                self.shapes.retain(|s| s.orig != drawing.orig || s.dest != drawing.dest);
                if num_shapes == self.shapes.len() {
                    self.shapes.push(drawing);
                }

                ctx.stream().emit(GroundMsg::ShapesChanged(self.shapes.clone()));
            }

            ctx.widget().queue_draw();
        }
    }

    pub(crate) fn draw(&self, cr: &Context) {
        for shape in &self.shapes {
            shape.draw(cr);
        }

        self.drawing.as_ref().map(|shape| shape.draw(cr));
    }
}

impl DrawShape {
    /// First square.
    pub fn orig(&self) -> Square {
        self.orig
    }

    /// Second square.
    pub fn dest(&self) -> Square {
        self.dest
    }

    /// Shape color.
    pub fn brush(&self) -> DrawBrush {
        self.brush
    }

    /// Check if the shape is a circle.
    pub fn is_circle(&self) -> bool {
        self.orig == self.dest
    }

    /// Check if the shape is an arrow.
    pub fn is_arrow(&self) -> bool {
        self.orig != self.dest
    }

    fn draw(&self, cr: &Context) {
        let opacity = 0.5;

        match self.brush {
            DrawBrush::Green => cr.set_source_rgba(0.08, 0.47, 0.11, opacity),
            DrawBrush::Red => cr.set_source_rgba(0.53, 0.13, 0.13, opacity),
            DrawBrush::Blue => cr.set_source_rgba(0.0, 0.19, 0.53, opacity),
            DrawBrush::Yellow => cr.set_source_rgba(0.90, 0.94, 0.0, opacity),
        }

        let orig_x = 0.5 + self.orig.file() as f64;
        let orig_y = 7.5 - self.orig.rank() as f64;
        let dest_x = 0.5 + self.dest.file() as f64;
        let dest_y = 7.5 - self.dest.rank() as f64;

        if self.is_circle() {
            // draw circle
            let stroke = 0.05;
            cr.set_line_width(stroke);
            cr.arc(dest_x, dest_y, 0.5 * (1.0 - stroke), 0.0, 2.0 * PI);
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

            let stroke = 0.2;
            cr.set_line_width(stroke);

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
