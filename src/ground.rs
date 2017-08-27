use std::cmp::{min, max};
use std::rc::Rc;
use std::cell::RefCell;
use std::f64::consts::PI;

use shakmaty;
use shakmaty::{Square, Color, Role, Piece, Board, Bitboard, MoveList, Position, Chess, Setup};

use gtk;
use gtk::prelude::*;
use gtk::DrawingArea;
use gdk;
use gdk::{EventButton, EventMotion};
use cairo;
use cairo::prelude::*;
use cairo::{Context, RadialGradient};
use rsvg::HandleExt;

use option_filter::OptionFilterExt;

use time::SteadyTime;
use rand;
use rand::distributions::{IndependentSample, Range};

use relm::{Relm, Widget, Update, EventStream};

use util;
use pieceset;
use drawable::Drawable;
use pieceset::PieceSet;

pub struct Model {
    state: Rc<RefCell<BoardState>>,
}

#[derive(Msg)]
pub enum GroundMsg {
    UserMove { orig: Square, dest: Square },
}

pub struct Ground {
    drawing_area: DrawingArea,
    _model: Model,
}

impl Update for Ground {
    type Model = Model;
    type ModelParam = ();
    type Msg = GroundMsg;

    fn model(_: &Relm<Self>, _: ()) -> Model {
        Model {
            state: Rc::new(RefCell::new(BoardState::new())),
        }
    }

    fn update(&mut self, event: GroundMsg) {
    }
}

impl Widget for Ground {
    type Root = DrawingArea;

    fn root(&self) -> Self::Root {
        self.drawing_area.clone()
    }

    fn view(relm: &Relm<Self>, model: Model) -> Self {
        let drawing_area = DrawingArea::new();

        drawing_area.add_events((gdk::BUTTON_PRESS_MASK |
                                 gdk::BUTTON_RELEASE_MASK |
                                 gdk::POINTER_MOTION_MASK).bits() as i32);

        {
            let weak_state = Rc::downgrade(&model.state);
            drawing_area.connect_draw(move |widget, cr| {
                if let Some(state) = weak_state.upgrade() {
                    let mut state = state.borrow_mut();
                    state.now = SteadyTime::now();
                    let animating = state.pieces.is_animating(state.now) || promoting_is_animating(&state);

                    let matrix = util::compute_matrix(widget, state.orientation);
                    cr.set_matrix(matrix);

                    draw_border(cr, &state);
                    draw_board(cr, &state);
                    draw_check(cr, &state);
                    state.pieces.render(cr, &state);
                    state.drawable.render(cr);
                    draw_move_hints(cr, &state);
                    draw_drag(cr, &state);
                    draw_promoting(cr, &state);

                    let weak_state = weak_state.clone();
                    let widget = widget.clone();
                    if animating {
                        gtk::idle_add(move || {
                            if let Some(state) = weak_state.upgrade() {
                                let state = state.borrow();
                                state.pieces.queue_animation(&state, &widget);
                                promoting_queue_animation(&state, &widget);
                            }
                            Continue(false)
                        });
                    }
                }
                Inhibit(false)
            });
        }

        {
            let state = Rc::downgrade(&model.state);
            let stream = relm.stream().clone();
            drawing_area.connect_button_press_event(move |widget, e| {
                if let Some(state) = state.upgrade() {
                    let mut state = state.borrow_mut();

                    let context = EventContext {
                        drawing_area: &widget,
                        stream: &stream,
                        pos: e.get_position(),
                        square: util::pos_to_square(widget, state.orientation, e.get_position()),
                    };

                    if !promoting_mouse_down(&mut state, widget, context.square) {
                        state.selection_mouse_down(&context, e);
                        drag_mouse_down(&mut state, widget, context.square, e);
                        state.drawable.mouse_down(widget, context.square, e);
                    }
                }
                Inhibit(false)
            });
        }

        {
            let state = Rc::downgrade(&model.state);
            let stream = relm.stream().clone();
            drawing_area.connect_button_release_event(move |widget, e| {
                if let Some(state) = state.upgrade() {
                    let mut state = state.borrow_mut();

                    let context = EventContext {
                        drawing_area: widget,
                        stream: &stream,
                        pos: e.get_position(),
                        square: util::pos_to_square(widget, state.orientation, e.get_position()),
                    };

                    state.drag_mouse_up(&context);
                    state.drawable.mouse_up(&context);
                }
                Inhibit(false)
            });
        }

        {
            let state = Rc::downgrade(&model.state);
            drawing_area.connect_motion_notify_event(move |widget, e| {
                if let Some(state) = state.upgrade() {
                    let mut state = state.borrow_mut();
                    let square = util::pos_to_square(widget, state.orientation, e.get_position());

                    if !promoting_mouse_move(&mut state, widget, square) {
                        drag_mouse_move(&mut state, widget, square, e);
                        state.drawable.mouse_move(widget, square);
                    }
                }
                Inhibit(false)
            });
        }

        Ground {
            drawing_area,
            _model: model,
        }
    }
}

