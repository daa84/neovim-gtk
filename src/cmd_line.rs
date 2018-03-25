use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::cell::RefCell;
use std::cmp::max;

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
    prompt_offset: usize,
    preferred_width: i32,
    preferred_height: i32,
}

impl Level {
    //TODO: im

    pub fn insert(&mut self, c: &str, shift: bool, render_state: &shell::RenderState) {
        self.model_layout.insert_char(c, shift);
        self.update_preferred_size(render_state);
    }

    pub fn replace_from_ctx(&mut self, ctx: &CmdLineContext, render_state: &shell::RenderState) {
        let content = ctx.get_lines();
        self.replace_line(content.lines, false);
        self.prompt_offset = content.prompt_offset;
        self.model_layout
            .set_cursor(self.prompt_offset + ctx.pos as usize);
        self.update_preferred_size(render_state);
    }

    pub fn from_ctx(ctx: &CmdLineContext, render_state: &shell::RenderState) -> Self {
        let content = ctx.get_lines();
        let mut level = Level::from_lines(content.lines, ctx.max_width, render_state);

        level.prompt_offset = content.prompt_offset;
        level
            .model_layout
            .set_cursor(level.prompt_offset + ctx.pos as usize);
        level.update_preferred_size(render_state);

        level
    }

    fn replace_line(&mut self, lines: Vec<Vec<(Option<Attrs>, Vec<char>)>>, append: bool) {
        if append {
            self.model_layout.layout_append(lines);
        } else {
            self.model_layout.layout(lines);
        }
    }

    fn update_preferred_size(&mut self, render_state: &shell::RenderState) {
        let &CellMetrics {
            line_height,
            char_width,
            ..
        } = render_state.font_ctx.cell_metrics();

        let (columns, rows) = self.model_layout.size();
        let columns = max(columns, 5);

        self.preferred_width = (char_width * columns as f64) as i32;
        self.preferred_height = (line_height * rows as f64) as i32;
    }

    fn to_attributed_content(
        content: &Vec<Vec<(HashMap<String, Value>, String)>>,
    ) -> Vec<Vec<(Option<Attrs>, Vec<char>)>> {
        content
            .iter()
            .map(|line_chars| {
                line_chars
                    .iter()
                    .map(|c| (Some(Attrs::from_value_map(&c.0)), c.1.chars().collect()))
                    .collect()
            })
            .collect()
    }

    pub fn from_multiline_content(
        content: &Vec<Vec<(HashMap<String, Value>, String)>>,
        max_width: i32,
        render_state: &shell::RenderState,
    ) -> Self {
        Level::from_lines(
            Level::to_attributed_content(content),
            max_width,
            render_state,
        )
    }

    pub fn from_lines(
        lines: Vec<Vec<(Option<Attrs>, Vec<char>)>>,
        max_width: i32,
        render_state: &shell::RenderState,
    ) -> Self {
        let &CellMetrics { char_width, .. } = render_state.font_ctx.cell_metrics();

        let max_width_chars = (max_width as f64 / char_width) as u64;

        let mut model_layout = ModelLayout::new(max_width_chars);
        model_layout.layout(lines);

        let mut level = Level {
            model_layout,
            preferred_width: -1,
            preferred_height: -1,
            prompt_offset: 0,
        };

        level.update_preferred_size(render_state);
        level
    }

    fn update_cache(&mut self, render_state: &shell::RenderState) {
        render::shape_dirty(
            &render_state.font_ctx,
            &mut self.model_layout.model,
            &render_state.color_model,
        );
    }

    fn set_cursor(&mut self, render_state: &shell::RenderState, pos: usize) {
        self.model_layout.set_cursor(self.prompt_offset + pos);
        self.update_preferred_size(render_state);
    }
}

fn prompt_lines(
    firstc: &str,
    prompt: &str,
    indent: u64,
) -> (usize, Vec<(Option<Attrs>, Vec<char>)>) {
    let prompt: Vec<(Option<Attrs>, Vec<char>)> = if !firstc.is_empty() {
        if firstc.len() >= indent as usize {
            vec![(None, firstc.chars().collect())]
        } else {
            vec![
                (
                    None,
                    firstc
                        .chars()
                        .chain((firstc.len()..indent as usize).map(|_| ' '))
                        .collect(),
                ),
            ]
        }
    } else if !prompt.is_empty() {
        prompt
            .lines()
            .map(|l| (None, l.chars().collect()))
            .collect()
    } else {
        vec![]
    };

    let prompt_offset = prompt.last().map(|l| l.1.len()).unwrap_or(0);

    (prompt_offset, prompt)
}

