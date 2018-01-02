use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::cell::RefCell;

use gtk;
use gtk::prelude::*;
use cairo;

use neovim_lib::Value;

use ui_model::{Attrs, ModelLayout};
use ui::UiMutex;
use render;
use shell;
use cursor;

pub struct Level {
    model_layout: ModelLayout,
}

impl Level {

    pub fn from(
        content: Vec<(HashMap<String, Value>, String)>,
        pos: u64,
        firstc: String,
        prompt: String,
        indent: u64,
    ) -> Self {
        //TODO: double width chars
        //TODO: im

        let content_line: Vec<(Option<Attrs>, Vec<char>)> = content
            .iter()
            .map(|c| (Some(Attrs::from_value_map(&c.0)), c.1.chars().collect()))
            .collect();
        let prompt_lines = prompt_lines(firstc, prompt, indent);

        let mut content: Vec<_> = prompt_lines.into_iter().map(|line| vec![line]).collect();

        if content.is_empty() {
            content.push(content_line);
        } else {
            content.last_mut().map(|line| line.extend(content_line));
        }

        let mut model_layout = ModelLayout::new();
        // TODO: calculate width
        model_layout.layout(content, 5);

        Level { model_layout }
    }

    fn update_cache(&mut self, render_state: &shell::RenderState) {
        render::shape_dirty(
            &render_state.font_ctx,
            &mut self.model_layout.model,
            &render_state.color_model,
        );
    }
}

fn prompt_lines(firstc: String, prompt: String, indent: u64) -> Vec<(Option<Attrs>, Vec<char>)> {
    if !firstc.is_empty() {
        vec![(None, firstc.chars().chain((0..indent).map(|_| ' ')).collect())]
    } else if !prompt.is_empty() {
        prompt.lines().map(|l| (None, l.chars().collect())).collect()
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
        popover.set_position(gtk::PositionType::Top);

        let edit_frame = gtk::Frame::new(None);
        edit_frame.set_shadow_type(gtk::ShadowType::In);
        let drawing_area = gtk::DrawingArea::new();
        drawing_area.set_size_request(150, 50);
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
            let allocation = self.popover.get_relative_to().unwrap().get_allocation();
            self.popover.set_pointing_to(&gtk::Rectangle {
                x: allocation.width / 2,
                y: allocation.height / 2,
                width: 1,
                height: 1,
            });

            self.popover.popup();
        } else {
            state.drawing_area.queue_draw()
        }
    }

    pub fn hide_level(&mut self, level_idx: u64) {
        let mut state = self.state.borrow_mut();

        if level_idx as usize == state.levels.len() {
            state.levels.pop();
        }

        if state.levels.is_empty() {
            self.popover.hide();
            self.displyed = false;
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
            &level.model_layout.model,
            &render_state.color_model,
            &render_state.mode,
        );
    }
    Inhibit(false)
}
