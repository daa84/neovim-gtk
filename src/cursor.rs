use cairo;
use crate::mode;
use crate::render;
use crate::render::CellMetrics;
use crate::highlight::HighlightMap;
use std::sync::{Arc, Weak};
use crate::ui::UiMutex;

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

struct State<CB: CursorRedrawCb> {
    alpha: Alpha,
    anim_phase: AnimPhase,
    redraw_cb: Weak<UiMutex<CB>>,

    timer: Option<glib::SourceId>,
}

impl<CB: CursorRedrawCb> State<CB> {
    fn new(redraw_cb: Weak<UiMutex<CB>>) -> Self {
        State {
            alpha: Alpha(1.0),
            anim_phase: AnimPhase::Shown,
            redraw_cb,
            timer: None,
        }
    }

    fn reset_to(&mut self, phase: AnimPhase) {
        self.alpha = Alpha(1.0);
        self.anim_phase = phase;
        if let Some(timer_id) = self.timer.take() {
            glib::source_remove(timer_id);
        }
    }
}

pub trait Cursor {
    /// return cursor current alpha value
    fn draw(
        &self,
        ctx: &cairo::Context,
        font_ctx: &render::Context,
        line_y: f64,
        double_width: bool,
        hl: &HighlightMap,
    ) -> f64;

    fn is_visible(&self) -> bool;

    fn mode_info(&self) -> Option<&mode::ModeInfo>;
}

pub struct EmptyCursor;

impl EmptyCursor {
    pub fn new() -> Self {
        EmptyCursor {}
    }
}

impl Cursor for EmptyCursor {
    fn draw(
        &self,
        _ctx: &cairo::Context,
        _font_ctx: &render::Context,
        _line_y: f64,
        _double_width: bool,
        _color: &HighlightMap,
    ) -> f64 {
        0.0
    }

    fn is_visible(&self) -> bool {
        false
    }

    fn mode_info(&self) -> Option<&mode::ModeInfo> {
        None
    }
}

pub struct BlinkCursor<CB: CursorRedrawCb> {
    state: Arc<UiMutex<State<CB>>>,
    mode_info: Option<mode::ModeInfo>,
}

impl<CB: CursorRedrawCb + 'static> BlinkCursor<CB> {
    pub fn new(redraw_cb: Weak<UiMutex<CB>>) -> Self {
        BlinkCursor {
            state: Arc::new(UiMutex::new(State::new(redraw_cb))),
            mode_info: None,
        }
    }

    pub fn set_mode_info(&mut self, mode_info: Option<mode::ModeInfo>) {
        self.mode_info = mode_info;
    }

    pub fn start(&mut self) {
        let blinkwait = self
            .mode_info
            .as_ref()
            .and_then(|mi| mi.blinkwait)
            .unwrap_or(500);

        let state = self.state.clone();
        let mut mut_state = self.state.borrow_mut();
        mut_state.reset_to(AnimPhase::Shown);
        mut_state.timer = Some(glib::timeout_add(
            if blinkwait > 0 { blinkwait } else { 500 },
            move || anim_step(&state),
        ));
    }

    pub fn reset_state(&mut self) {
        if self.state.borrow().anim_phase != AnimPhase::Busy {
            self.start();
        }
    }

    pub fn enter_focus(&mut self) {
        if self.state.borrow().anim_phase != AnimPhase::Busy {
            self.start();
        }
    }

    pub fn leave_focus(&mut self) {
        if self.state.borrow().anim_phase != AnimPhase::Busy {
            self.state.borrow_mut().reset_to(AnimPhase::NoFocus);
        }
    }

    pub fn busy_on(&mut self) {
        self.state.borrow_mut().reset_to(AnimPhase::Busy);
    }

    pub fn busy_off(&mut self) {
        self.start();
    }
}

impl<CB: CursorRedrawCb> Cursor for BlinkCursor<CB> {
    fn draw(
        &self,
        ctx: &cairo::Context,
        font_ctx: &render::Context,
        line_y: f64,
        double_width: bool,
        hl: &HighlightMap,
    ) -> f64 {
        let state = self.state.borrow();

        let current_point = ctx.get_current_point();

        let bg = hl.cursor_bg();
        ctx.set_source_rgba(bg.0, bg.1, bg.2, state.alpha.0);

        let (y, width, height) = cursor_rect(
            self.mode_info(),
            font_ctx.cell_metrics(),
            line_y,
            double_width,
        );

        ctx.rectangle(current_point.0, y, width, height);
        if state.anim_phase == AnimPhase::NoFocus {
            ctx.stroke();
        } else {
            ctx.fill();
        }

        state.alpha.0
    }

