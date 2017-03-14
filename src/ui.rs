use std::cell::RefCell;
use std::thread;
use std::string::String;

use cairo;
use pangocairo as pc;
use pango;
use pango::FontDescription;
use gtk;
use gtk::prelude::*;
use gtk::{ApplicationWindow, HeaderBar, DrawingArea, ToolButton, Image};
use gtk_sys;
use gdk::{ModifierType, Event, EventKey, EventConfigure, EventButton, EventMotion, EventType};
use gdk_sys;
use glib;
use neovim_lib::{Neovim, NeovimApi, Value, Integer};

use ui_model::{UiModel, Cell, Attrs, Color, COLOR_BLACK, COLOR_WHITE, COLOR_RED};
use nvim::{RedrawEvents, GuiApi, ErrorReport};
use settings;

use input::{convert_key, keyval_to_input_string};

const FONT_NAME: &'static str = "DejaVu Sans Mono 12";

macro_rules! ui_thread_var {
    ($id:ident, $ty:ty, $expr:expr) => (thread_local!(pub static $id: RefCell<$ty> = {
        let thread = thread::current();
        let current_thread_name = thread.name();
        if current_thread_name != Some("main") {
            panic!("Can create UI  only from main thread, {:?}", current_thread_name);
        }
        RefCell::new($expr)
    });)
}

ui_thread_var![UI, Ui, Ui::new()];
ui_thread_var![SET, settings::Settings, settings::Settings::new()];

#[derive(PartialEq)]
enum NvimMode {
    Normal,
    Insert,
    Other,
}

pub struct Ui {
    pub model: UiModel,
    nvim: Option<Neovim>,
    drawing_area: DrawingArea,
    window: Option<ApplicationWindow>,
    header_bar: HeaderBar,
    cur_attrs: Option<Attrs>,
    bg_color: Color,
    fg_color: Color,
    sp_color: Color,
    line_height: Option<f64>,
    char_width: Option<f64>,
    resize_timer: Option<glib::SourceId>,
    mode: NvimMode,
    mouse_enabled: bool,
    mouse_pressed: bool,
    font_desc: FontDescription,
}

impl Ui {
    pub fn new() -> Ui {
        Ui {
            model: UiModel::empty(),
            drawing_area: DrawingArea::new(),
            window: None,
            header_bar: HeaderBar::new(),
            nvim: None,
            cur_attrs: None,
            bg_color: COLOR_BLACK,
            fg_color: COLOR_WHITE,
            sp_color: COLOR_RED,
            line_height: None,
            char_width: None,
            resize_timer: None,
            mode: NvimMode::Normal,
            mouse_enabled: false,
            mouse_pressed: false,
            font_desc: FontDescription::from_string(FONT_NAME),
        }
    }

    pub fn set_nvim(&mut self, nvim: Neovim) {
        self.nvim = Some(nvim);
    }

    pub fn nvim(&mut self) -> &mut Neovim {
        self.nvim.as_mut().unwrap()
    }

    pub fn destroy(&self) {
        self.window.as_ref().unwrap().destroy();
    }

