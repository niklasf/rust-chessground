struct Promotable {
    promoting: Option<Promoting>,
}

struct Promoting {
    orig: Square,
    dest: Square,
    hover: Option<Hover>,
}

struct Hover {
    square: Square,
    since: SteadyTime,
}