    fn is_visible(&self) -> bool {
        let state = self.state.borrow();

        if state.anim_phase == AnimPhase::Busy {
            return false;
        }

        if state.alpha.0 < 0.000001 {
            false
        } else {
            true
        }
    }

    fn mode_info(&self) -> Option<&mode::ModeInfo> {
        self.mode_info.as_ref()
    }
}

pub fn cursor_rect(
    mode_info: Option<&mode::ModeInfo>,
    cell_metrics: &CellMetrics,
    line_y: f64,
    double_width: bool,
) -> (f64, f64, f64) {
    let &CellMetrics {
        line_height,
        char_width,
        ..
    } = cell_metrics;

    if let Some(mode_info) = mode_info {
        match mode_info.cursor_shape() {
            None | Some(&mode::CursorShape::Unknown) | Some(&mode::CursorShape::Block) => {
                let cursor_width = if double_width {
                    char_width * 2.0
                } else {
                    char_width
                };
                (line_y, cursor_width, line_height)
            }
            Some(&mode::CursorShape::Vertical) => {
                let cell_percentage = mode_info.cell_percentage();
                let cursor_width = if cell_percentage > 0 {
                    (char_width * cell_percentage as f64) / 100.0
                } else {
                    char_width
                };
                (line_y, cursor_width, line_height)
            }
            Some(&mode::CursorShape::Horizontal) => {
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
        let cursor_width = if double_width {
            char_width * 2.0
        } else {
            char_width
        };

        (line_y, cursor_width, line_height)
    }
}

fn anim_step<CB: CursorRedrawCb + 'static>(state: &Arc<UiMutex<State<CB>>>) -> glib::Continue {
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

    let redraw_cb = mut_state.redraw_cb.upgrade().unwrap();
    let mut redraw_cb = redraw_cb.borrow_mut();
    redraw_cb.queue_redraw_cursor();

    if let Some(timeout) = next_event {
        let moved_state = state.clone();
        mut_state.timer = Some(glib::timeout_add(timeout, move || anim_step(&moved_state)));

        glib::Continue(false)
    } else {
        glib::Continue(true)
    }
}

impl<CB: CursorRedrawCb> Drop for BlinkCursor<CB> {
    fn drop(&mut self) {
        if let Some(timer_id) = self.state.borrow_mut().timer.take() {
            glib::source_remove(timer_id);
        }
    }
}

pub trait CursorRedrawCb {
    fn queue_redraw_cursor(&mut self);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_cursor_rect_horizontal() {
        let mut mode_data = HashMap::new();
        mode_data.insert("cursor_shape".to_owned(), From::from("horizontal"));
        mode_data.insert("cell_percentage".to_owned(), From::from(25));

        let mode_info = mode::ModeInfo::new(&mode_data).ok();
        let char_width = 50.0;
        let line_height = 30.0;
        let line_y = 0.0;

        let (y, width, height) = cursor_rect(
            mode_info.as_ref(),
            &CellMetrics::new_hw(line_height, char_width),
            line_y,
            false,
        );
        assert_eq!(line_y + line_height - line_height / 4.0, y);
        assert_eq!(char_width, width);
        assert_eq!(line_height / 4.0, height);
    }

    #[test]
    fn test_cursor_rect_horizontal_doublewidth() {
        let mut mode_data = HashMap::new();
        mode_data.insert("cursor_shape".to_owned(), From::from("horizontal"));
        mode_data.insert("cell_percentage".to_owned(), From::from(25));

        let mode_info = mode::ModeInfo::new(&mode_data).ok();
        let char_width = 50.0;
        let line_height = 30.0;
        let line_y = 0.0;

        let (y, width, height) = cursor_rect(
            mode_info.as_ref(),
            &CellMetrics::new_hw(line_height, char_width),
            line_y,
            true,
        );
        assert_eq!(line_y + line_height - line_height / 4.0, y);
        assert_eq!(char_width * 2.0, width);
        assert_eq!(line_height / 4.0, height);
    }

    #[test]
    fn test_cursor_rect_vertical() {
        let mut mode_data = HashMap::new();
        mode_data.insert("cursor_shape".to_owned(), From::from("vertical"));
        mode_data.insert("cell_percentage".to_owned(), From::from(25));

        let mode_info = mode::ModeInfo::new(&mode_data).ok();
        let char_width = 50.0;
        let line_height = 30.0;
        let line_y = 0.0;

        let (y, width, height) = cursor_rect(
            mode_info.as_ref(),
            &CellMetrics::new_hw(line_height, char_width),
            line_y,
            false,
        );
        assert_eq!(line_y, y);
        assert_eq!(char_width / 4.0, width);
        assert_eq!(line_height, height);
    }
}