pub struct EventContext<'a> {
    pub drawing_area: &'a DrawingArea,
    pub stream: &'a EventStream<GroundMsg>,
    pub pos: (f64, f64),
    pub square: Option<Square>,
}

pub const ANIMATE_DURATION: f64 = 0.2;

fn ease_in_out_cubic(start: f64, end: f64, elapsed: f64, duration: f64) -> f64 {
    let t = elapsed / duration;
    let ease = if t >= 1.0 {
        1.0
    } else if t >= 0.5 {
        (t - 1.0) * (2.0 * t - 2.0) * (2.0 * t - 2.0) + 1.0
    } else if t >= 0.0 {
        4.0 * t * t * t
    } else {
        0.0
    };
    start + (end - start) * ease
}

struct Figurine {
    square: Square,
    piece: Piece,
    pos: (f64, f64),
    time: SteadyTime,
    fading: bool,
    replaced: bool,
    dragging: bool,
}

impl Figurine {
    fn pos(&self, now: SteadyTime) -> (f64, f64) {
        let end = util::square_to_inverted(self.square);
        if self.dragging {
            end
        } else if self.fading {
            self.pos
        } else {
            (ease_in_out_cubic(self.pos.0, end.0, self.elapsed(now), ANIMATE_DURATION),
             ease_in_out_cubic(self.pos.1, end.1, self.elapsed(now), ANIMATE_DURATION))
        }
    }

    fn alpha(&self, now: SteadyTime) -> f64 {
        if self.dragging {
            0.2 * self.alpha_easing(1.0, now)
        } else {
            self.drag_alpha(now)
        }
    }

    fn drag_alpha(&self, now: SteadyTime) -> f64 {
        let base = if self.fading && self.replaced { 0.5 } else { 1.0 };
        self.alpha_easing(base, now)
    }

    fn alpha_easing(&self, base: f64, now: SteadyTime) -> f64 {
        if self.fading {
            base * ease_in_out_cubic(1.0, 0.0, self.elapsed(now), ANIMATE_DURATION)
        } else {
            base
        }
    }

    fn elapsed(&self, now: SteadyTime) -> f64 {
        (now - self.time).num_milliseconds() as f64 / 1000.0
    }

    fn is_animating(&self, now: SteadyTime) -> bool {
        !self.dragging && self.elapsed(now) <= ANIMATE_DURATION &&
        (self.fading || self.pos != util::square_to_inverted(self.square))
    }

    fn queue_animation(&self, state: &BoardState, widget: &DrawingArea) {
        if self.is_animating(state.now) {
            let matrix = util::compute_matrix(widget, state.orientation);
            let pos = self.pos(state.now);

            let (x1, y1) = matrix.transform_point(pos.0 - 0.5, pos.1 - 0.5);
            let (x2, y2) = matrix.transform_point(pos.0 + 0.5, pos.1 + 0.5);
            let (x3, y3) = matrix.transform_point(self.square.file() as f64, 7.0 - self.square.rank() as f64);
            let (x4, y4) = matrix.transform_point(1.0 + self.square.file() as f64, 8.0 - self.square.rank() as f64);

            let xmin = min(
                min(x1.floor() as i32, x2.floor() as i32),
                min(x3.floor() as i32, x4.floor() as i32));
            let xmax = max(
                max(x1.ceil() as i32, x2.ceil() as i32),
                max(x3.ceil() as i32, x4.ceil() as i32));
            let ymin = min(
                min(y1.floor() as i32, y2.floor() as i32),
                min(y3.floor() as i32, y4.floor() as i32));
            let ymax = max(
                max(y1.ceil() as i32, y2.ceil() as i32),
                max(y3.ceil() as i32, y4.ceil() as i32));

            widget.queue_draw_area(xmin, ymin, xmax - xmin, ymax - ymin);
        }
    }