struct State {
    levels: Vec<Level>,
    block: Option<Level>,
    render_state: Rc<RefCell<shell::RenderState>>,
    drawing_area: gtk::DrawingArea,
    cursor: Option<cursor::BlinkCursor<State>>,
}

impl State {
    fn new(drawing_area: gtk::DrawingArea, render_state: Rc<RefCell<shell::RenderState>>) -> Self {
        State {
            levels: Vec::new(),
            block: None,
            render_state,
            drawing_area,
            cursor: None,
        }
    }

    fn request_area_size(&self) {
        let drawing_area = self.drawing_area.clone();
        let block = self.block.as_ref();
        let level = self.levels.last();

        let (block_width, block_height) = block
            .map(|b| (b.preferred_width, b.preferred_height))
            .unwrap_or((0, 0));
        let (level_width, level_height) = level
            .map(|l| (l.preferred_width, l.preferred_height))
            .unwrap_or((0, 0));

        drawing_area.set_size_request(
            max(level_width, block_width),
            max(block_height + level_height, 40),
        );
    }

    fn preferred_height(&self) -> i32 {
        let level = self.levels.last();
        level.map(|l| l.preferred_height).unwrap_or(0)
            + self.block.as_ref().map(|b| b.preferred_height).unwrap_or(0)
    }

    fn set_cursor(&mut self, render_state: &shell::RenderState, pos: usize, level: usize) {
        debug_assert!(level > 0);

        // queue old cursor position
        self.queue_redraw_cursor();

        self.levels
            .get_mut(level - 1)
            .map(|l| l.set_cursor(render_state, pos));
    }

    fn queue_redraw_cursor(&mut self) {
        if let Some(ref level) = self.levels.last() {
            let level_preferred_height = level.preferred_height;
            let block_preferred_height =
                self.block.as_ref().map(|b| b.preferred_height).unwrap_or(0);

            let gap = self.drawing_area.get_allocated_height() - level_preferred_height
                - block_preferred_height;

            let model = &level.model_layout.model;

            let mut cur_point = model.cur_point();
            cur_point.extend_by_items(model);

            let render_state = self.render_state.borrow();
            let cell_metrics = render_state.font_ctx.cell_metrics();

            let (x, y, width, height) = cur_point.to_area_extend_ink(model, cell_metrics);

            if gap > 0 {
                self.drawing_area
                    .queue_draw_area(x, y + gap / 2, width, height);
            } else {
                self.drawing_area
                    .queue_draw_area(x, y + block_preferred_height, width, height);
            }
        }
    }
}

impl cursor::CursorRedrawCb for State {
    fn queue_redraw_cursor(&mut self) {
        self.queue_redraw_cursor();
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

        let drawing_area = gtk::DrawingArea::new();
        drawing_area.show_all();
        popover.add(&drawing_area);

        let state = Arc::new(UiMutex::new(State::new(drawing_area.clone(), render_state)));
        let weak_cb = Arc::downgrade(&state);
        let cursor = cursor::BlinkCursor::new(weak_cb);
        state.borrow_mut().cursor = Some(cursor);

        drawing_area.connect_draw(clone!(state => move |_, ctx| gtk_draw(ctx, &state)));

        CmdLine {
            popover,
            state,
            displyed: false,
        }
    }

    pub fn show_level(&mut self, ctx: &CmdLineContext) {
        let mut state = self.state.borrow_mut();
        let render_state = state.render_state.clone();
        let render_state = render_state.borrow();

        if ctx.level_idx as usize == state.levels.len() {
            let level = state.levels.last_mut().unwrap();
            level.replace_from_ctx(ctx, &*render_state);
            level.update_cache(&*render_state);
        } else {
            let mut level = Level::from_ctx(ctx, &*render_state);
            level.update_cache(&*render_state);
            state.levels.push(level);
        }

        state.request_area_size();

        if !self.displyed {
            self.displyed = true;
            self.popover.set_pointing_to(&gtk::Rectangle {
                x: ctx.x,
                y: ctx.y,
                width: ctx.width,
                height: ctx.height,
            });

            self.popover.popup();
            state.cursor.as_mut().unwrap().start();
        } else {
            state.drawing_area.queue_draw()
        }
    }

