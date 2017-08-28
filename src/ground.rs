use std::rc::Rc;
use std::cell::RefCell;
use std::f64::consts::PI;
use std::cmp::{min, max};

use shakmaty::{Square, Role, Board, MoveList};

use gtk;
use gtk::prelude::*;
use gtk::DrawingArea;
use gdk;
use gdk::{EventButton, EventMotion};
use cairo::prelude::*;
use cairo::{Context, Matrix};

use relm::{Relm, Widget, Update, EventStream};

use util::pos_to_square;
use pieces::Pieces;
use drawable::Drawable;
use promotable::Promotable;
use board_state::BoardState;

type Stream = EventStream<GroundMsg>;

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
            // draw
            let weak_state = Rc::downgrade(&model.state);
            drawing_area.connect_draw(move |widget, cr| {
                if let Some(state) = weak_state.upgrade() {
                    let state = state.borrow();
                    let animating = state.is_animating();
                    state.draw(widget, cr);

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

    fn is_animating(&self) -> bool {
        self.promotable.is_animating() || self.pieces.is_animating()
    }

    fn queue_animation(&self, drawing_area: &DrawingArea) {
        let ctx = WidgetContext::new(&self.board_state, drawing_area);
        self.pieces.queue_animation(&ctx);
        self.promotable.queue_animation(&ctx);
    }

    fn draw(&self, drawing_area: &DrawingArea, cr: &Context) {
        let ctx = WidgetContext::new(&self.board_state, drawing_area);
        cr.set_matrix(ctx.matrix());

        // draw
        self.board_state.draw(cr);
        self.pieces.draw(cr, &self.board_state, &self.promotable);
        self.drawable.draw(cr);
        self.pieces.draw_drag(cr, &self.board_state);
        self.promotable.draw(cr, &self.board_state);
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
    drawing_area: &'a DrawingArea,
}

impl<'a> WidgetContext<'a> {
    fn new(board_state: &'a BoardState, drawing_area: &'a DrawingArea) -> WidgetContext<'a> {
        let w = drawing_area.get_allocated_width();
        let h = drawing_area.get_allocated_height();
        let size = max(min(w, h), 9);

        let mut matrix = Matrix::identity();

        matrix.translate(w as f64 / 2.0, h as f64 / 2.0);
        matrix.scale(size as f64 / 9.0, size as f64 / 9.0);
        matrix.rotate(board_state.orientation.fold(0.0, PI));
        matrix.translate(-4.0, -4.0);

        WidgetContext {
            matrix,
            drawing_area
        }
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
        self.queue_draw_rect(square.file() as f64, 7.0 - square.rank() as f64, 1.0, 1.0);
    }

    pub fn queue_draw_rect(&self, x: f64, y: f64, width: f64, height: f64) {
        let matrix = self.matrix();
        let (x1, y1) = matrix.transform_point(x, y);
        let (x2, y2) = matrix.transform_point(x + width, y + height);

        let xmin = min(x1.floor() as i32, x2.floor() as i32);
        let ymin = min(y1.floor() as i32, y2.floor() as i32);
        let xmax = max(x1.ceil() as i32, x2.ceil() as i32);
        let ymax = max(y1.ceil() as i32, y2.ceil() as i32);

        self.drawing_area.queue_draw_area(xmin, ymin, xmax - xmin, ymax - ymin);
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
