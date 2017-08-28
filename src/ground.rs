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
use cairo::{Context, RadialGradient};
use rsvg::HandleExt;

use time::SteadyTime;

use relm::{Relm, Widget, Update, EventStream};

use util;
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
                    let mut state = state.borrow_mut();
                    state.board_state.now = SteadyTime::now();

                    let animating = state.pieces.is_animating(state.board_state.now) ||
                                    state.promotable.is_animating();

                    let matrix = util::compute_matrix(widget, state.board_state.orientation);
                    cr.set_matrix(matrix);

                    draw_border(cr, &state.board_state);
                    draw_board(cr, &state.board_state, &state.pieces);
                    draw_check(cr, &state.board_state);
                    state.pieces.draw(cr, &state.board_state, &state.promotable);
                    state.drawable.draw(cr);
                    draw_drag(cr, &state.board_state, &state.pieces);
                    state.promotable.draw(cr, &state.board_state);

                    let weak_state = weak_state.clone();
                    let widget = widget.clone();
                    if animating {
                        gtk::idle_add(move || {
                            if let Some(state) = weak_state.upgrade() {
                                let state = state.borrow();
                                state.pieces.queue_animation(&state.board_state, &widget);
                                state.promotable.queue_animation(&state.board_state, &widget);
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

                    let ctx = EventContext {
                        drawing_area: &widget,
                        stream: &stream,
                        pos: e.get_position(),
                        square: util::pos_to_square(widget, state.board_state.orientation, e.get_position()),
                    };

                    button_press_event(&mut state, &ctx, e);
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

                    let ctx = EventContext {
                        drawing_area: widget,
                        stream: &stream,
                        pos: e.get_position(),
                        square: util::pos_to_square(widget, state.board_state.orientation, e.get_position()),
                    };

                    button_release_event(&mut state, &ctx);
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

                    let ctx = EventContext {
                        drawing_area: widget,
                        stream: &stream,
                        pos: e.get_position(),
                        square: util::pos_to_square(widget, state.board_state.orientation, e.get_position()),
                    };

                    motion_notify_event(&mut state, &ctx, e);
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

fn button_release_event(state: &mut State, ctx: &EventContext) {
    state.pieces.drag_mouse_up(&ctx);
    state.drawable.mouse_up(&ctx);
}

fn motion_notify_event(state: &mut State, ctx: &EventContext, e: &EventMotion) {
    state.promotable.mouse_move(&state.board_state, &ctx);
    state.pieces.drag_mouse_move(&state.board_state, &ctx, e);
    state.drawable.mouse_move(&ctx);
}

fn button_press_event(state: &mut State, ctx: &EventContext, e: &EventButton) {
    let promotable = &mut state.promotable;
    let board_state = &mut state.board_state;
    let pieces = &mut state.pieces;

    if let Inhibit(false) = promotable.mouse_down(pieces, &ctx) {
        pieces.selection_mouse_down(&ctx, e);
        pieces.drag_mouse_down(board_state, &ctx, e);
        state.drawable.mouse_down(&ctx, e);
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
}

pub struct EventContext<'a> {
    pub drawing_area: &'a DrawingArea,
    pub stream: &'a EventStream<GroundMsg>,
    pub pos: (f64, f64),
    pub square: Option<Square>,
}

pub(crate) struct BoardState {
    pub(crate) orientation: Color,
    check: Option<Square>,
    last_move: Option<(Square, Square)>,
    pub(crate) piece_set: PieceSet,
    pub(crate) now: SteadyTime,
    pub(crate) legals: MoveList,
}

impl BoardState {
    pub fn move_targets(&self, orig: Square) -> Bitboard {
        self.legals.iter().filter(|m| m.from() == Some(orig)).map(|m| m.to()).collect()
    }

    fn valid_move(&self, orig: Square, dest: Square) -> bool {
        self.move_targets(orig).contains(dest)
    }
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
            now: SteadyTime::now(),
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

fn draw_board(cr: &Context, state: &BoardState, _pieces: &Pieces) {
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

    /* TODO XXX
    if let Some(selected) = state.selected {
        cr.rectangle(selected.file() as f64, 7.0 - selected.rank() as f64, 1.0, 1.0);
        cr.set_source_rgba(0.08, 0.47, 0.11, 0.5);
        cr.fill();

        if let Some(hovered) = pieces.dragging().and_then(|d| util::inverted_to_square(d.pos)) {
            if state.valid_move(selected, hovered) {
                cr.rectangle(hovered.file() as f64, 7.0 - hovered.rank() as f64, 1.0, 1.0);
                cr.set_source_rgba(0.08, 0.47, 0.11, 0.25);
                cr.fill();
            }
        }
    } */

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

fn draw_drag(cr: &Context, state: &BoardState, pieces: &Pieces) {
    if let Some(dragging) = pieces.dragging() {
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
