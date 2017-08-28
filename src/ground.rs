use std::rc::Rc;
use std::cell::RefCell;
use std::f64::consts::PI;

use shakmaty::{Square, Color, Role, Board, Bitboard, MoveList, Position, Chess};

use gtk;
use gtk::prelude::*;
use gtk::DrawingArea;
use gdk;
use gdk::{EventButton, EventMotion};
use cairo;
use cairo::prelude::*;
use cairo::{Context, RadialGradient, Matrix};

use relm::{Relm, Widget, Update, EventStream};

use util;
use util::pos_to_square;
use pieceset;
use pieces::Pieces;
use drawable::Drawable;
use promotable::Promotable;
use pieceset::PieceSet;

pub struct Model {
    state: Rc<RefCell<State>>,
}

#[derive(Msg)]
pub enum GroundMsg {
    SetPosition {
        board: Board,
        legals: MoveList,
        last_move: Option<(Square, Square)>,
        check: Option<Square>,
    },
    UserMove(Square, Square, Option<Role>),
    ShapesChanged,
}

type Stream = EventStream<GroundMsg>;

pub struct Ground {
    drawing_area: DrawingArea,
    model: Model,
}

impl Update for Ground {
    type Model = Model;
    type ModelParam = ();
    type Msg = GroundMsg;

    fn model(_: &Relm<Self>, _: ()) -> Model {
        Model {
            state: Rc::new(RefCell::new(State::new())),
        }
    }

    fn update(&mut self, event: GroundMsg) {
        let mut state = self.model.state.borrow_mut();

        match event {
            GroundMsg::UserMove(orig, dest, None) if state.board_state.valid_move(orig, dest) => {
                if state.board_state.legals.iter().any(|m| m.from() == Some(orig) && m.to() == dest && m.promotion().is_some()) {
                    state.promotable.start_promoting(orig, dest);
                    self.drawing_area.queue_draw();
                }
            },
            GroundMsg::SetPosition { board, legals, last_move, check } => {
                state.pieces.set_board(board);
                state.board_state.legals = legals;
                state.board_state.last_move = last_move;
                state.board_state.check = check;

                self.drawing_area.queue_draw();
            },
            _ => {}
        }
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
                    let state = state.borrow();
                    let animating = state.is_animating();

                    // set transform
                    let matrix = util::compute_matrix(widget, state.board_state.orientation);
                    cr.set_matrix(matrix);

                    // draw
                    state.board_state.draw(cr);
                    state.pieces.draw(cr, &state.board_state, &state.promotable);
                    state.drawable.draw(cr);
                    state.pieces.draw_drag(cr, &state.board_state);
                    state.promotable.draw(cr, &state.board_state);

                    // queue next draw for animation
                    let weak_state = weak_state.clone();
                    let widget = widget.clone();
                    if animating {
                        gtk::idle_add(move || {
                            if let Some(state) = weak_state.upgrade() {
                                state.borrow().queue_animation(&widget);
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
                    state.button_press_event(&stream, widget, e);
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
                    state.button_release_event(&stream, widget, e);
                }
                Inhibit(false)
            });
        }

        {
            let state = Rc::downgrade(&model.state);
            let stream = relm.stream().clone();
            drawing_area.connect_motion_notify_event(move |widget, e| {
                if let Some(state) = state.upgrade() {
                    let mut state = state.borrow_mut();
                    state.motion_notify_event(&stream, widget, e);
                }
                Inhibit(false)
            });
        }

        drawing_area.set_hexpand(true);
        drawing_area.set_vexpand(true);
        drawing_area.show();

        Ground {
            drawing_area,
            model,
        }
    }
}

struct State {
    board_state: BoardState,
    drawable: Drawable,
    promotable: Promotable,
    pieces: Pieces,
}

impl State {
    fn new() -> State {
        State {
            board_state: BoardState::new(),
            drawable: Drawable::new(),
            promotable: Promotable::new(),
            pieces: Pieces::new(),
        }
    }

    fn is_animating(&self) -> bool {
        self.promotable.is_animating() || self.pieces.is_animating()
    }

    fn queue_animation(&self, drawing_area: &DrawingArea) {
        let ctx = WidgetContext::new(&self.board_state, drawing_area);
        self.pieces.queue_animation(&ctx);
        self.promotable.queue_animation(&ctx);
    }

    fn button_release_event(&mut self, stream: &Stream, drawing_area: &DrawingArea, e: &EventButton) {
        let ctx = EventContext::new(&self.board_state, stream, drawing_area, e.get_position());
        self.pieces.drag_mouse_up(&ctx);
        self.drawable.mouse_up(&ctx);
    }

    fn motion_notify_event(&mut self, stream: &Stream, drawing_area: &DrawingArea, e: &EventMotion) {
        let ctx = EventContext::new(&self.board_state, stream, drawing_area, e.get_position());
        self.promotable.mouse_move(&ctx);
        self.pieces.drag_mouse_move(&ctx);
        self.drawable.mouse_move(&ctx);
    }

    fn button_press_event(&mut self, stream: &Stream, drawing_area: &DrawingArea, e: &EventButton) {
        let ctx = EventContext::new(&self.board_state, stream, drawing_area, e.get_position());
        let promotable = &mut self.promotable;
        let pieces = &mut self.pieces;

        if let Inhibit(false) = promotable.mouse_down(pieces, &ctx) {
            pieces.selection_mouse_down(&ctx, e);
            pieces.drag_mouse_down(&ctx, e);
            self.drawable.mouse_down(&ctx, e);
        }
    }
}

