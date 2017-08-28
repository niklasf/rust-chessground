pub struct Promotable {
    promoting: Option<Promoting>,
}

struct Promoting {
    orig: Square,
    dest: Square,
    hover: Option<Square>,
    time: SteadyTime,
}

impl Promotable {
    pub fn start_promoting(&mut self, orig: Square, dest: Square) {
        self.promoting = Some(Promoting {
            orig,
            dest,
            hover: Some(dest),
            time: SteadyTime::now(),
        });
    }

    pub fn is_animating(&self) -> bool {
        if let Some(ref promoting) = self.promoting {
            false
            //promoting.hover.map_or(false, |h| h.since // todo: elapsed
        } else {
            false
        }
    }

    pub fn queue_animation(&self, drawing_area: &DrawingArea) {
        if let Some(Promoting { hover: Hover { square, .. } }) = self.promoting {
            // queue draw square
        }
    }

    pub fn mouse_move(&mut self, ctx: &EventContext) -> bool {
        self.queue_animation(ctx.drawing_area);

        let consume = if let Some(ref mut promoting) = self.promoting {
            let current = promoting.hover.map(|h| h.square);
            if current != ctx.square {
            }
            // todo: still hovering?
            true
        } else {
            false
        };

        self.queue_animation(ctx.drawing_area);
        consume
    }

    pub fn draw(&self, cr: &Context) {
    }
}
