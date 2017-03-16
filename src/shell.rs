use std::string::String;

use cairo;
use pangocairo as pc;
use pango;
use pango::FontDescription;
use gdk::{ModifierType, EventKey, EventConfigure, EventButton, EventMotion, EventType, EventScroll, ScrollDirection};
use gdk_sys;
use glib;
use gtk::prelude::*;
use gtk::DrawingArea;

use neovim_lib::{Neovim, NeovimApi, Value, Integer};

use settings;
use ui_model::{UiModel, Cell, Attrs, Color, COLOR_BLACK, COLOR_WHITE, COLOR_RED};
use nvim::{RedrawEvents, GuiApi};
use input::{convert_key, keyval_to_input_string};
use ui::{UI, Ui, SET};

const DEFAULT_FONT_NAME: &'static str = "DejaVu Sans Mono 12";

macro_rules! SHELL {
    ($id:ident = $expr:expr) => (
        UI.with(|ui_cell| {
        let mut $id = &mut ui_cell.borrow_mut().shell;
        $expr
    });
    )
}

#[derive(PartialEq)]
pub enum NvimMode {
    Normal,
    Insert,
    Other,
}

pub struct Shell {
    pub model: UiModel,
    pub drawing_area: DrawingArea,
    nvim: Option<Neovim>,
    cur_attrs: Option<Attrs>,
    bg_color: Color,
    fg_color: Color,
    sp_color: Color,
    line_height: Option<f64>,
    char_width: Option<f64>,
    pub mode: NvimMode,
    mouse_enabled: bool,
    mouse_pressed: bool,
    font_desc: FontDescription,
    resize_timer: Option<glib::SourceId>,
}

impl Shell {
    pub fn new() -> Shell {
        Shell { 
            model: UiModel::empty(),
            drawing_area: DrawingArea::new(),
            nvim: None,
            cur_attrs: None,
            bg_color: COLOR_BLACK,
            fg_color: COLOR_WHITE,
            sp_color: COLOR_RED,
            line_height: None,
            char_width: None,
            mode: NvimMode::Normal,
            mouse_enabled: true,
            mouse_pressed: false,
            font_desc: FontDescription::from_string(DEFAULT_FONT_NAME),
            resize_timer: None,
        }
    }

    pub fn init(&mut self) {
        self.drawing_area.set_size_request(500, 300);
        self.drawing_area.set_hexpand(true);
        self.drawing_area.set_vexpand(true);
        self.drawing_area.set_can_focus(true);

        self.drawing_area
            .set_events((gdk_sys::GDK_BUTTON_RELEASE_MASK | gdk_sys::GDK_BUTTON_PRESS_MASK |
                         gdk_sys::GDK_BUTTON_MOTION_MASK | gdk_sys::GDK_SCROLL_MASK)
                            .bits() as i32);
        self.drawing_area.connect_button_press_event(gtk_button_press);
        self.drawing_area.connect_button_release_event(gtk_button_release);
        self.drawing_area.connect_motion_notify_event(gtk_motion_notify);
        self.drawing_area.connect_draw(gtk_draw);
        self.drawing_area.connect_key_press_event(gtk_key_press);
        self.drawing_area.connect_scroll_event(gtk_scroll_event);
    }

    pub fn add_configure_event(&mut self) {
        self.drawing_area.connect_configure_event(gtk_configure_event);
    }

    pub fn set_nvim(&mut self, nvim: Neovim) {
        self.nvim = Some(nvim);
    }

    pub fn nvim(&mut self) -> &mut Neovim {
        self.nvim.as_mut().unwrap()
    }

    fn create_pango_font(&self) -> FontDescription {
        self.font_desc.clone()
    }

    pub fn set_font_desc(&mut self, desc: &str) {
        self.font_desc = FontDescription::from_string(desc);
    }

    fn colors<'a>(&'a self, cell: &'a Cell) -> (&'a Color, &'a Color) {
        let bg = if let Some(ref bg) = cell.attrs.background {
            bg
        } else {
            &self.bg_color
        };
        let fg = if let Some(ref fg) = cell.attrs.foreground {
            fg
        } else {
            &self.fg_color
        };

        if cell.attrs.reverse {
            (fg, bg)
        } else {
            (bg, fg)
        }
    }
}

