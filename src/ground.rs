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

use std::rc::{Rc, Weak};
use std::cell::RefCell;
use std::f64::consts::PI;
use std::cmp::{min, max};
use std::fmt;

use gtk::prelude::*;
use gtk::DrawingArea;
use gdk::{EventButton, EventMotion, EventMask};
use cairo::{Context, Matrix};

use relm::{Relm, Widget, Update, StreamHandle};

use shakmaty::{Square, Rank, Color, Role, Board, Move, MoveList, Chess, Position};

use util::{file_to_float, pos_to_square, rank_to_float};
use pieces::Pieces;
use drawable::{Drawable, DrawShape};
use promotable::Promotable;
use boardstate::BoardState;

type Stream = StreamHandle<GroundMsg>;

pub struct Model {
    state: Rc<RefCell<State>>,
}

impl fmt::Debug for Model {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Model").finish()
    }
}

/// Chessground events and messages.
#[derive(Debug, Msg)]
pub enum GroundMsg {
    /// Flip the board.
    Flip,
    /// Set the board orientation.
    SetOrientation(Color),
    /// Set up a position configuration.
    SetPos(Pos),
    /// Set up a board.
    SetBoard(Board),

    /// Sent when the completed a piece drag or move.
    UserMove(Square, Square, Option<Role>),
    /// Sent when shapes are added, removed or cleared.
    ShapesChanged(Vec<DrawShape>),
}

/// A position configuration.
///
/// * Piece positions
/// * Legal move hints
/// * Check hint
/// * Last move hint
/// * Side to move
#[derive(Debug, Clone)]
pub struct Pos {
    board: Board,
    legals: Box<MoveList>,
    check: Option<Square>,
    last_move: Option<(Square, Square)>,
    turn: Option<Color>,
}

impl Pos {
    /// Create a new position configuration.
    pub fn new<P: Position>(p: &P) -> Pos {
        Pos {
            board: p.board().clone(),
            legals: Box::new(p.legal_moves()),
            check: if p.checkers().any() { p.board().king_of(p.turn()) } else { None },
            last_move: None,
            turn: Some(p.turn()),
        }
    }

    /// Create a position configuration from a board, without any other hints.
    pub fn from_board(board: Board) -> Pos {
        Pos {
            board,
            legals: Box::new(MoveList::new()),
            check: None,
            last_move: None,
            turn: None,
        }
    }

    /// Set the hint for the last move, so that it can be highlighted on
    /// the board.
    pub fn set_last_move(&mut self, m: Option<&Move>) {
        self.last_move = m.map(|m| (m.from().unwrap_or_else(|| m.to()), m.to()))
    }

    pub fn with_last_move(mut self, m: &Move) -> Self {
        self.set_last_move(Some(m));
        self
    }

    /// Set the check hint.
    pub fn set_check(&mut self, king: Option<Square>) {
        self.check = king;
    }

    pub fn with_check(mut self, king: Square) -> Pos {
        self.check = Some(king);
        self
    }

    /// Set the legal move hints.
    pub fn set_legals(&mut self, legals: MoveList) {
        self.legals = Box::new(legals);
    }

    pub fn with_legals(mut self, legals: MoveList) -> Pos {
        self.legals = Box::new(legals);
        self
    }

    /// Set the side to move.
    pub fn set_turn(&mut self, turn: Option<Color>) {
        self.turn = turn;
    }

    pub fn with_turn(mut self, turn: Color) -> Pos {
        self.turn = Some(turn);
        self
    }
}

impl Default for Pos {
    fn default() -> Pos {
        Pos::new(&Chess::default())
    }
}

