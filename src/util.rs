use shakmaty::Square;

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