fn gtk_scroll_event(_: &DrawingArea, ev: &EventScroll) -> Inhibit {
    SHELL!(shell = {
        if !shell.mouse_enabled {
            return;
        }

        match ev.as_ref().direction {
            ScrollDirection::Right => mouse_input(&mut shell, "ScrollWheelRight", ev.get_state(), ev.get_position()),
            ScrollDirection::Left => mouse_input(&mut shell, "ScrollWheelLeft", ev.get_state(), ev.get_position()),
            ScrollDirection::Up => mouse_input(&mut shell, "ScrollWheelUp", ev.get_state(), ev.get_position()),
            ScrollDirection::Down => mouse_input(&mut shell, "ScrollWheelDown", ev.get_state(), ev.get_position()),
            _ => (),
        }
    });
    Inhibit(false)
}

fn gtk_button_press(_: &DrawingArea, ev: &EventButton) -> Inhibit {
    if ev.get_event_type() != EventType::ButtonPress {
        return Inhibit(false);
    }

    SHELL!(shell = {
        if !shell.mouse_enabled {
            return;
        }

        mouse_input(&mut shell, "LeftMouse", ev.get_state(), ev.get_position());
    });
    Inhibit(false)
}

fn mouse_input(shell: &mut Shell, input: &str, state: ModifierType, position: (f64, f64)) {
    if let Some(line_height) = shell.line_height {
        if let Some(char_width) = shell.char_width {
            shell.mouse_pressed = true;

            let nvim = shell.nvim();
            let (x, y) = position;
            let col = (x / char_width).trunc() as u64;
            let row = (y / line_height).trunc() as u64;
            let input_str = format!("{}<{},{}>", keyval_to_input_string(input, state), col, row);
            nvim.input(&input_str).expect("Can't send mouse input event");
        }
    }
}

fn gtk_button_release(_: &DrawingArea, _: &EventButton) -> Inhibit {
    SHELL!(shell = {
        shell.mouse_pressed = false;
    });
    Inhibit(false)
}

fn gtk_motion_notify(_: &DrawingArea, ev: &EventMotion) -> Inhibit {
    SHELL!(shell = {
        if !shell.mouse_enabled || !shell.mouse_pressed {
            return;
        }

        mouse_input(&mut shell, "LeftDrag", ev.get_state(), ev.get_position());
    });
    Inhibit(false)
}

fn gtk_key_press(_: &DrawingArea, ev: &EventKey) -> Inhibit {
    if let Some(input) = convert_key(ev) {
        SHELL!(shell = {
            shell.nvim().input(&input).expect("Error run input command to nvim");
        });
        Inhibit(true)
    } else {
        Inhibit(false)
    }
}

fn gtk_draw(_: &DrawingArea, ctx: &cairo::Context) -> Inhibit {
    UI.with(|ui_cell| {
        let mut ui = ui_cell.borrow_mut();

        let (width, height) = calc_char_bounds(&ui.shell, ctx);
        ui.shell.line_height = Some(height as f64);
        ui.shell.char_width = Some(width as f64);

        draw(&ui.shell, ctx);
        request_width(&ui);
    });

    Inhibit(false)
}

#[inline]
fn draw_joined_rect(shell: &Shell,
                    ctx: &cairo::Context,
                    from_col_idx: usize,
                    col_idx: usize,
                    char_width: f64,
                    line_height: f64,
                    color: &Color) {
    let current_point = ctx.get_current_point();
    let rect_width = char_width * (col_idx - from_col_idx) as f64;

    if &shell.bg_color != color {
        ctx.set_source_rgb(color.0, color.1, color.2);
        ctx.rectangle(current_point.0, current_point.1, rect_width, line_height);
        ctx.fill();
    }

    ctx.move_to(current_point.0 + rect_width, current_point.1);
}