    fn render(&self, cr: &Context, state: &BoardState) {
        if let Some(ref promoting) = state.promoting {
            // hide piece while promotion dialog is open
            if promoting.orig == self.square {
                return;
            }
        }

        cr.push_group();

        let (x, y) = self.pos(state.now);
        cr.translate(x, y);
        cr.rotate(state.orientation.fold(0.0, PI));
        cr.translate(-0.5, -0.5);
        cr.scale(state.piece_set.scale(), state.piece_set.scale());

        state.piece_set.by_piece(&self.piece).render_cairo(cr);

        cr.pop_group_to_source();
        cr.paint_with_alpha(self.alpha(state.now));
    }
}

struct Pieces {
    board: Board,
    figurines: Vec<Figurine>,
}

impl Pieces {
    pub fn new() -> Pieces {
        Pieces::new_from_board(&Board::new())
    }

    pub fn new_from_board(board: &Board) -> Pieces {
        Pieces {
            board: board.clone(),
            figurines: board.pieces().map(|(square, piece)| Figurine {
                square,
                piece,
                pos: (0.5 + square.file() as f64, 7.5 - square.rank() as f64),
                time: SteadyTime::now(),
                fading: false,
                replaced: false,
                dragging: false,
            }).collect()
        }
    }

    pub fn set_board(&mut self, board: &Board) {
        let now = SteadyTime::now();

        // clean and freeze previous animation
        self.figurines.retain(|f| f.alpha(now) > 0.0001);
        for figurine in &mut self.figurines {
            if !figurine.fading {
                figurine.pos = figurine.pos(now);
                figurine.time = now;
            }
        }

        // diff
        let mut removed = Bitboard(0);
        let mut added = Vec::new();

        for square in self.board.occupied() | board.occupied() {
            let old = self.board.piece_at(square);
            let new = board.piece_at(square);
            if old != new {
                if old.is_some() {
                    removed.add(square);
                }
                if let Some(new) = new {
                    added.push((square, new));
                }
            }
        }

        // try to match additions and removals
        let mut matched = Vec::new();
        added.retain(|&(square, piece)| {
            let best = removed
                .filter(|sq| self.board.by_piece(piece).contains(*sq))
                .min_by_key(|sq| sq.distance(square));

            if let Some(best) = best {
                removed.remove(best);
                matched.push((best, square));
                false
            } else {
                true
            }
        });

        for square in removed {
            for figurine in &mut self.figurines {
                if !figurine.fading && figurine.square == square {
                    figurine.fading = true;
                    figurine.replaced = board.occupied().contains(square);
                    figurine.time = now;
                }
            }
        }

        for (orig, dest) in matched {
            if let Some(figurine) = self.figurines.iter_mut().find(|f| !f.fading && f.square == orig) {
                figurine.square = dest;
                figurine.time = now;
            }
        }

        for (square, piece) in added {
            self.figurines.push(Figurine {
                square: square,
                piece: piece,
                pos: (0.5 + square.file() as f64, 7.5 - square.rank() as f64),
                time: now,
                fading: false,
                replaced: false,
                dragging: false,
            });
        }

        self.board = board.clone();
    }

    pub fn occupied(&self) -> Bitboard {
        self.board.occupied()
    }

    pub fn render(&self, cr: &Context, state: &BoardState) {
        let now = SteadyTime::now();

        for figurine in &self.figurines {
            if figurine.fading {
                figurine.render(cr, state);
            }
        }

        for figurine in &self.figurines {
            if !figurine.fading && !figurine.is_animating(now) {
                figurine.render(cr, state);
            }
        }

        for figurine in &self.figurines {
            if !figurine.fading && figurine.is_animating(now) {
                figurine.render(cr, state);
            }
        }
    }

