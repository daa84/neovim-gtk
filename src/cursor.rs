use cairo;
use color::Color;
use ui::UiMutex;
use shell;
use mode;
use nvim;
use nvim::{RepaintMode, RedrawEvents};
use std::sync::{Arc, Weak};

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

#[derive(PartialEq)]
enum AnimPhase {
    Shown,
    Hide,
    Hidden,
    Show,
    NoFocus,
    Busy,
}

struct State {
    alpha: Alpha,
    anim_phase: AnimPhase,
    shell: Weak<UiMutex<shell::State>>,

    timer: Option<glib::SourceId>,
}

impl State {
    fn new(shell: Weak<UiMutex<shell::State>>) -> State {
        State {
            alpha: Alpha(1.0),
            anim_phase: AnimPhase::Shown,
            shell: shell,
            timer: None,
        }
    }

    fn reset_to(&mut self, phase: AnimPhase) {
        self.alpha = Alpha(1.0);
        self.anim_phase = phase;
        if let Some(timer_id) = self.timer {
            glib::source_remove(timer_id);
            self.timer = None;
        }
    }
}

pub struct Cursor {
    state: Arc<UiMutex<State>>,
}

impl Cursor {
    pub fn new(shell: Weak<UiMutex<shell::State>>) -> Cursor {
        Cursor { state: Arc::new(UiMutex::new(State::new(shell))) }
    }

    pub fn start(&mut self) {
        let state = self.state.clone();
        let mut mut_state = self.state.borrow_mut();
        mut_state.reset_to(AnimPhase::Shown);
        mut_state.timer = Some(glib::timeout_add(500, move || anim_step(&state)));
    }

    pub fn reset_state(&mut self) {
        self.start();
    }

    pub fn enter_focus(&mut self) {
        self.start();
    }

    pub fn leave_focus(&mut self) {
        self.state.borrow_mut().reset_to(AnimPhase::NoFocus);
    }

    pub fn busy_on(&mut self) {
        self.state.borrow_mut().reset_to(AnimPhase::Busy);
    }

    pub fn busy_off(&mut self) {
        self.start();
    }

    pub fn draw(&self,
                ctx: &cairo::Context,
                shell: &shell::State,
                char_width: f64,
                line_height: f64,
                line_y: f64,
                double_width: bool,
                bg: &Color) {

        let state = self.state.borrow();

        if state.anim_phase == AnimPhase::Busy {
            return;
        }

        let current_point = ctx.get_current_point();
        ctx.set_source_rgba(1.0 - bg.0, 1.0 - bg.1, 1.0 - bg.2, 0.6 * state.alpha.0);

        let (y, width, height) =
            cursor_rect(&shell.mode, char_width, line_height, line_y, double_width);

        ctx.rectangle(current_point.0, y, width, height);
        if state.anim_phase == AnimPhase::NoFocus {
            ctx.stroke();
        } else {
            ctx.fill();
        }
    }
}