fn draw(shell: &Shell, ctx: &cairo::Context) {
    ctx.set_source_rgb(shell.bg_color.0, shell.bg_color.1, shell.bg_color.2);
    ctx.paint();

    let line_height = shell.line_height.unwrap();
    let char_width = shell.char_width.unwrap();
    let (row, col) = shell.model.get_cursor();
    let mut buf = String::with_capacity(4);

    let mut line_y: f64 = 0.0;


    let layout = pc::create_layout(ctx);
    let mut desc = shell.create_pango_font();

    for (line_idx, line) in shell.model.model().iter().enumerate() {
        ctx.move_to(0.0, line_y);

        // first draw background
        // here we join same bg color for given line
        // this gives less drawing primitives
        let mut from_col_idx = 0;
        let mut from_bg = None;
        for (col_idx, cell) in line.iter().enumerate() {
            let (bg, _) = shell.colors(cell);

            if from_bg.is_none() {
                from_bg = Some(bg);
                from_col_idx = col_idx;
            } else if from_bg != Some(bg) {
                draw_joined_rect(shell,
                                 ctx,
                                 from_col_idx,
                                 col_idx,
                                 char_width,
                                 line_height,
                                 from_bg.take().unwrap());
                from_bg = Some(bg);
                from_col_idx = col_idx;
            }
        }
        draw_joined_rect(shell,
                         ctx,
                         from_col_idx,
                         line.len(),
                         char_width,
                         line_height,
                         from_bg.take().unwrap());

        ctx.move_to(0.0, line_y);

        for (col_idx, cell) in line.iter().enumerate() {
            let double_width = line.get(col_idx + 1).map(|c| c.attrs.double_width).unwrap_or(false);
            let current_point = ctx.get_current_point();

            let (bg, fg) = shell.colors(cell);

            if row == line_idx && col == col_idx {
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
                ctx.move_to(current_point.0, current_point.1);
            }


            if !cell.ch.is_whitespace() {
                update_font_description(&mut desc, &cell.attrs);

                layout.set_font_description(Some(&desc));
                buf.clear();
                buf.push(cell.ch);
                layout.set_text(&buf, -1);

                // correct layout for double_width chars
                if double_width {
                    let (dw_width, dw_height) = layout.get_pixel_size();
                    let x_offset = (char_width * 2.0 - dw_width as f64) / 2.0;
                    let y_offset = (line_height - dw_height as f64) / 2.0;
                    ctx.rel_move_to(x_offset, y_offset);
                }

                ctx.set_source_rgb(fg.0, fg.1, fg.2);
                pc::update_layout(ctx, &layout);
                pc::show_layout(ctx, &layout);
            }

            if cell.attrs.underline || cell.attrs.undercurl {
                // [TODO]: Current gtk-rs bindings does not provide fontmetrics access
                // so it is not possible to find right position for underline or undercurl position
                // > update_font_description(&mut desc, &cell.attrs);
                // > layout.get_context().unwrap().get_metrics();
                let top_offset = line_height * 0.9;

                let sp = if let Some(ref sp) = cell.attrs.special {
                    sp
                } else {
                    &shell.sp_color
                };

                ctx.set_source_rgba(sp.0, sp.1, sp.2, 0.7);
                if cell.attrs.undercurl {
                    ctx.set_dash(&[4.0, 2.0], 0.0);
                    ctx.set_line_width(2.0);
                    ctx.move_to(current_point.0, line_y + top_offset);
                    ctx.line_to(current_point.0 + char_width, line_y + top_offset);
                    ctx.stroke();
                    ctx.set_dash(&[], 0.0);
                } else if cell.attrs.underline {
                    ctx.set_line_width(1.0);
                    ctx.move_to(current_point.0, line_y + top_offset);
                    ctx.line_to(current_point.0 + char_width, line_y + top_offset);
                    ctx.stroke();
                }
            }

            ctx.move_to(current_point.0 + char_width, current_point.1);
        }

        line_y += line_height;
    }
}

#[inline]
fn update_font_description(desc: &mut FontDescription, attrs: &Attrs) {
    desc.unset_fields(pango::FONT_MASK_STYLE | pango::FONT_MASK_WEIGHT);
    if attrs.italic {
        desc.set_style(pango::Style::Italic);
    }
    if attrs.bold {
        desc.set_weight(pango::Weight::Bold);
    }
}

fn calc_char_bounds(shell: &Shell, ctx: &cairo::Context) -> (i32, i32) {
    let layout = pc::create_layout(ctx);

    let desc = shell.create_pango_font();
    layout.set_font_description(Some(&desc));
    layout.set_text("A", -1);

    layout.get_pixel_size()
}

fn request_width(ui: &Ui) {
    if ui.shell.resize_timer.is_some() {
        return;
    }

    let width = ui.shell.drawing_area.get_allocated_width();
    let height = ui.shell.drawing_area.get_allocated_height();
    let request_height = (ui.shell.model.rows as f64 * ui.shell.line_height.unwrap()) as i32;
    let request_width = (ui.shell.model.columns as f64 * ui.shell.char_width.unwrap()) as i32;

    if width != request_width || height != request_height {
        let window = ui.window.as_ref().unwrap();
        let (win_width, win_height) = window.get_size();
        let h_border = win_width - width;
        let v_border = win_height - height;
        window.resize(request_width + h_border, request_height + v_border);
    }
}