    pub fn figurine_at(&self, square: Square) -> Option<&Figurine> {
        self.figurines.iter().find(|f| !f.fading && f.square == square)
    }

    pub fn figurine_at_mut(&mut self, square: Square) -> Option<&mut Figurine> {
        self.figurines.iter_mut().find(|f| !f.fading && f.square == square)
    }

    pub fn dragging(&self) -> Option<&Figurine> {
        self.figurines.iter().find(|f| f.dragging)
    }

    pub fn dragging_mut(&mut self) -> Option<&mut Figurine> {
        self.figurines.iter_mut().find(|f| f.dragging)
    }

    pub fn is_animating(&self, now: SteadyTime) -> bool {
        self.figurines.iter().any(|f| f.is_animating(now))
    }

    pub fn queue_animation(&self, state: &BoardState, widget: &DrawingArea) {
        for figurine in &self.figurines {
            figurine.queue_animation(state, widget);
        }
    }
}

struct DragStart {
    pos: (f64, f64),
    square: Square,
}

struct BoardState {
    pieces: Pieces,
    orientation: Color,
    check: Option<Square>,
    selected: Option<Square>,
    last_move: Option<(Square, Square)>,
    drag_start: Option<DragStart>,
    piece_set: PieceSet,
    drawable: Drawable,
    now: SteadyTime,
    promoting: Option<Promoting>,
    legals: MoveList,
    pos: Chess,
}

impl BoardState {
    fn user_move(&mut self, orig: Square, dest: Square) {
        match self.pieces.board.piece_at(orig) {
            Some(Piece { role: Role::Pawn, color }) if color.fold(7, 0) == dest.rank() => {
                self.promoting = Some(Promoting {
                    orig, dest, hover: Some(dest), hover_since: SteadyTime::now()
                });
            },
            _ => self.on_user_move(orig, dest, None)
        }
    }

    fn on_user_move(&mut self, orig: Square, dest: Square, promotion: Option<Role>) {
        println!("user move: {} {}", orig, dest);

        let m = self.legals.drain(..).find(|m| m.from() == Some(orig) && m.to() == dest && m.promotion() == promotion);
        if let Some(m) = m {
            self.pos = self.pos.clone().play_unchecked(&m);
            self.pieces.set_board(self.pos.board());
            self.last_move = Some((m.to(), m.from().unwrap_or_else(|| m.to())));

            // respond
            self.legals.clear();
            self.pos.legal_moves(&mut self.legals);
            if !self.legals.is_empty() {
                let mut rng = rand::thread_rng();
                let idx = Range::new(0, self.legals.len()).ind_sample(&mut rng);
                let m = &self.legals[idx];
                self.pos = self.pos.clone().play_unchecked(m);
                self.pieces.set_board(self.pos.board());
                self.last_move = Some((m.to(), m.from().unwrap_or_else(|| m.to())));
            }
        }

        self.legals.clear();
        self.pos.legal_moves(&mut self.legals);
        self.check = self.pos.board().king_of(self.pos.turn()).filter(|_| self.pos.checkers().any());
    }

    fn move_targets(&self, orig: Square) -> Bitboard {
        self.legals.iter().filter(|m| m.from() == Some(orig)).map(|m| m.to()).collect()
    }

    fn valid_move(&self, orig: Square, dest: Square) -> bool {
        self.move_targets(orig).contains(dest)
    }
}

impl BoardState {
    fn new() -> Self {
        let pos = Chess::default();

        let mut state = BoardState {
            pieces: Pieces::new(),
            orientation: Color::White,
            check: None,
            last_move: None,
            selected: None,
            drag_start: None,
            promoting: None,
            drawable: Drawable::new(),
            piece_set: pieceset::PieceSet::merida(),
            legals: MoveList::new(),
            pos: pos.clone(),
            now: SteadyTime::now(),
        };

        pos.legal_moves(&mut state.legals);

        state
    }
}

struct Promoting {
    orig: Square,
    dest: Square,
    hover: Option<Square>,
    hover_since: SteadyTime,
}

