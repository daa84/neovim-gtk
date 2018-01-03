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
use render::{self, CellMetrics};
use shell;
use cursor;

pub struct Level {
    model_layout: ModelLayout,
    preferred_width: i32,
    preferred_height: i32,
}

impl Level {

    pub fn from(ctx: &CmdLineContext, render_state: &shell::RenderState) -> Self {
        //TODO: double width chars render, also note in text wrapping
        //TODO: im

        let content_line: Vec<(Option<Attrs>, Vec<char>)> = ctx.content
            .iter()
            .map(|c| (Some(Attrs::from_value_map(&c.0)), c.1.chars().collect()))
            .collect();
        let prompt_lines = prompt_lines(&ctx.firstc, &ctx.prompt, ctx.indent);

        let mut content: Vec<_> = prompt_lines.into_iter().map(|line| vec![line]).collect();

        if content.is_empty() {
            content.push(content_line);
        } else {
            content.last_mut().map(|line| line.extend(content_line));
        }

        let &CellMetrics {
            line_height,
            char_width,
            ..
        } = render_state.font_ctx.cell_metrics();

        let max_width_chars = (ctx.max_width as f64 / char_width) as u64;

        let mut model_layout = ModelLayout::new();
        let (columns, rows) = model_layout.layout(content, max_width_chars);

        let preferred_width = (char_width * columns as f64) as i32;
        let preferred_height = (line_height * rows as f64) as i32;
        Level { model_layout, preferred_width, preferred_height }
    }

    fn update_cache(&mut self, render_state: &shell::RenderState) {
        render::shape_dirty(
            &render_state.font_ctx,
            &mut self.model_layout.model,
            &render_state.color_model,
        );
    }
}

fn prompt_lines(firstc: &str, prompt: &str, indent: u64) -> Vec<(Option<Attrs>, Vec<char>)> {
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
        popover.set_position(gtk::PositionType::Right);

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
        ctx: &CmdLineContext,
    ) {
        let mut state = self.state.borrow_mut();

        let mut level = Level::from(ctx, &*state.render_state.borrow());
        level.update_cache(&*state.render_state.borrow());

        if ctx.level_idx as usize == state.levels.len() {
            // TODO: update level
            state.levels.pop();
        }
        state.levels.push(level);
        if !self.displyed {
            self.displyed = true;
            self.popover.set_pointing_to(&gtk::Rectangle {
                x: ctx.x,
                y: ctx.y,
                width: ctx.width,
                height: ctx.height,
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

pub struct CmdLineContext {
    pub content: Vec<(HashMap<String, Value>, String)>,
    pub pos: u64,
    pub firstc: String,
    pub prompt: String,
    pub indent: u64,
    pub level_idx: u64,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub max_width: i32,
}
