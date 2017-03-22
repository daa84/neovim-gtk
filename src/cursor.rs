use cairo;
use ui_model::Color;
use shell::{Shell, NvimMode};

use glib;

// display, 2 sec -> hiding 1 sec -> not visible 1 sec -> showing 1 sec
pub struct Cursor {
}

impl Cursor {
    pub fn new() -> Cursor {
        Cursor {  }
    }

    pub fn draw(&self, ctx: &cairo::Context, shell: &Shell, 
                char_width: f64, line_height: f64, line_y: f64, double_width: bool, bg: &Color) {
        let current_point = ctx.get_current_point();
        ctx.set_source_rgba(1.0 - bg.0, 1.0 - bg.1, 1.0 - bg.2, 0.5);

        let cursor_width = if shell.mode == NvimMode::Insert {
            char_width / 5.0
        } else {
            if double_width {
                char_width * 2.0
            } else {
                char_width
            }
        };

        ctx.rectangle(current_point.0, line_y, cursor_width, line_height);
        ctx.fill();
    }
}


pub struct Animation {
    state_stream: Vec<Box<AnimationState>>,
    state: Option<Box<AnimationState>>,
    timer: Option<glib::SourceId>,
}

impl Animation {
    pub fn new() -> Animation {
        Animation { 
            state_stream: vec![],
            state: None,
            timer: None,
        }
    }
}

trait AnimationState {
    fn clone(&self) -> AnimationState;

    // [TODO]: Description - repaint rect here
    fn next(&mut self) -> Option<u32>;

    fn paint(&self, ctx: &cairo::Context, shell: &Shell);
}