impl Promoting {
    fn elapsed(&self, now: SteadyTime) -> f64 {
        (now - self.hover_since).num_milliseconds() as f64 / 1000.0
    }

    fn orientation(&self) -> Color {
        Color::from_bool(self.dest.rank() > 4)
    }
}

fn promoting_mouse_down(state: &mut BoardState, widget: &DrawingArea, square: Option<Square>) -> bool {
    if let Some(promoting) = state.promoting.take() {
        widget.queue_draw();

        // animate the figurine when cancelling
        if let Some(figurine) = state.pieces.figurine_at_mut(promoting.orig) {
            figurine.pos = util::square_to_inverted(promoting.dest);
            figurine.time = SteadyTime::now();
        }

        if let Some(square) = square {
            let side = promoting.orientation();

            if square.file() == promoting.dest.file() {
                let role = match square.rank() {
                    r if r == side.fold(7, 0) => Some(Role::Queen),
                    r if r == side.fold(6, 1) => Some(Role::Rook),
                    r if r == side.fold(5, 2) => Some(Role::Bishop),
                    r if r == side.fold(4, 3) => Some(Role::Knight),
                    r if r == side.fold(3, 4) => Some(Role::King),
                    r if r == side.fold(2, 5) => Some(Role::Pawn),
                    _ => None,
                };

                if role.is_some() {
                    state.on_user_move(promoting.orig, promoting.dest, role);
                    return true;
                }
            }
        }
    }

    false
}

fn promoting_is_animating(state: &BoardState) -> bool {
    if let Some(ref promoting) = state.promoting {
        promoting.hover.is_some() && promoting.elapsed(state.now) <= 1.0
    } else {
        false
    }
}

fn promoting_queue_animation(state: &BoardState, widget: &DrawingArea) {
    if let Some(Promoting { hover: Some(hover), .. }) = state.promoting {
        queue_draw_square(widget, state.orientation, hover);
    }
}

fn promoting_mouse_move(state: &mut BoardState, widget: &DrawingArea, square: Option<Square>) -> bool {
    promoting_queue_animation(state, widget);

    let consume = if let Some(ref mut promoting) = state.promoting {
        if promoting.hover != square {
            promoting.hover = square;
            promoting.hover_since = SteadyTime::now();
        }
        true
    } else {
        false
    };

    promoting_queue_animation(state, widget);
    consume
}

impl BoardState {
    fn selection_mouse_down(&mut self, context: &EventContext, e: &EventButton) {
        let orig = self.selected.take();

        if e.get_button() == 1 {
            let dest = context.square;
            self.selected = dest.filter(|sq| self.pieces.occupied().contains(*sq));

            if let (Some(orig), Some(dest)) = (orig, dest) {
                if self.valid_move(orig, dest) {
                    self.selected = None;
                    context.stream.emit(GroundMsg::UserMove { orig, dest });
                } else if orig == dest {
                    self.selected = None;
                }
            }
        }

        context.drawing_area.queue_draw();
    }
}

fn drag_mouse_down(state: &mut BoardState, widget: &DrawingArea, square: Option<Square>, e: &EventButton) {
    if e.get_button() == 1 {
        if let Some(square) = square {
            if state.pieces.figurine_at(square).is_some() {
                state.drag_start = Some(DragStart {
                    pos: util::invert_pos(widget, state.orientation, e.get_position()),
                    square,
                });
            }
        }
    }
}

fn queue_draw_square(widget: &DrawingArea, orientation: Color, square: Square) {
    queue_draw_rect(widget, orientation, square.file() as f64, 7.0 - square.rank() as f64, 1.0, 1.0);
}

fn queue_draw_rect(widget: &DrawingArea, orientation: Color, x: f64, y: f64, width: f64, height: f64) {
    let matrix = util::compute_matrix(widget, orientation);
    let (x1, y1) = matrix.transform_point(x, y);
    let (x2, y2) = matrix.transform_point(x + width, y + height);

    let xmin = min(x1.floor() as i32, x2.floor() as i32);
    let ymin = min(y1.floor() as i32, y2.floor() as i32);
    let xmax = max(x1.ceil() as i32, x2.ceil() as i32);
    let ymax = max(y1.ceil() as i32, y2.ceil() as i32);

    widget.queue_draw_area(xmin, ymin, xmax - xmin, ymax - ymin);
}