pub(crate) struct WidgetContext<'a> {
    matrix: Matrix,
    board_state: &'a BoardState,
    drawing_area: &'a DrawingArea,
}

impl<'a> WidgetContext<'a> {
    fn new(board_state: &'a BoardState, drawing_area: &'a DrawingArea) -> WidgetContext<'a> {
        let matrix = util::compute_matrix(drawing_area, board_state.orientation);

        WidgetContext {
            matrix,
            board_state,
            drawing_area
        }
    }

    fn invert_pos(&self, pos: (f64, f64)) -> (f64, f64) {
        util::invert_pos(self.drawing_area, self.board_state.orientation, pos)
    }

    pub fn matrix(&self) -> Matrix {
        self.matrix
    }

    pub fn queue_draw(&self) {
        self.drawing_area.queue_draw()
    }

    pub fn queue_draw_square(&self, square: Square) {
        util::queue_draw_square(self.drawing_area, self.board_state.orientation, square)
    }

    pub fn queue_draw_rect(&self, x1: f64, y1: f64, x2: f64, y2: f64) {
        util::queue_draw_rect(self.drawing_area, self.board_state.orientation, x1, y1, x2, y2);
    }
}

pub(crate) struct EventContext<'a> {
    widget: WidgetContext<'a>,
    stream: &'a Stream,
    pub pos: (f64, f64),
    pub square: Option<Square>,
}

impl<'a> EventContext<'a> {
    fn new(board_state: &'a BoardState,
           stream: &'a Stream,
           drawing_area: &'a DrawingArea,
           pos: (f64, f64)) -> EventContext<'a>
    {
        let widget = WidgetContext::new(board_state, drawing_area);
        let pos = widget.invert_pos(pos);
        let square = pos_to_square(pos);

        EventContext {
            widget,
            stream,
            pos,
            square,
        }
    }

    pub fn widget(&self) -> &WidgetContext<'a> {
        &self.widget
    }

    pub fn stream(&self) -> &'a Stream {
        self.stream
    }
}

pub(crate) struct BoardState {
    pub(crate) orientation: Color,
    check: Option<Square>,
    last_move: Option<(Square, Square)>,
    pub(crate) piece_set: PieceSet,
    legals: MoveList,
}

impl BoardState {
    fn new() -> Self {
        let pos = Chess::default();
        let mut legals = MoveList::new();
        pos.legal_moves(&mut legals);

        BoardState {
            orientation: Color::White,
            check: None,
            last_move: None,
            piece_set: pieceset::PieceSet::merida(),
            legals,
        }
    }

    pub fn move_targets(&self, orig: Square) -> Bitboard {
        self.legals.iter().filter(|m| m.from() == Some(orig)).map(|m| m.to()).collect()
    }

    pub fn valid_move(&self, orig: Square, dest: Square) -> bool {
        self.move_targets(orig).contains(dest)
    }

    pub fn legal_move(&self, orig: Square, dest: Square, promotion: Option<Role>) -> bool {
        self.legals.iter().any(|m| {
            m.from() == Some(orig) && m.to() == dest && m.promotion() == promotion
        })
    }

    fn draw(&self, cr: &Context) {
        self.draw_border(cr);
        self.draw_board(cr);
        self.draw_last_move(cr);
        self.draw_check(cr);
    }

    fn draw_border(&self, cr: &Context) {
        cr.set_source_rgb(0.2, 0.2, 0.5);
        cr.rectangle(-0.5, -0.5, 9.0, 9.0);
        cr.fill();

        cr.set_font_size(0.20);
        cr.set_source_rgb(0.8, 0.8, 0.8);

        for (rank, glyph) in ["1", "2", "3", "4", "5", "6", "7", "8"].iter().enumerate() {
            self.draw_text(cr, (-0.25, 7.5 - rank as f64), glyph);
            self.draw_text(cr, (8.25, 7.5 - rank as f64), glyph);
        }

        for (file, glyph) in ["a", "b", "c", "d", "e", "f", "g", "h"].iter().enumerate() {
            self.draw_text(cr, (0.5 + file as f64, -0.25), glyph);
            self.draw_text(cr, (0.5 + file as f64, 8.25), glyph);
        }
    }

    fn draw_text(&self, cr: &Context, (x, y): (f64, f64), text: &str) {
        let font = cr.font_extents();
        let e = cr.text_extents(text);

        cr.save();
        cr.translate(x, y);
        cr.rotate(self.orientation.fold(0.0, PI));
        cr.move_to(-0.5 * e.width, 0.5 * font.ascent);
        cr.show_text(text);
        cr.restore();
    }

    fn draw_board(&self, cr: &Context) {
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
    }

    fn draw_last_move(&self, cr: &Context) {
        if let Some((orig, dest)) = self.last_move {
            cr.set_source_rgba(0.61, 0.78, 0.0, 0.41);
            cr.rectangle(orig.file() as f64, 7.0 - orig.rank() as f64, 1.0, 1.0);
            cr.fill();

            if dest != orig {
                cr.rectangle(dest.file() as f64, 7.0 - dest.rank() as f64, 1.0, 1.0);
                cr.fill();
            }
        }
    }

    fn draw_check(&self, cr: &Context) {
        if let Some(check) = self.check {
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
}