fn split_color(indexed_color: u64) -> Color {
    let r = ((indexed_color >> 16) & 0xff) as f64;
    let g = ((indexed_color >> 8) & 0xff) as f64;
    let b = (indexed_color & 0xff) as f64;
    Color(r / 255.0, g / 255.0, b / 255.0)
}

fn gtk_configure_event(_: &DrawingArea, ev: &EventConfigure) -> bool {
    SHELL!(shell = {
        let (width, height) = ev.get_size();

        if let Some(timer) = shell.resize_timer {
            glib::source_remove(timer);
        }
        if let Some(line_height) = shell.line_height {
            if let Some(char_width) = shell.char_width {

                shell.resize_timer = Some(glib::timeout_add(250, move || {
                    SHELL!(shell = {
                        shell.resize_timer = None;

                        let rows = (height as f64 / line_height).trunc() as usize;
                        let columns = (width as f64 / char_width).trunc() as usize;
                        if shell.model.rows != rows || shell.model.columns != columns {
                            if let Err(err) = shell.nvim().ui_try_resize(columns as u64, rows as u64) {
                                println!("Error trying resize nvim {}", err);
                            }
                        }
                    });
                    Continue(false)
                }));
            }
        }
    });
    false
}

impl RedrawEvents for Shell {
    fn on_cursor_goto(&mut self, row: u64, col: u64) {
        self.model.set_cursor(row, col);
    }

    fn on_put(&mut self, text: &str) {
        self.model.put(text, self.cur_attrs.as_ref());
    }

    fn on_clear(&mut self) {
        self.model.clear();
    }

    fn on_eol_clear(&mut self) {
        self.model.eol_clear();
    }

    fn on_resize(&mut self, columns: u64, rows: u64) {
        self.model = UiModel::new(rows, columns);
    }

    fn on_redraw(&self) {
        self.drawing_area.queue_draw();
    }

    fn on_set_scroll_region(&mut self, top: u64, bot: u64, left: u64, right: u64) {
        self.model.set_scroll_region(top, bot, left, right);
    }

    fn on_scroll(&mut self, count: i64) {
        self.model.scroll(count);
    }

    fn on_highlight_set(&mut self, attrs: &Vec<(Value, Value)>) {
        let mut model_attrs = Attrs::new();

        for &(ref key_val, ref val) in attrs {
            if let &Value::String(ref key) = key_val {
                match key.as_ref() {
                    "foreground" => {
                        if let &Value::Integer(Integer::U64(fg)) = val {
                            model_attrs.foreground = Some(split_color(fg));
                        }
                    }
                    "background" => {
                        if let &Value::Integer(Integer::U64(bg)) = val {
                            model_attrs.background = Some(split_color(bg));
                        }
                    }
                    "special" => {
                        if let &Value::Integer(Integer::U64(bg)) = val {
                            model_attrs.special = Some(split_color(bg));
                        }
                    }
                    "reverse" => model_attrs.reverse = true,
                    "bold" => model_attrs.bold = true,
                    "italic" => model_attrs.italic = true,
                    "underline" => model_attrs.underline = true,
                    "undercurl" => model_attrs.undercurl = true,
                    attr_key => println!("unknown attribute {}", attr_key),
                };
            } else {
                panic!("attr key must be string");
            }
        }

        self.cur_attrs = Some(model_attrs);
    }

    fn on_update_bg(&mut self, bg: i64) {
        if bg >= 0 {
            self.bg_color = split_color(bg as u64);
        } else {
            self.bg_color = COLOR_BLACK;
        }
    }

    fn on_update_fg(&mut self, fg: i64) {
        if fg >= 0 {
            self.fg_color = split_color(fg as u64);
        } else {
            self.fg_color = COLOR_WHITE;
        }
    }

    fn on_update_sp(&mut self, sp: i64) {
        if sp >= 0 {
            self.sp_color = split_color(sp as u64);
        } else {
            self.sp_color = COLOR_RED;
        }
    }

    fn on_mode_change(&mut self, mode: &str) {
        match mode {
            "normal" => self.mode = NvimMode::Normal,
            "insert" => self.mode = NvimMode::Insert,
            _ => self.mode = NvimMode::Other,
        }
    }

    fn on_mouse_on(&mut self) {
        self.mouse_enabled = true;
    }

    fn on_mouse_off(&mut self) {
        self.mouse_enabled = false;
    }
}

impl GuiApi for Shell {
    fn set_font(&mut self, font_desc: &str) {
        self.set_font_desc(font_desc);

        SET.with(|settings| {
            let mut settings = settings.borrow_mut();
            settings.set_font_source(settings::FontSource::Rpc);
        });
    }
}

