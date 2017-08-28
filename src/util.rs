use std::cmp::{min, max};
use std::f64::consts::PI;

use shakmaty::{Color, Square};

use gtk::prelude::*;
use gtk::DrawingArea;
use cairo::prelude::*;
use cairo::Matrix;

pub fn fmin(a: f64, b: f64) -> f64 {
    if a < b { a } else { b }
}

pub fn fmax(a: f64, b: f64) -> f64 {
    if a > b { a } else { b }
}

pub fn ease(start: f64, end: f64, elapsed: f64, duration: f64) -> f64 {
    // cubic in out easing
    let t = fmax(0.0, fmin(1.0, elapsed / duration));
    let ease = if t >= 0.5 {
        (t - 1.0) * (2.0 * t - 2.0) * (2.0 * t - 2.0) + 1.0
    } else {
        4.0 * t * t * t
    };
    start + (end - start) * ease
}

pub fn compute_matrix(widget: &DrawingArea, orientation: Color) -> Matrix {
    let mut matrix = Matrix::identity();

    let w = widget.get_allocated_width();
    let h = widget.get_allocated_height();
    let size = min(w, h);

    matrix.translate(w as f64 / 2.0, h as f64 / 2.0);
    matrix.scale(size as f64 / 9.0, size as f64 / 9.0);
    matrix.rotate(orientation.fold(0.0, PI));
    matrix.translate(-4.0, -4.0);

    matrix
}

pub fn invert_pos(widget: &DrawingArea, orientation: Color, (x, y): (f64, f64)) -> (f64, f64) {
    compute_matrix(widget, orientation)
        .try_invert()
        .map(|m| m.transform_point(x, y))
        .unwrap_or((x, y))
}

pub fn pos_to_square((x, y): (f64, f64)) -> Option<Square> {
    let (x, y) = (x.floor(), y.floor());
    if 0f64 <= x && x <= 7f64 && 0f64 <= y && y <= 7f64 {
        Square::from_coords(x as i8, 7 - y as i8)
    } else {
        None
    }
}

pub fn square_to_pos(square: Square) -> (f64, f64) {
    (0.5 + square.file() as f64, 7.5 - square.rank() as f64)
}

pub fn queue_draw_square(widget: &DrawingArea, orientation: Color, square: Square) {
    queue_draw_rect(widget, orientation, square.file() as f64, 7.0 - square.rank() as f64, 1.0, 1.0);
}

pub fn queue_draw_rect(widget: &DrawingArea, orientation: Color, x: f64, y: f64, width: f64, height: f64) {
    let matrix = compute_matrix(widget, orientation);
    let (x1, y1) = matrix.transform_point(x, y);
    let (x2, y2) = matrix.transform_point(x + width, y + height);

    let xmin = min(x1.floor() as i32, x2.floor() as i32);
    let ymin = min(y1.floor() as i32, y2.floor() as i32);
    let xmax = max(x1.ceil() as i32, x2.ceil() as i32);
    let ymax = max(y1.ceil() as i32, y2.ceil() as i32);

    widget.queue_draw_area(xmin, ymin, xmax - xmin, ymax - ymin);
}
