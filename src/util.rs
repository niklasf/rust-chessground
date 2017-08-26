use std::cmp::min;
use std::f64::consts::PI;

use shakmaty::{Color, Square};

use gtk::prelude::*;
use gtk::DrawingArea;
use cairo::prelude::*;
use cairo::Matrix;

pub fn compute_matrix(widget: &DrawingArea, orientation: Color) -> Matrix {
    let mut matrix = Matrix::identity();

    let w = widget.get_allocated_width();
    let h = widget.get_allocated_height();
    let size = min(w, h);

    matrix.translate(w as f64 / 2.0, h as f64 / 2.0);
    matrix.scale(size as f64 / 10.0, size as f64 / 10.0);
    matrix.rotate(orientation.fold(0.0, PI));
    matrix.translate(-4.0, -4.0);

    matrix
}

pub fn pos_to_square(widget: &DrawingArea, orientation: Color, (x, y): (f64, f64)) -> Option<Square> {
    compute_matrix(widget, orientation).try_invert().ok().and_then(|matrix| {
        let (x, y) = matrix.transform_point(x, y);
        let (x, y) = (x.floor(), y.floor());
        if 0f64 <= x && x <= 7f64 && 0f64 <= y && y <= 7f64 {
            Square::from_coords(x as i8, 7 - y as i8)
        } else {
            None
        }
    })
}

pub fn invert_pos(widget: &DrawingArea, orientation: Color, (x, y): (f64, f64)) -> (f64, f64) {
    compute_matrix(widget, orientation)
        .try_invert()
        .map(|m| m.transform_point(x, y))
        .unwrap_or((x, y))
}
