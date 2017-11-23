use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::cell::RefCell;

use gtk;
use gtk::prelude::*;
use cairo;

use neovim_lib::Value;

use ui_model::{UiModel, Attrs};
use ui::UiMutex;
use render;
use shell;
use cursor;

pub struct Level {
    model: UiModel,
}

impl Level {
    const COLUMNS_STEP: u64 = 50;
    const ROWS_STEP: u64 = 10;

    pub fn from(
        content: Vec<(HashMap<String, Value>, String)>,
        pos: u64,
        firstc: String,
        prompt: String,
        indent: u64,
    ) -> Self {
        //TODO: double width chars
        //TODO: im

        let prompt = prompt_lines(firstc, prompt, indent);
        let content: Vec<(Attrs, Vec<char>)> = content
            .iter()
            .map(|c| (Attrs::from_value_map(&c.0), c.1.chars().collect()))
            .collect();

        let width = (content.iter().map(|c| c.1.len()).count() +
                         prompt.last().map_or(0, |p| p.len())) as u64;
        let columns = ((width / Level::COLUMNS_STEP) + 1) * Level::COLUMNS_STEP;
        let rows = ((prompt.len() as u64 / Level::ROWS_STEP) + 1) * Level::ROWS_STEP;

        let mut model = UiModel::new(rows, columns);

        for (row_idx, prompt_line) in prompt.iter().enumerate() {
            for (col_idx, &ch) in prompt_line.iter().enumerate() {
                model.set_cursor(row_idx, col_idx);
                model.put(ch, false, None);
            }
        }

        let mut col_idx = 0;
        let row_idx = if prompt.len() > 0 {
            prompt.len() - 1
        } else {
            0
        };
        for (attr, ch_list) in content {
            for ch in ch_list {
                model.set_cursor(row_idx, col_idx);
                model.put(ch, false, Some(&attr));
                col_idx += 1;
            }
        }

        Level { model }
    }

    fn update_cache(&mut self, render_state: &shell::RenderState) {
        render::shape_dirty(
            &render_state.font_ctx,
            &mut self.model,
            &render_state.color_model,
        );
    }
}

fn prompt_lines(firstc: String, prompt: String, indent: u64) -> Vec<Vec<char>> {
    if !firstc.is_empty() {
        vec![firstc.chars().chain((0..indent).map(|_| ' ')).collect()]
    } else if !prompt.is_empty() {
        prompt.lines().map(|l| l.chars().collect()).collect()
    } else {
        vec![]
    }
}

struct State {
    levels: Vec<Level>,
    render_state: Rc<RefCell<shell::RenderState>>,
    drawing_area: gtk::DrawingArea,
}

impl State {
    fn new(drawing_area: gtk::DrawingArea, render_state: Rc<RefCell<shell::RenderState>>) -> Self {
        State {
            levels: Vec::new(),
            render_state,
            drawing_area,
        }
    }
}

impl cursor::CursorRedrawCb for State {
    fn queue_redraw_cursor(&mut self) {
        // TODO: implement
    }
}

pub struct CmdLine {
    popover: gtk::Popover,
    displyed: bool,
    state: Arc<UiMutex<State>>,
}

impl CmdLine {
    pub fn new(drawing: &gtk::DrawingArea, render_state: Rc<RefCell<shell::RenderState>>) -> Self {
        let popover = gtk::Popover::new(Some(drawing));
        popover.set_modal(false);
        let edit_frame = gtk::Frame::new(None);
        edit_frame.set_shadow_type(gtk::ShadowType::In);
        let drawing_area = gtk::DrawingArea::new();
        drawing_area.set_size_request(50, 50);
        edit_frame.add(&drawing_area);
        edit_frame.show_all();

        popover.add(&edit_frame);

        let state = Arc::new(UiMutex::new(State::new(drawing_area.clone(), render_state)));
        let weak_cb = Arc::downgrade(&state);
        let cursor = cursor::Cursor::new(weak_cb);

        drawing_area.connect_draw(
            clone!(state => move |_, ctx| gtk_draw(ctx, &state, &cursor)),
        );

        CmdLine {
            popover,
            state,
            displyed: false,
        }
    }

    pub fn show_level(
        &mut self,
        content: Vec<(HashMap<String, Value>, String)>,
        pos: u64,
        firstc: String,
        prompt: String,
        indent: u64,
        level_idx: u64,
    ) {
        let mut state = self.state.borrow_mut();

        let mut level = Level::from(content, pos, firstc, prompt, indent);
        level.update_cache(&*state.render_state.borrow());

        if level_idx as usize == state.levels.len() {
            // TODO: update level
            state.levels.pop();
        }
        state.levels.push(level);
        if !self.displyed {
            self.displyed = true;
            self.popover.popup();
        } else {
            state.drawing_area.queue_draw()
        }
    }
}

fn gtk_draw(
    ctx: &cairo::Context,
    state: &Arc<UiMutex<State>>,
    cursor: &cursor::Cursor<State>,
) -> Inhibit {
    let state = state.borrow();
    let level = state.levels.last();

    if let Some(level) = level {
        let render_state = state.render_state.borrow();

        render::render(
            ctx,
            cursor,
            &render_state.font_ctx,
            &level.model,
            &render_state.color_model,
            &render_state.mode,
        );
    }
    Inhibit(false)
}