    pub fn init(&mut self, app: &gtk::Application) {
        SET.with(|settings| {
            let mut settings = settings.borrow_mut();
            settings.init(self);
        });

        self.header_bar.set_show_close_button(true);

        let save_image = Image::new_from_icon_name("document-save",
                                                   gtk_sys::GTK_ICON_SIZE_SMALL_TOOLBAR as i32);
        let save_btn = ToolButton::new(Some(&save_image), None);
        save_btn.connect_clicked(|_| edit_save_all());
        self.header_bar.pack_start(&save_btn);

        let paste_image = Image::new_from_icon_name("edit-paste",
                                                    gtk_sys::GTK_ICON_SIZE_SMALL_TOOLBAR as i32);
        let paste_btn = ToolButton::new(Some(&paste_image), None);
        paste_btn.connect_clicked(|_| edit_paste());
        self.header_bar.pack_start(&paste_btn);


        self.drawing_area.set_size_request(500, 300);
        self.drawing_area.set_hexpand(true);
        self.drawing_area.set_vexpand(true);

        self.drawing_area
            .set_events((gdk_sys::GDK_BUTTON_RELEASE_MASK | gdk_sys::GDK_BUTTON_PRESS_MASK |
                         gdk_sys::GDK_BUTTON_MOTION_MASK)
                .bits() as i32);
        self.drawing_area.connect_button_press_event(gtk_button_press);
        self.drawing_area.connect_button_release_event(gtk_button_release);
        self.drawing_area.connect_motion_notify_event(gtk_motion_notify);
        self.drawing_area.connect_draw(gtk_draw);

        self.window = Some(ApplicationWindow::new(app));
        let window = self.window.as_ref().unwrap();

        window.set_titlebar(Some(&self.header_bar));
        window.add(&self.drawing_area);
        window.show_all();
        window.connect_key_press_event(gtk_key_press);
        window.connect_delete_event(gtk_delete);
        window.set_title("Neovim-gtk");
        self.drawing_area.connect_configure_event(gtk_configure_event);
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

fn gtk_button_press(_: &DrawingArea, ev: &EventButton) -> Inhibit {
    if ev.get_event_type() != EventType::ButtonPress {
        return Inhibit(false);
    }

    UI.with(|ui_cell| {
        let mut ui = ui_cell.borrow_mut();
        if !ui.mouse_enabled {
            return;
        }

        mouse_input(&mut *ui, "LeftMouse", ev.get_state(), ev.get_position());
    });
    Inhibit(false)
}

fn mouse_input(ui: &mut Ui, input: &str, state: ModifierType, position: (f64, f64)) {
    if let Some(line_height) = ui.line_height {
        if let Some(char_width) = ui.char_width {
            ui.mouse_pressed = true;

            let nvim = ui.nvim();
            let (x, y) = position;
            let col = (x / char_width).trunc() as u64;
            let row = (y / line_height).trunc() as u64;
            let input_str = format!("{}<{},{}>", keyval_to_input_string(input, state), col, row);
            nvim.input(&input_str).expect("Can't send mouse input event");
        }
    }
}

fn gtk_button_release(_: &DrawingArea, _: &EventButton) -> Inhibit {
    UI.with(|ui_cell| ui_cell.borrow_mut().mouse_pressed = false);
    Inhibit(false)
}

fn gtk_motion_notify(_: &DrawingArea, ev: &EventMotion) -> Inhibit {
    UI.with(|ui_cell| {
        let mut ui = ui_cell.borrow_mut();
        if !ui.mouse_enabled || !ui.mouse_pressed {
            return;
        }

        mouse_input(&mut *ui, "LeftDrag", ev.get_state(), ev.get_position());
    });
    Inhibit(false)
}

fn edit_paste() {
    UI.with(|ui_cell| {
        let mut ui = ui_cell.borrow_mut();

        let paste_command = if ui.mode == NvimMode::Normal {
            "\"*p"
        } else {
            "<Esc>\"*p"
        };

        let mut nvim = ui.nvim();
        nvim.input(paste_command).report_err(nvim);
    });
}

fn edit_save_all() {
    UI.with(|ui_cell| {
        let mut ui = ui_cell.borrow_mut();

        let mut nvim = ui.nvim();
        nvim.command(":wa").report_err(nvim);
    });
}

fn quit() {
    UI.with(|ui_cell| {
        let mut ui = ui_cell.borrow_mut();
        ui.destroy();

        let nvim = ui.nvim();
        nvim.ui_detach().expect("Error in ui_detach");
        // nvim.quit_no_save().expect("Can't stop nvim instance");
    });
}

fn gtk_delete(_: &ApplicationWindow, _: &Event) -> Inhibit {
    quit();
    Inhibit(false)
}

fn gtk_key_press(_: &ApplicationWindow, ev: &EventKey) -> Inhibit {
    if let Some(input) = convert_key(ev) {
        UI.with(|ui_cell| {
            let mut ui = ui_cell.borrow_mut();
            ui.nvim().input(&input).expect("Error run input command to nvim");
        });
        Inhibit(true)
    } else {
        Inhibit(false)
    }
}

fn gtk_draw(_: &DrawingArea, ctx: &cairo::Context) -> Inhibit {
    UI.with(|ui_cell| {
        let mut ui = ui_cell.borrow_mut();

        let (width, height) = calc_char_bounds(&*ui, ctx);
        ui.line_height = Some(height as f64);
        ui.char_width = Some(width as f64);

        draw(&*ui, ctx);
        request_width(&*ui);
    });

    Inhibit(false)
}

fn gtk_configure_event(_: &DrawingArea, ev: &EventConfigure) -> bool {
    UI.with(|ui_cell| {
        let mut ui = ui_cell.borrow_mut();
        let (width, height) = ev.get_size();

        if let Some(timer) = ui.resize_timer {
            glib::source_remove(timer);
        }
        if let Some(line_height) = ui.line_height {
            if let Some(char_width) = ui.char_width {

                ui.resize_timer = Some(glib::timeout_add(250, move || {
                    UI.with(|ui_cell| {
                        let mut ui = ui_cell.borrow_mut();
                        ui.resize_timer = None;

                        let rows = (height as f64 / line_height).trunc() as usize;
                        let columns = (width as f64 / char_width).trunc() as usize;
                        if ui.model.rows != rows || ui.model.columns != columns {
                            if let Err(err) = ui.nvim().ui_try_resize(columns as u64, rows as u64) {
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

#[inline]
fn draw_joined_rect(ui: &Ui,
                    ctx: &cairo::Context,
                    from_col_idx: usize,
                    col_idx: usize,
                    char_width: f64,
                    line_height: f64,
                    color: &Color) {
    let current_point = ctx.get_current_point();
    let rect_width = char_width * (col_idx - from_col_idx) as f64;

    if &ui.bg_color != color {
        ctx.set_source_rgb(color.0, color.1, color.2);
        ctx.rectangle(current_point.0, current_point.1, rect_width, line_height);
        ctx.fill();
    }

    ctx.move_to(current_point.0 + rect_width, current_point.1);
}

fn draw(ui: &Ui, ctx: &cairo::Context) {
    ctx.set_source_rgb(ui.bg_color.0, ui.bg_color.1, ui.bg_color.2);
    ctx.paint();

    let line_height = ui.line_height.unwrap();
    let char_width = ui.char_width.unwrap();
    let (row, col) = ui.model.get_cursor();
    let mut buf = String::with_capacity(4);

    let mut line_y: f64 = 0.0;


    let layout = pc::create_layout(ctx);
    let mut desc = ui.create_pango_font();

    for (line_idx, line) in ui.model.model().iter().enumerate() {
        ctx.move_to(0.0, line_y);

        // first draw background
        // here we join same bg color for given line
        // this gives less drawing primitives
        let mut from_col_idx = 0;
        let mut from_bg = None;
        for (col_idx, cell) in line.iter().enumerate() {
            let (bg, _) = ui.colors(cell);

            if from_bg.is_none() {
                from_bg = Some(bg);
                from_col_idx = col_idx;
            } else if from_bg != Some(bg) {
                draw_joined_rect(ui,
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
        draw_joined_rect(ui,
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

            let (bg, fg) = ui.colors(cell);

            if row == line_idx && col == col_idx {
                ctx.set_source_rgba(1.0 - bg.0, 1.0 - bg.1, 1.0 - bg.2, 0.5);

                let cursor_width = if ui.mode == NvimMode::Insert {
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
                    &ui.sp_color
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

fn calc_char_bounds(ui: &Ui, ctx: &cairo::Context) -> (i32, i32) {
    let layout = pc::create_layout(ctx);

    let desc = ui.create_pango_font();
    layout.set_font_description(Some(&desc));
    layout.set_text("A", -1);

    layout.get_pixel_size()
}

fn request_width(ui: &Ui) {
    if ui.resize_timer.is_some() {
        return;
    }

    let width = ui.drawing_area.get_allocated_width();
    let height = ui.drawing_area.get_allocated_height();
    let request_height = (ui.model.rows as f64 * ui.line_height.unwrap()) as i32;
    let request_width = (ui.model.columns as f64 * ui.char_width.unwrap()) as i32;

    if width != request_width || height != request_height {
        let window = ui.window.as_ref().unwrap();
        let (win_width, win_height) = window.get_size();
        let h_border = win_width - width;
        let v_border = win_height - height;
        window.resize(request_width + h_border, request_height + v_border);
    }
}

impl GuiApi for Ui {
    fn set_font(&mut self, font_desc: &str) {
        self.set_font_desc(font_desc);

        SET.with(|settings| {
            let mut settings = settings.borrow_mut();
            settings.set_font_source(settings::FontSource::Rpc);
        });
    }
}

impl RedrawEvents for Ui {
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

fn split_color(indexed_color: u64) -> Color {
    let r = ((indexed_color >> 16) & 0xff) as f64;
    let g = ((indexed_color >> 8) & 0xff) as f64;
    let b = (indexed_color & 0xff) as f64;
    Color(r / 255.0, g / 255.0, b / 255.0)
}
