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
    preferred_width: i32,
    preferred_height: i32,
}

impl Level {
    //TODO: double width chars render, also note in text wrapping
    //TODO: im
    //TODO: cursor

    pub fn replace_from_ctx(&mut self, ctx: &CmdLineContext, render_state: &shell::RenderState) {
        self.replace_line(&ctx.get_lines(), render_state, false);
    }

    fn replace_line(
        &mut self,
        lines: &Vec<Vec<(Option<Attrs>, Vec<char>)>>,
        render_state: &shell::RenderState,
        append: bool,
    ) {
        let &CellMetrics {
            line_height,
            char_width,
            ..
        } = render_state.font_ctx.cell_metrics();

        let (columns, rows) = if append {
            self.model_layout.layout_append(lines)
        } else {
            self.model_layout.layout(lines)
        };

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
                    .map(|c| {
                        (Some(Attrs::from_value_map(&c.0)), c.1.chars().collect())
                    })
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
            &Level::to_attributed_content(content),
            max_width,
            render_state,
        )
    }

    pub fn from_lines(
        lines: &Vec<Vec<(Option<Attrs>, Vec<char>)>>,
        max_width: i32,
        render_state: &shell::RenderState,
    ) -> Self {
        let &CellMetrics {
            line_height,
            char_width,
            ..
        } = render_state.font_ctx.cell_metrics();

        let max_width_chars = (max_width as f64 / char_width) as u64;

        let mut model_layout = ModelLayout::new(max_width_chars);
        let (columns, rows) = model_layout.layout(lines);

        let columns = max(columns, 5);

        let preferred_width = (char_width * columns as f64) as i32;
        let preferred_height = (line_height * rows as f64) as i32;

        Level {
            model_layout,
            preferred_width,
            preferred_height,
        }
    }

    pub fn from_ctx(ctx: &CmdLineContext, render_state: &shell::RenderState) -> Self {
        Level::from_lines(&ctx.get_lines(), ctx.max_width, render_state)
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
        vec![
            (
                None,
                firstc.chars().chain((0..indent).map(|_| ' ')).collect(),
            ),
        ]
    } else if !prompt.is_empty() {
        prompt
            .lines()
            .map(|l| (None, l.chars().collect()))
            .collect()
    } else {
        vec![]
    }
}

struct State {
    levels: Vec<Level>,
    block: Option<Level>,
    render_state: Rc<RefCell<shell::RenderState>>,
    drawing_area: gtk::DrawingArea,
}

impl State {
    fn new(drawing_area: gtk::DrawingArea, render_state: Rc<RefCell<shell::RenderState>>) -> Self {
        State {
            levels: Vec::new(),
            block: None,
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

        let drawing_area = gtk::DrawingArea::new();
        drawing_area.show_all();
        popover.add(&drawing_area);

        let state = Arc::new(UiMutex::new(State::new(drawing_area.clone(), render_state)));
        let weak_cb = Arc::downgrade(&state);
        let cursor = cursor::Cursor::new(weak_cb);

        drawing_area.connect_draw(clone!(state => move |_, ctx| gtk_draw(ctx, &state, &cursor)));

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
            state
                .levels
                .last_mut()
                .unwrap()
                .replace_from_ctx(ctx, &*render_state);
        } else {
            let level = Level::from_ctx(ctx, &*render_state);
            state.levels.push(level);
        }



        let drawing_area = state.drawing_area.clone();
        let block_height = state
            .block
            .as_ref()
            .map(|b| b.preferred_height)
            .unwrap_or(0);
        let level = state.levels.last_mut().unwrap();

        level.update_cache(&*render_state);
        drawing_area.set_size_request(
            level.preferred_width,
            max(block_height + level.preferred_height, 40),
        );

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
            drawing_area.queue_draw()
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
        //TODO: drawing size update
    }


    pub fn block_append(&mut self, content: &Vec<Vec<(HashMap<String, Value>, String)>>) {
        let mut state = self.state.borrow_mut();
        let render_state = state.render_state.clone();
        let block = state.block.as_mut().unwrap();
        block.replace_line(
            &Level::to_attributed_content(content),
            &*render_state.borrow(),
            true,
        );
        block.update_cache(&*render_state.borrow());
        //TODO: drawing size update
    }

    pub fn block_hide(&mut self) {
        self.state.borrow_mut().block = None;
    }
}

fn gtk_draw(
    ctx: &cairo::Context,
    state: &Arc<UiMutex<State>>,
    cursor: &cursor::Cursor<State>,
) -> Inhibit {
    let state = state.borrow();
    let level = state.levels.last();

    //TODO: draw block
    if let Some(level) = level {
        let render_state = state.render_state.borrow();
        let gap = state.drawing_area.get_allocated_height() - level.preferred_height;
        if gap > 0 {
            ctx.translate(0.0, gap as f64 / 2.0);
        }

        //TODO: limit model to row filled
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

impl CmdLineContext {
    fn get_lines(&self) -> Vec<Vec<(Option<Attrs>, Vec<char>)>> {
        let content_line: Vec<(Option<Attrs>, Vec<char>)> = self.content
            .iter()
            .map(|c| {
                (Some(Attrs::from_value_map(&c.0)), c.1.chars().collect())
            })
            .collect();
        let prompt_lines = prompt_lines(&self.firstc, &self.prompt, self.indent);

        let mut content: Vec<_> = prompt_lines.into_iter().map(|line| vec![line]).collect();

        if content.is_empty() {
            content.push(content_line);
        } else {
            content.last_mut().map(|line| line.extend(content_line));
        }

        content
    }
}
