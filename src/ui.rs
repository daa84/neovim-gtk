use std::cell::RefCell;
use std::thread;
use std::collections::HashMap;
use std::string::String;

use cairo;
use pangocairo as pc;
use pango;
use pango::FontDescription;
use gtk;
use gtk::prelude::*;
use gtk::{ApplicationWindow, DrawingArea, Grid, ToolButton, Image, Toolbar, IconSize};
use gdk::{ModifierType, Event, EventKey, EventConfigure, EventButton, EventMotion, EventType};
use gdk_sys;
use glib;
use neovim_lib::{Neovim, NeovimApi, Value, Integer};

use ui_model::{UiModel, Attrs, Color, COLOR_BLACK, COLOR_WHITE};
use nvim::RedrawEvents;

use input::{convert_key, keyval_to_input_string};

#[cfg(target_os = "linux")]
const FONT_NAME: &'static str = "Droid Sans Mono for Powerline 12";
#[cfg(target_os = "windows")]
const FONT_NAME: &'static str = "DejaVu Sans Mono 12";

thread_local!(pub static UI: RefCell<Ui> = {
    let thread = thread::current();
    let current_thread_name = thread.name();
    if current_thread_name != Some("main") {
        panic!("Can create UI  only from main thread, {:?}", current_thread_name);
    }
    RefCell::new(Ui::new())
});

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
    cur_attrs: Option<Attrs>,
    bg_color: Color,
    fg_color: Color,
    line_height: Option<f64>,
    char_width: Option<f64>,
    resize_timer: Option<glib::SourceId>,
    mode: NvimMode,
    mouse_enabled: bool,
    mouse_pressed: bool,
}

impl Ui {
    pub fn new() -> Ui {
        Ui {
            model: UiModel::empty(),
            drawing_area: DrawingArea::new(),
            window: None,
            nvim: None,
            cur_attrs: None,
            bg_color: COLOR_BLACK,
            fg_color: COLOR_WHITE,
            line_height: None,
            char_width: None,
            resize_timer: None,
            mode: NvimMode::Normal,
            mouse_enabled: false,
            mouse_pressed: false,
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
        let grid = Grid::new();

        let button_bar = Toolbar::new();
        button_bar.set_icon_size(IconSize::SmallToolbar);
        button_bar.set_hexpand(true);

        let open_image = Image::new_from_icon_name("document-open", 50);
        let open_btn = ToolButton::new(Some(&open_image), None);
        button_bar.add(&open_btn);

        let save_image = Image::new_from_icon_name("document-save", 50);
        let save_btn = ToolButton::new(Some(&save_image), None);
        button_bar.add(&save_btn);

        let exit_image = Image::new_from_icon_name("application-exit", 50);
        let exit_btn = ToolButton::new(Some(&exit_image), None);
        exit_btn.connect_clicked(|_| quit());
        button_bar.add(&exit_btn);

        grid.attach(&button_bar, 0, 0, 1, 1);

        self.drawing_area.set_size_request(500, 300);
        self.drawing_area.set_hexpand(true);
        self.drawing_area.set_vexpand(true);

        grid.attach(&self.drawing_area, 0, 1, 1, 1);

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

        window.add(&grid);
        window.show_all();
        window.connect_key_press_event(gtk_key_press);
        window.connect_delete_event(gtk_delete);
        window.set_title("Neovim-gtk");
        self.drawing_area.connect_configure_event(gtk_configure_event);
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

fn quit() {
    UI.with(|ui_cell| {
        let mut ui = ui_cell.borrow_mut();
        ui.destroy();

        let nvim = ui.nvim();
        nvim.ui_detach().expect("Error in ui_detach");
        //nvim.quit_no_save().expect("Can't stop nvim instance");
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

        let (width, height) = calc_char_bounds(ctx);
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

fn font_description() -> FontDescription {
    FontDescription::from_string(FONT_NAME)
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

    for (line_idx, line) in ui.model.model().iter().enumerate() {
        ctx.move_to(0.0, line_y);
        for (col_idx, cell) in line.iter().enumerate() {

            let current_point = ctx.get_current_point();

            if let Some(ref bg) = cell.attrs.background {
                ctx.set_source_rgb(bg.0, bg.1, bg.2);
                ctx.rectangle(current_point.0, line_y, char_width, line_height);
                ctx.fill();

                ctx.move_to(current_point.0, current_point.1);
            }

            let fg = if let Some(ref fg) = cell.attrs.foreground {
                fg
            } else {
                &ui.fg_color
            };

            let bg = if let Some(ref bg) = cell.attrs.background {
                bg
            } else {
                &ui.bg_color
            };

            if row == line_idx && col == col_idx {
                ctx.set_source_rgba(1.0 - bg.0, 1.0 - bg.1, 1.0 - bg.2, 0.5);

                let cursor_width = if ui.mode == NvimMode::Insert {
                    char_width / 5.0
                } else {
                    char_width
                };

                ctx.rectangle(current_point.0, line_y, cursor_width, line_height);
                ctx.fill();
                ctx.move_to(current_point.0, current_point.1);
            }

            if !cell.ch.is_whitespace() {
                let mut desc = font_description();
                if cell.attrs.italic {
                    desc.set_style(pango::Style::Italic);
                }
                if cell.attrs.bold {
                    desc.set_weight(pango::Weight::Bold);
                }

                layout.set_font_description(Some(&desc));
                buf.clear();
                buf.push(cell.ch);
                layout.set_text(&buf, -1);

                ctx.set_source_rgb(fg.0, fg.1, fg.2);
                pc::update_layout(ctx, &layout);
                pc::show_layout(ctx, &layout);
            }

            ctx.move_to(current_point.0 + char_width, current_point.1);
        }

        line_y += line_height;
    }


}

fn calc_char_bounds(ctx: &cairo::Context) -> (i32, i32) {
    let layout = pc::create_layout(ctx);

    let desc = font_description();
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

impl RedrawEvents for Ui {
    fn on_cursor_goto(&mut self, row: u64, col: u64) {
        self.model.set_cursor(row, col);
    }

    fn on_put(&mut self, text: &str) {
        self.model.put(text, &self.cur_attrs);
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

    fn on_highlight_set(&mut self, attrs: &HashMap<String, Value>) {
        let mut model_attrs = Attrs::new();
        if let Some(&Value::Integer(Integer::U64(fg))) = attrs.get("foreground") {
            model_attrs.foreground = Some(split_color(fg));
        }
        if let Some(&Value::Integer(Integer::U64(bg))) = attrs.get("background") {
            model_attrs.background = Some(split_color(bg));
        }
        if attrs.contains_key("reverse") {
            let fg = if let Some(ref fg) = model_attrs.foreground {
                fg.clone()
            } else {
                self.fg_color.clone()
            };
            let bg = if let Some(ref bg) = model_attrs.background {
                bg.clone()
            } else {
                self.bg_color.clone()
            };
            model_attrs.foreground = Some(bg);
            model_attrs.background = Some(fg);
        }
        model_attrs.bold = attrs.contains_key("bold");
        model_attrs.italic = attrs.contains_key("italic");
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