fn drag_mouse_move(state: &mut BoardState, widget: &DrawingArea, square: Option<Square>, e: &EventMotion) {
    let pos = util::invert_pos(widget, state.orientation, e.get_position());

    if let Some(ref drag_start) = state.drag_start {
        let drag_distance = (drag_start.pos.0 - pos.0).hypot(drag_start.pos.1 - pos.1);
        if drag_distance >= 0.1 {
            if let Some(dragging) = state.pieces.figurine_at_mut(drag_start.square) {
                dragging.dragging = true;
            }
        }
    }

    if let Some(dragging) = state.pieces.dragging_mut() {
        // ensure orig square is selected
        if state.selected != Some(dragging.square) {
            state.selected = Some(dragging.square);
            widget.queue_draw();
        }

        // invalidate previous
        queue_draw_rect(widget, state.orientation, dragging.pos.0 - 0.5, dragging.pos.1 - 0.5, 1.0, 1.0);
        queue_draw_square(widget, state.orientation, dragging.square);
        if let Some(sq) = util::inverted_to_square(dragging.pos) {
            queue_draw_square(widget, state.orientation, sq);
        }

        // update position
        dragging.pos = pos;
        dragging.time = SteadyTime::now();

        // invalidate new
        queue_draw_rect(widget, state.orientation, dragging.pos.0 - 0.5, dragging.pos.1 - 0.5, 1.0, 1.0);
        if let Some(sq) = square {
            queue_draw_square(widget, state.orientation, sq);
        }
    }
}

impl BoardState {
    fn drag_mouse_up(&mut self, context: &EventContext) {
        self.drag_start = None;

        let m = if let Some(dragging) = self.pieces.dragging_mut() {
            context.drawing_area.queue_draw();

            let dest = context.square.unwrap_or(dragging.square);
            dragging.pos = util::square_to_inverted(dest);
            dragging.time = SteadyTime::now();
            dragging.dragging = false;

            if dragging.square != dest && !dragging.fading {
                self.selected = None;
                Some((dragging.square, dest))
            } else {
                None
            }
        } else {
            None
        };

        if let Some((orig, dest)) = m {
            if self.valid_move(orig, dest) {
                self.user_move(orig, dest);
            }
        }
    }
}

fn draw_text(cr: &Context, orientation: Color, (x, y): (f64, f64), text: &str) {
    let font = cr.font_extents();
    let e = cr.text_extents(text);

    cr.save();
    cr.translate(x, y);
    cr.rotate(orientation.fold(0.0, PI));
    cr.move_to(-0.5 * e.width, 0.5 * font.ascent);
    cr.show_text(text);
    cr.restore();
}

fn draw_border(cr: &Context, state: &BoardState) {
    let border = cairo::SolidPattern::from_rgb(0.2, 0.2, 0.5);
    cr.set_source(&border);
    cr.rectangle(-0.5, -0.5, 9.0, 9.0);
    cr.fill();

    cr.set_font_size(0.20);
    cr.set_source_rgb(0.8, 0.8, 0.8);

    for (rank, glyph) in ["1", "2", "3", "4", "5", "6", "7", "8"].iter().enumerate() {
        draw_text(cr, state.orientation, (-0.25, 7.5 - rank as f64), glyph);
        draw_text(cr, state.orientation, (8.25, 7.5 - rank as f64), glyph);
    }

    for (file, glyph) in ["a", "b", "c", "d", "e", "f", "g", "h"].iter().enumerate() {
        draw_text(cr, state.orientation, (0.5 + file as f64, -0.25), glyph);
        draw_text(cr, state.orientation, (0.5 + file as f64, 8.25), glyph);
    }
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

    if let Some(selected) = state.selected {
        cr.rectangle(selected.file() as f64, 7.0 - selected.rank() as f64, 1.0, 1.0);
        cr.set_source_rgba(0.08, 0.47, 0.11, 0.5);
        cr.fill();

        if let Some(hovered) = state.pieces.dragging().and_then(|d| util::inverted_to_square(d.pos)) {
            if state.valid_move(selected, hovered) {
                cr.rectangle(hovered.file() as f64, 7.0 - hovered.rank() as f64, 1.0, 1.0);
                cr.set_source_rgba(0.08, 0.47, 0.11, 0.25);
                cr.fill();
            }
        }
    }

    if let Some((orig, dest)) = state.last_move {
        cr.set_source_rgba(0.61, 0.78, 0.0, 0.41);
        cr.rectangle(orig.file() as f64, 7.0 - orig.rank() as f64, 1.0, 1.0);
        cr.fill();

        if dest != orig {
            cr.rectangle(dest.file() as f64, 7.0 - dest.rank() as f64, 1.0, 1.0);
            cr.fill();
        }
    }
}