    pub fn special_char(
        &self,
        render_state: &shell::RenderState,
        c: String,
        shift: bool,
        level: u64,
    ) {
        let mut state = self.state.borrow_mut();

        if let Some(level) = state.levels.get_mut((level - 1) as usize) {
            level.insert(&c, shift, render_state);
            level.update_cache(&*render_state);
        } else {
            error!("Level {} does not exists", level);
        }

        state.request_area_size();
        state.drawing_area.queue_draw()
    }

    pub fn hide_level(&mut self, level_idx: u64) {
        let mut state = self.state.borrow_mut();

        if level_idx as usize == state.levels.len() {
            state.levels.pop();
        }

        if state.levels.is_empty() {
            self.popover.hide();
            self.displyed = false;
            state.cursor.as_mut().unwrap().leave_focus();
        }
    }

    pub fn show_block(
        &mut self,
        content: &Vec<Vec<(HashMap<String, Value>, String)>>,
        max_width: i32,
    ) {
        let mut state = self.state.borrow_mut();
        let mut block =
            Level::from_multiline_content(content, max_width, &*state.render_state.borrow());
        block.update_cache(&*state.render_state.borrow());
        state.block = Some(block);
        state.request_area_size();
    }

    pub fn block_append(&mut self, content: &Vec<(HashMap<String, Value>, String)>) {
        let mut state = self.state.borrow_mut();
        let render_state = state.render_state.clone();
        {
            let attr_content = content
                .iter()
                .map(|c| (Some(Attrs::from_value_map(&c.0)), c.1.chars().collect()))
                .collect();

            let block = state.block.as_mut().unwrap();
            block.replace_line(vec![attr_content], true);
            block.update_preferred_size(&*render_state.borrow());
            block.update_cache(&*render_state.borrow());
        }
        state.request_area_size();
    }

    pub fn block_hide(&mut self) {
        self.state.borrow_mut().block = None;
    }

    pub fn pos(&mut self, render_state: &shell::RenderState, pos: u64, level: u64) {
        self.state
            .borrow_mut()
            .set_cursor(render_state, pos as usize, level as usize);
    }
}

fn gtk_draw(ctx: &cairo::Context, state: &Arc<UiMutex<State>>) -> Inhibit {
    let state = state.borrow();
    let preferred_height = state.preferred_height();
    let level = state.levels.last();
    let block = state.block.as_ref();

    let render_state = state.render_state.borrow();

    let gap = state.drawing_area.get_allocated_height() - preferred_height;
    if gap > 0 {
        ctx.translate(0.0, (gap / 2) as f64);
    }

    render::clear(ctx, &render_state.color_model);

    if let Some(block) = block {
        render::render(
            ctx,
            &cursor::EmptyCursor::new(),
            &render_state.font_ctx,
            &block.model_layout.model,
            &render_state.color_model,
            &render_state.mode,
        );

        ctx.translate(0.0, block.preferred_height as f64);
    }

    if let Some(level) = level {
        //TODO: limit model to row filled
        render::render(
            ctx,
            state.cursor.as_ref().unwrap(),
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

impl CmdLineContext {
    fn get_lines(&self) -> LineContent {
        let content_line: Vec<(Option<Attrs>, Vec<char>)> = self.content
            .iter()
            .map(|c| (Some(Attrs::from_value_map(&c.0)), c.1.chars().collect()))
            .collect();
        let (prompt_offset, prompt_lines) = prompt_lines(&self.firstc, &self.prompt, self.indent);

        let mut content: Vec<_> = prompt_lines.into_iter().map(|line| vec![line]).collect();

        if content.is_empty() {
            content.push(content_line);
        } else {
            content.last_mut().map(|line| line.extend(content_line));
        }

        LineContent {
            lines: content,
            prompt_offset,
        }
    }
}

struct LineContent {
    lines: Vec<Vec<(Option<Attrs>, Vec<char>)>>,
    prompt_offset: usize,
}
