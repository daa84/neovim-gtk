use cairo;
use ui_model::Color;
use ui::UI;
use shell::{Shell, NvimMode};
use nvim::{RepaintMode, RedrawEvents};
use std::sync::{Arc, Mutex};

use glib;

struct Alpha(f64);

impl Alpha {
    pub fn show(&mut self, step: f64) -> bool {
        self.0 += step;
        if self.0 > 1.0 {
            self.0 = 1.0;
            false
        } else {
            true
        }
    }
    pub fn hide(&mut self, step: f64) -> bool {
        self.0 -= step;
        if self.0 < 0.0 {
            self.0 = 0.0;
            false
        } else {
            true
        }
    }
}

enum AnimPhase {
    Shown,
    Hide,
    Hidden,
    Show,
}

pub struct State {
    alpha: Alpha,
    anim_phase: AnimPhase,

    timer: Option<glib::SourceId>,
}

impl State {
    pub fn new() -> State {
        State {
            alpha: Alpha(1.0),
            anim_phase: AnimPhase::Shown,
            timer: None,
        }
    }
}

pub struct Cursor {
    state: Arc<Mutex<State>>,
}

impl Cursor {
    pub fn new() -> Cursor {

        Cursor {
            state: Arc::new(Mutex::new(State::new())),
        }

    }

    pub fn start(&mut self) {
        let state = self.state.clone();
        let mut mut_state = self.state.lock().unwrap();
        if mut_state.timer.is_none() {
            mut_state.timer = Some(glib::timeout_add(100, move || anim_step(&state)));
        }
    }

    pub fn draw(&self,
                ctx: &cairo::Context,
                shell: &Shell,
                char_width: f64,
                line_height: f64,
                line_y: f64,
                double_width: bool,
                bg: &Color) {

        let current_point = ctx.get_current_point();
        let state = self.state.lock().unwrap();
        ctx.set_source_rgba(1.0 - bg.0, 1.0 - bg.1, 1.0 - bg.2, 0.6 * state.alpha.0);

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

// [TODO]: Reset animation phase on events - 2017-03-24 11:33
fn anim_step(state: &Arc<Mutex<State>>) -> glib::Continue {
    let moved_state = state.clone();
    let mut mut_state = state.lock().unwrap();

    let next_event = match mut_state.anim_phase {
        AnimPhase::Shown => {
            mut_state.anim_phase = AnimPhase::Hide;
            Some(100)
        }
        AnimPhase::Hide => {
            if !mut_state.alpha.hide(0.3) {
                mut_state.anim_phase = AnimPhase::Hidden;

                Some(300)
            } else {
                None
            }
        }
        AnimPhase::Hidden => {
            mut_state.anim_phase = AnimPhase::Show;

            Some(100)
        }
        AnimPhase::Show => {
            if !mut_state.alpha.show(0.3) {
                mut_state.anim_phase = AnimPhase::Shown;

                Some(500)
            } else {
                None
            }
        }
    };

    SHELL!(&shell = {
        let point = shell.model.cur_point();
        shell.on_redraw(&RepaintMode::Area(point));
    });

    
    if let Some(timeout) = next_event {
        mut_state.timer = Some(glib::timeout_add(timeout, move || anim_step(&moved_state) ));

        glib::Continue(false)
    } else {
        glib::Continue(true)
    }

}

impl Drop for Cursor {
    fn drop(&mut self) {
        if let Some(timer_id) = self.state.lock().unwrap().timer {
            glib::source_remove(timer_id);
        }
    }
}