fn cursor_rect(mode: &mode::Mode,
               char_width: f64,
               line_height: f64,
               line_y: f64,
               double_width: bool)
               -> (f64, f64, f64) {
    if let Some(mode_info) = mode.mode_info() {
        match mode_info.cursor_shape() {
            None |
            Some(&nvim::CursorShape::Unknown) |
            Some(&nvim::CursorShape::Block) => {
                let cursor_width = if double_width {
                    char_width * 2.0
                } else {
                    char_width
                };
                (line_y, cursor_width, line_height)
            }
            Some(&nvim::CursorShape::Vertical) => {
                let cell_percentage = mode_info.cell_percentage();
                let cursor_width = if cell_percentage > 0 {
                    (char_width * cell_percentage as f64) / 100.0
                } else {
                    char_width
                };
                (line_y, cursor_width, line_height)
            }
            Some(&nvim::CursorShape::Horizontal) => {
                let cell_percentage = mode_info.cell_percentage();
                let cursor_width = if double_width {
                    char_width * 2.0
                } else {
                    char_width
                };

                if cell_percentage > 0 {
                    let height = (line_height * cell_percentage as f64) / 100.0;
                    (line_y + line_height - height, cursor_width, height)
                } else {
                    (line_y, cursor_width, line_height)
                }
            }
        }
    } else {
        let cursor_width = if mode.is(&mode::NvimMode::Insert) {
            char_width / 5.0
        } else if double_width {
            char_width * 2.0
        } else {
            char_width
        };

        (line_y, cursor_width, line_height)
    }
}
fn anim_step(state: &Arc<UiMutex<State>>) -> glib::Continue {
    let mut mut_state = state.borrow_mut();

    let next_event = match mut_state.anim_phase {
        AnimPhase::Shown => {
            mut_state.anim_phase = AnimPhase::Hide;
            Some(60)
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

            Some(60)
        }
        AnimPhase::Show => {
            if !mut_state.alpha.show(0.3) {
                mut_state.anim_phase = AnimPhase::Shown;

                Some(500)
            } else {
                None
            }
        }
        AnimPhase::NoFocus => None, 
        AnimPhase::Busy => None,
    };

    let shell = mut_state.shell.upgrade().unwrap();
    let shell = shell.borrow();
    let point = shell.model.cur_point();
    shell.on_redraw(&RepaintMode::Area(point));


    if let Some(timeout) = next_event {
        let moved_state = state.clone();
        mut_state.timer = Some(glib::timeout_add(timeout, move || anim_step(&moved_state)));

        glib::Continue(false)
    } else {
        glib::Continue(true)
    }

}

impl Drop for Cursor {
    fn drop(&mut self) {
        if let Some(timer_id) = self.state.borrow().timer {
            glib::source_remove(timer_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_rect_horizontal() {
        let mut mode = mode::Mode::new();
        let mode_info = nvim::ModeInfo::new(&vec![(From::from("cursor_shape"),
                                                   From::from("horizontal")),
                                                  (From::from("cell_percentage"), From::from(25))]);
        mode.update("insert", 0);
        mode.set_info(true, vec![mode_info.unwrap()]);
        let char_width = 50.0;
        let line_height = 30.0;
        let line_y = 0.0;

        let (y, width, height) = cursor_rect(&mode, char_width, line_height, line_y, false);
        assert_eq!(line_y + line_height - line_height / 4.0, y);
        assert_eq!(char_width, width);
        assert_eq!(line_height / 4.0, height);
    }

    #[test]
    fn test_cursor_rect_horizontal_doublewidth() {
        let mut mode = mode::Mode::new();
        let mode_info = nvim::ModeInfo::new(&vec![(From::from("cursor_shape"),
                                                   From::from("horizontal")),
                                                  (From::from("cell_percentage"), From::from(25))]);
        mode.update("insert", 0);
        mode.set_info(true, vec![mode_info.unwrap()]);
        let char_width = 50.0;
        let line_height = 30.0;
        let line_y = 0.0;

        let (y, width, height) = cursor_rect(&mode, char_width, line_height, line_y, true);
        assert_eq!(line_y + line_height - line_height / 4.0, y);
        assert_eq!(char_width * 2.0, width);
        assert_eq!(line_height / 4.0, height);
    }

    #[test]
    fn test_cursor_rect_vertical() {
        let mut mode = mode::Mode::new();
        let mode_info = nvim::ModeInfo::new(&vec![(From::from("cursor_shape"),
                                                   From::from("vertical")),
                                                  (From::from("cell_percentage"), From::from(25))]);
        mode.update("insert", 0);
        mode.set_info(true, vec![mode_info.unwrap()]);
        let char_width = 50.0;
        let line_height = 30.0;
        let line_y = 0.0;

        let (y, width, height) = cursor_rect(&mode, char_width, line_height, line_y, false);
        assert_eq!(line_y, y);
        assert_eq!(char_width / 4.0, width);
        assert_eq!(line_height, height);
    }
}