fn draw_move_hints(cr: &Context, state: &BoardState) {
    if let Some(selected) = state.selected {
        cr.set_source_rgba(0.08, 0.47, 0.11, 0.5);

        let radius = 0.12;
        let corner = 1.8 * radius;

        for square in state.move_targets(selected) {
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

fn draw_drag(cr: &Context, state: &BoardState) {
    if let Some(dragging) = state.pieces.dragging() {
        cr.push_group();
        cr.translate(dragging.pos.0, dragging.pos.1);
        cr.rotate(state.orientation.fold(0.0, PI));
        cr.translate(-0.5, -0.5);
        cr.scale(state.piece_set.scale(), state.piece_set.scale());
        state.piece_set.by_piece(&dragging.piece).render_cairo(cr);
        cr.pop_group_to_source();
        cr.paint_with_alpha(dragging.drag_alpha(state.now));
    }
}

fn draw_promoting(cr: &Context, state: &BoardState) {
    if let Some(ref promoting) = state.promoting {
        cr.rectangle(0.0, 0.0, 8.0, 8.0);
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.5);
        cr.fill();

        for (offset, role) in [Role::Queen, Role::Rook, Role::Bishop, Role::Knight, Role::King, Role::Pawn].iter().enumerate() {
            if !state.legals.iter().any(|m| {
                m.from() == Some(promoting.orig) &&
                m.to() == promoting.dest &&
                m.promotion() == Some(*role)
            }) {
                continue;
            }

            let rank = promoting.orientation().fold(7 - offset as i8, offset as i8);
            let light = promoting.dest.file() + rank & 1 == 1;

            cr.save();
            cr.rectangle(promoting.dest.file() as f64, 7.0 - rank as f64, 1.0, 1.0);
            cr.clip_preserve();

            if light {
                cr.set_source_rgb(0.25, 0.25, 0.25);
            } else {
                cr.set_source_rgb(0.18, 0.18, 0.18);
            }
            cr.fill();

            let radius = match promoting.hover {
                Some(hover) if hover.file() == promoting.dest.file() && hover.rank() == rank => {
                    cr.set_source_rgb(
                        ease_in_out_cubic(0.69, 1.0, promoting.elapsed(state.now), 1.0),
                        ease_in_out_cubic(0.69, 0.65, promoting.elapsed(state.now), 1.0),
                        ease_in_out_cubic(0.69, 0.0, promoting.elapsed(state.now), 1.0));

                    ease_in_out_cubic(0.5, 0.5f64.hypot(0.5), promoting.elapsed(state.now), 1.0)
                },
                _ => {
                    cr.set_source_rgb(0.69, 0.69, 0.69);
                    0.5
                },
            };

            cr.arc(0.5 + promoting.dest.file() as f64, 7.5 - rank as f64, radius, 0.0, 2.0 * PI);
            cr.fill();

            cr.translate(0.5 + promoting.dest.file() as f64, 7.5 - rank as f64);
            cr.scale(2f64.sqrt() * radius, 2f64.sqrt() * radius);
            cr.translate(-0.5, -0.5);
            cr.scale(state.piece_set.scale(), state.piece_set.scale());
            state.piece_set.by_piece(&role.of(Color::White)).render_cairo(cr);

            cr.restore();
        }
    }
}