/// Chessground, a chess board widget.
#[derive(Debug)]
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
            GroundMsg::Flip => {
                let orientation = state.board_state.orientation();
                state.board_state.set_orientation(!orientation);
                self.drawing_area.queue_draw();
            },
            GroundMsg::SetOrientation(orientation) => {
                state.board_state.set_orientation(orientation);
                self.drawing_area.queue_draw();
            },
            GroundMsg::SetPos(pos) => {
                state.pieces.set_board(&pos.board);
                state.promotable.update(&pos.legals);
                state.board_state.set_check(pos.check);
                state.board_state.set_last_move(pos.last_move);
                state.board_state.set_turn(pos.turn);
                *state.board_state.legals_mut() = *pos.legals;
                self.drawing_area.queue_draw();
            },
            GroundMsg::SetBoard(board) => {
                state.pieces.set_board(&board);
                state.board_state.set_check(None);
                state.board_state.set_last_move(None);
                state.board_state.set_turn(None);
                state.board_state.legals_mut().clear();
                state.promotable.cancel();
                self.drawing_area.queue_draw();
            },
            GroundMsg::UserMove(orig, dest, None) if state.board_state.valid_move(orig, dest) => {
                if state.board_state.legals().iter().any(|m| m.from() == Some(orig) && m.to() == dest && m.promotion().is_some()) {
                    let color = state.pieces.figurine_at(orig).map_or_else(|| {
                        Color::from_white(dest.rank() > Rank::Fourth)
                    }, |figurine| figurine.piece().color);
                    state.promotable.start(color, orig, dest);
                    self.drawing_area.queue_draw();
                }
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

        drawing_area.add_events(EventMask::BUTTON_PRESS_MASK |
                                EventMask::BUTTON_RELEASE_MASK |
                                EventMask::POINTER_MOTION_MASK |
                                EventMask::SCROLL_MASK);

        {
            // draw
            let weak_state = Rc::downgrade(&model.state);
            drawing_area.connect_draw(move |widget, cr| {
                if let Some(state) = weak_state.upgrade() {
                    let state = state.borrow();
                    state.draw(widget, cr).unwrap();

                    // queue next draw for animation
                    let weak_state = Weak::clone(&weak_state);
                    let widget = widget.clone();
                    cairo::glib::idle_add_local(move || {
                        if let Some(state) = weak_state.upgrade() {
                            state.borrow_mut().queue_animation(&widget);
                        }
                        Continue(false)
                    });
                }
                Inhibit(false)
            });
        }

        {
            // mouse down
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
            // mouse up
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
            // mouse move
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

    fn queue_animation(&mut self, drawing_area: &DrawingArea) {
        let ctx = WidgetContext::new(&self.board_state, drawing_area);
        self.pieces.queue_animation(&ctx);
        self.promotable.queue_animation(&ctx);
    }

    fn draw(&self, drawing_area: &DrawingArea, cr: &Context) -> Result<(), cairo::Error> {
        let ctx = WidgetContext::new(&self.board_state, drawing_area);
        cr.set_matrix(ctx.matrix());

        // draw
        self.board_state.draw(cr)?;
        self.pieces.draw(cr, &self.board_state, &self.promotable)?;
        self.drawable.draw(cr)?;
        self.pieces.draw_drag(cr, &self.board_state)?;
        self.promotable.draw(cr, &self.board_state)?;

        Ok(())
    }

    fn button_release_event(&mut self, stream: &Stream, drawing_area: &DrawingArea, e: &EventButton) {
        let ctx = EventContext::new(&self.board_state, stream, drawing_area, e.position());
        self.pieces.drag_mouse_up(&ctx);
        self.drawable.mouse_up(&ctx);
    }

    fn motion_notify_event(&mut self, stream: &Stream, drawing_area: &DrawingArea, e: &EventMotion) {
        let ctx = EventContext::new(&self.board_state, stream, drawing_area, e.position());
        self.promotable.mouse_move(&ctx);
        self.pieces.drag_mouse_move(&ctx);
        self.drawable.mouse_move(&ctx);
    }

    fn button_press_event(&mut self, stream: &Stream, drawing_area: &DrawingArea, e: &EventButton) {
        let ctx = EventContext::new(&self.board_state, stream, drawing_area, e.position());
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
    drawing_area: &'a DrawingArea,
}

impl<'a> WidgetContext<'a> {
    fn new(board_state: &'a BoardState, drawing_area: &'a DrawingArea) -> WidgetContext<'a>
    {
        let alloc = drawing_area.allocation();
        let size = max(min(alloc.width, alloc.height), 9);

        let mut matrix = Matrix::identity();
        matrix.translate(f64::from(alloc.x), f64::from(alloc.y));

        matrix.translate(f64::from(alloc.width) / 2.0, f64::from(alloc.height) / 2.0);
        matrix.scale(f64::from(size) / 9.0, f64::from(size) / 9.0);
        matrix.rotate(board_state.orientation().fold_wb(0.0, PI));
        matrix.translate(-4.0, -4.0);

        WidgetContext { matrix, drawing_area }
    }

    fn invert_pos(&self, (x, y): (f64, f64)) -> (f64, f64) {
        self.matrix()
            .try_invert().expect("transform invertible")
            .transform_point(x, y)
    }

    pub fn matrix(&self) -> Matrix {
        self.matrix
    }

    pub fn queue_draw(&self) {
        self.drawing_area.queue_draw()
    }

    pub fn queue_draw_square(&self, square: Square) {
        self.queue_draw_rect(file_to_float(square.file()), 7.0 - rank_to_float(square.rank()), 1.0, 1.0);
    }

    pub fn queue_draw_rect(&self, x: f64, y: f64, width: f64, height: f64) {
        // round to square grid
        let (rx, ry) = (x.floor(), y.floor());

        // transform to widget coordinates
        let matrix = self.matrix();
        let (x1, y1) = matrix.transform_point(rx, ry);
        let (x2, y2) = matrix.transform_point((x + width).ceil(), (y + height).ceil());

        let xmin = min(x1.floor() as i32, x2.floor() as i32);
        let ymin = min(y1.floor() as i32, y2.floor() as i32);
        let xmax = max(x1.ceil() as i32, x2.ceil() as i32);
        let ymax = max(y1.ceil() as i32, y2.ceil() as i32);

        let alloc = self.drawing_area.allocation();
        self.drawing_area.queue_draw_area(xmin - alloc.x, ymin - alloc.y, xmax - xmin, ymax - ymin);
    }
}

pub(crate) struct EventContext<'a> {
    widget: WidgetContext<'a>,
    stream: &'a Stream,
    pos: (f64, f64),
    square: Option<Square>,
}

impl<'a> EventContext<'a> {
    fn new(board_state: &'a BoardState,
           stream: &'a Stream,
           drawing_area: &'a DrawingArea,
           pos: (f64, f64)) -> EventContext<'a>
    {
        let widget = WidgetContext::new(board_state, drawing_area);
        let alloc = drawing_area.allocation();
        let pos = (pos.0 + f64::from(alloc.x), pos.1 + f64::from(alloc.y));
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

    pub fn pos(&self) -> (f64, f64) {
        self.pos
    }

    pub fn square(&self) -> Option<Square> {
        self.square
    }
}
