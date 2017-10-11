use std::cell::{RefMut, RefCell, Cell};
use std::rc::Rc;
use std::sync::Arc;
use std::ops::Deref;
use std::thread;

use cairo;
use pangocairo::CairoContextExt;
use pango::{LayoutExt, FontDescription};
use gdk;
use gdk::{ModifierType, EventButton, EventMotion, EventType, EventScroll};
use gdk_sys;
use glib;
use gtk;
use gtk::prelude::*;

use neovim_lib::{Neovim, NeovimApi, Value};
use neovim_lib::neovim_api::Tabpage;

use settings::{Settings, FontSource};
use ui_model::{UiModel, Attrs, ModelRect};
use color::{ColorModel, Color, COLOR_BLACK, COLOR_WHITE, COLOR_RED};
use nvim;
use nvim::{RedrawEvents, GuiApi, RepaintMode, ErrorReport, NeovimClient};
use input;
use input::keyval_to_input_string;
use cursor::Cursor;
use ui::UiMutex;
use popup_menu::PopupMenu;
use tabline::Tabline;
use error;
use mode;
use render;
use render::CellMetrics;

const DEFAULT_FONT_NAME: &str = "DejaVu Sans Mono 12";
pub const MINIMUM_SUPPORTED_NVIM_VERSION: &str = "0.2";

macro_rules! idle_cb_call {
    ($state:ident.$cb:ident($( $x:expr ),*)) => (
            glib::idle_add(move || {
                               if let Some(ref cb) = $state.borrow().$cb {
                                   (&mut *cb.borrow_mut())($($x),*);
                               }

                               glib::Continue(false)
                           });
    )
}

#[derive(Debug, Clone, Copy)]
enum ResizeState {
    NvimResizeTimer(glib::SourceId, usize, usize),
    NvimResizeRequest(usize, usize),
    Wait,
}

pub struct State {
    pub model: UiModel,
    color_model: ColorModel,
    cur_attrs: Option<Attrs>,
    mouse_enabled: bool,
    nvim: Rc<RefCell<NeovimClient>>,
    pub font_ctx: render::Context,
    cursor: Option<Cursor>,
    popup_menu: RefCell<PopupMenu>,
    settings: Rc<RefCell<Settings>>,

    pub mode: mode::Mode,

    stack: gtk::Stack,
    drawing_area: gtk::DrawingArea,
    tabs: Tabline,
    im_context: gtk::IMMulticontext,
    error_area: error::ErrorArea,

    resize_state: Rc<Cell<ResizeState>>,

    options: ShellOptions,

    detach_cb: Option<Box<RefCell<FnMut() + Send + 'static>>>,
}

impl State {
    pub fn new(settings: Rc<RefCell<Settings>>, options: ShellOptions) -> State {
        let drawing_area = gtk::DrawingArea::new();
        let popup_menu = RefCell::new(PopupMenu::new(&drawing_area));
        let font_ctx = render::Context::new(FontDescription::from_string(DEFAULT_FONT_NAME));

        State {
            model: UiModel::empty(),
            color_model: ColorModel::new(),
            nvim: Rc::new(RefCell::new(NeovimClient::new())),
            cur_attrs: None,
            mouse_enabled: true,
            font_ctx,
            cursor: None,
            popup_menu,
            settings,

            mode: mode::Mode::new(),

            // UI
            stack: gtk::Stack::new(),
            drawing_area,
            tabs: Tabline::new(),
            im_context: gtk::IMMulticontext::new(),
            error_area: error::ErrorArea::new(),

            resize_state: Rc::new(Cell::new(ResizeState::Wait)),

            options,

            detach_cb: None,
        }
    }

    pub fn get_foreground(&self) -> &Color {
        &self.color_model.fg_color
    }

    pub fn get_background(&self) -> &Color {
        &self.color_model.bg_color
    }

    pub fn nvim(&self) -> Option<RefMut<Neovim>> {
        if self.nvim.borrow().is_initialized() {
            Some(RefMut::map(self.nvim.borrow_mut(), |n| n.nvim_mut()))
        } else {
            None
        }
    }

    pub fn nvim_clone(&self) -> Rc<RefCell<NeovimClient>> {
        self.nvim.clone()
    }

    pub fn start_nvim_initialization(&self) -> bool {
        let mut nvim = self.nvim.borrow_mut();
        if nvim.is_uninitialized() {
            nvim.set_in_progress();
            true
        } else {
            false
        }
    }

    pub fn set_detach_cb<F>(&mut self, cb: Option<F>)
    where
        F: FnMut() + Send + 'static,
    {
        if cb.is_some() {
            self.detach_cb = Some(Box::new(RefCell::new(cb.unwrap())));
        } else {
            self.detach_cb = None;
        }
    }

    pub fn get_font_desc(&self) -> &FontDescription {
        self.font_ctx.font_description()
    }

    pub fn set_font_desc(&mut self, desc: &str) {
        self.font_ctx.update(FontDescription::from_string(desc));
        self.model.clear_glyphs();
        self.try_nvim_resize();
        self.on_redraw(&RepaintMode::All);
    }

    pub fn open_file(&self, path: &str) {
        if let Some(mut nvim) = self.nvim() {
            nvim.command(&format!("e {}", path)).report_err(&mut *nvim);
        }
    }

    pub fn cd(&self, path: &str) {
        if let Some(mut nvim) = self.nvim() {
            nvim.command(&format!("cd {}", path)).report_err(&mut *nvim);
        }
    }

    fn close_popup_menu(&self) {
        if self.popup_menu.borrow().is_open() {
            if let Some(mut nvim) = self.nvim() {
                nvim.input("<Esc>").report_err(&mut *nvim);
            }
        }
    }

    fn queue_draw_area<M: AsRef<ModelRect>>(&mut self, rect_list: &[M]) {
        // extends by items before, then after changes

        let rects: Vec<_> = rect_list
            .iter()
            .map(|rect| rect.as_ref().clone())
            .map(|mut rect| {
                rect.extend_by_items(&self.model);
                rect
            })
            .collect();

        self.update_dirty_glyphs();

        for mut rect in rects {
            rect.extend_by_items(&self.model);

            let (x, y, width, height) =
                rect.to_area_extend_ink(&self.model, self.font_ctx.cell_metrics());
            self.drawing_area.queue_draw_area(x, y, width, height);
        }
    }

    #[inline]
    fn update_dirty_glyphs(&mut self) {
        render::shape_dirty(&self.font_ctx, &mut self.model, &self.color_model);
    }

    fn im_commit(&self, ch: &str) {
        input::im_input(&mut self.nvim.borrow_mut(), ch);
    }

    fn calc_nvim_size(&self) -> (usize, usize) {
        let &CellMetrics {
            line_height,
            char_width,
            ..
        } = self.font_ctx.cell_metrics();
        let alloc = self.drawing_area.get_allocation();
        (
            (alloc.width as f64 / char_width).trunc() as usize,
            (alloc.height as f64 / line_height).trunc() as usize,
        )
    }

    fn show_error_area(&self) {
        let stack = self.stack.clone();
        gtk::idle_add(move || {
            stack.set_visible_child_name("Error");
            Continue(false)
        });
    }

    fn set_im_location(&self) {
        let (row, col) = self.model.get_cursor();

        let (x, y, width, height) =
            ModelRect::point(col, row).to_area(self.font_ctx.cell_metrics());

        self.im_context.set_cursor_location(&gdk::Rectangle {
            x,
            y,
            width,
            height,
        });

        self.im_context.reset();
    }

    fn try_nvim_resize(&self) {
        if !self.nvim.borrow().is_initialized() {
            return;
        }

        let (columns, rows) = self.calc_nvim_size();


        match self.resize_state.get() {
            ResizeState::NvimResizeTimer(timer, req_columns, req_rows) => {
                if req_columns == columns && req_rows == rows {
                    return;
                }
                glib::source_remove(timer);
            }
            ResizeState::NvimResizeRequest(req_columns, req_rows) => {
                if req_columns == columns && req_rows == rows {
                    return;
                }
            }
            ResizeState::Wait => (),
        }


        let resize_state = self.resize_state.clone();
        let nvim = self.nvim.clone();


        if self.model.rows == rows && self.model.columns == columns {
            return;
        }

        self.resize_state.set(ResizeState::NvimResizeTimer(
            gtk::timeout_add(250, move || {
                resize_state.set(ResizeState::NvimResizeRequest(columns, rows));

                let mut nvim = nvim.borrow_mut();
                if let Err(err) = nvim.ui_try_resize(columns as u64, rows as u64) {
                    error!("Error trying resize nvim {}", err);
                }
                Continue(false)
            }),
            columns,
            rows,
        ));

    }

    fn resize_main_window(&mut self) {
        let &CellMetrics {
            line_height,
            char_width,
            ..
        } = self.font_ctx.cell_metrics();

        let width = self.drawing_area.get_allocated_width();
        let height = self.drawing_area.get_allocated_height();
        let request_height = (self.model.rows as f64 * line_height) as i32;
        let request_width = (self.model.columns as f64 * char_width) as i32;

        if width != request_width || height != request_height {
            let window: gtk::Window = self.drawing_area
                .get_toplevel()
                .unwrap()
                .downcast()
                .unwrap();
            let (win_width, win_height) = window.get_size();
            let h_border = win_width - width;
            let v_border = win_height - height;
            window.resize(request_width + h_border, request_height + v_border);
        }
    }
}

pub struct UiState {
    mouse_pressed: bool,
}

impl UiState {
    pub fn new() -> UiState {
        UiState { mouse_pressed: false }
    }
}

#[derive(Clone)]
pub struct ShellOptions {
    nvim_bin_path: Option<String>,
    open_path: Option<String>,
}

impl ShellOptions {
    pub fn new(nvim_bin_path: Option<String>, open_path: Option<String>) -> Self {
        ShellOptions {
            nvim_bin_path,
            open_path,
        }
    }
}

pub struct Shell {
    pub state: Arc<UiMutex<State>>,
    ui_state: Rc<RefCell<UiState>>,

    widget: gtk::Box,
}

impl Shell {
    pub fn new(settings: Rc<RefCell<Settings>>, options: ShellOptions) -> Shell {
        let shell = Shell {
            state: Arc::new(UiMutex::new(State::new(settings, options))),
            ui_state: Rc::new(RefCell::new(UiState::new())),

            widget: gtk::Box::new(gtk::Orientation::Vertical, 0),
        };

        let shell_ref = Arc::downgrade(&shell.state);
        shell.state.borrow_mut().cursor = Some(Cursor::new(shell_ref));

        shell
    }

    pub fn is_nvim_initialized(&self) -> bool {
        let state = self.state.borrow();
        let nvim = state.nvim.borrow();
        nvim.is_initialized()
    }

    pub fn init(&mut self) {
        let state = self.state.borrow();
        state.drawing_area.set_hexpand(true);
        state.drawing_area.set_vexpand(true);
        state.drawing_area.set_can_focus(true);

        state.im_context.set_use_preedit(false);

        let nvim_box = gtk::Box::new(gtk::Orientation::Vertical, 0);

        nvim_box.pack_start(&*state.tabs, false, true, 0);
        nvim_box.pack_start(&state.drawing_area, true, true, 0);

        state.stack.add_named(&nvim_box, "Nvim");
        state.stack.add_named(&*state.error_area, "Error");

        self.widget.pack_start(&state.stack, true, true, 0);

        state.drawing_area.set_events(
            (gdk_sys::GDK_BUTTON_RELEASE_MASK | gdk_sys::GDK_BUTTON_PRESS_MASK |
                 gdk_sys::GDK_BUTTON_MOTION_MASK |
                 gdk_sys::GDK_SCROLL_MASK)
                .bits() as i32,
        );

        let ref_state = self.state.clone();
        let ref_ui_state = self.ui_state.clone();
        state.drawing_area.connect_button_press_event(move |_, ev| {
            gtk_button_press(
                &mut *ref_state.borrow_mut(),
                &mut *ref_ui_state.borrow_mut(),
                ev,
            )
        });

        let ref_ui_state = self.ui_state.clone();
        state.drawing_area.connect_button_release_event(
            move |_, _| {
                gtk_button_release(&mut *ref_ui_state.borrow_mut())
            },
        );


        let ref_state = self.state.clone();
        let ref_ui_state = self.ui_state.clone();
        state.drawing_area.connect_motion_notify_event(
            move |_, ev| {
                gtk_motion_notify(
                    &mut *ref_state.borrow_mut(),
                    &mut *ref_ui_state.borrow_mut(),
                    ev,
                )
            },
        );

        let ref_state = self.state.clone();
        state.drawing_area.connect_draw(
            move |_, ctx| gtk_draw(&ref_state, ctx),
        );

        let ref_state = self.state.clone();
        state.drawing_area.connect_key_press_event(move |_, ev| {
            let mut shell = ref_state.borrow_mut();
            shell.cursor.as_mut().unwrap().reset_state();
            // GtkIMContext will eat a Shift-Space and not tell us about shift.
            // Also don't let IME eat any GDK_KEY_KP_ events
            if !ev.get_state().contains(gdk::SHIFT_MASK) &&
                ev.get_keyval() < gdk_sys::GDK_KEY_KP_Space as u32 &&
                ev.get_keyval() > gdk_sys::GDK_KEY_KP_Divide as u32 &&
                shell.im_context.filter_keypress(ev)
            {
                Inhibit(true)
            } else {
                if shell.nvim.borrow().is_initialized() {
                    input::gtk_key_press(&mut shell.nvim.borrow_mut(), ev)
                } else {
                    Inhibit(false)
                }
            }
        });
        let ref_state = self.state.clone();
        state.drawing_area.connect_key_release_event(move |_, ev| {
            ref_state.borrow().im_context.filter_keypress(ev);
            Inhibit(false)
        });

        let ref_state = self.state.clone();
        state.drawing_area.connect_scroll_event(move |_, ev| {
            gtk_scroll_event(&mut *ref_state.borrow_mut(), ev)
        });

        let ref_state = self.state.clone();
        state.drawing_area.connect_focus_in_event(move |_, _| {
            gtk_focus_in(&mut *ref_state.borrow_mut())
        });

        let ref_state = self.state.clone();
        state.drawing_area.connect_focus_out_event(move |_, _| {
            gtk_focus_out(&mut *ref_state.borrow_mut())
        });

        let ref_state = self.state.clone();
        state.drawing_area.connect_realize(move |w| {
            let ref_state = ref_state.clone();
            let w = w.clone();
            // sometime set_client_window does not work without idle_add
            // and looks like not enabled im_context
            gtk::idle_add(move || {
                ref_state.borrow().im_context.set_client_window(
                    w.get_window().as_ref(),
                );
                Continue(false)
            });
        });

        let ref_state = self.state.clone();
        state.im_context.connect_commit(move |_, ch| {
            ref_state.borrow().im_commit(ch)
        });

        let ref_state = self.state.clone();
        state.drawing_area.connect_configure_event(move |_, _| {
            ref_state.borrow().try_nvim_resize();
            false
        });

        let ref_state = self.state.clone();
        state.drawing_area.connect_size_allocate(
            move |_, _| init_nvim(&ref_state),
        );

    }

    #[cfg(unix)]
    pub fn set_font_desc(&self, font_name: &str) {
        self.state.borrow_mut().set_font_desc(font_name);
    }

    pub fn grab_focus(&self) {
        self.state.borrow().drawing_area.grab_focus();
    }

    pub fn open_file(&self, path: &str) {
        self.state.borrow().open_file(path);
    }

    pub fn cd(&self, path: &str) {
        self.state.borrow().cd(path);
    }

    pub fn detach_ui(&mut self) {
        let state = self.state.borrow();

        let nvim = state.nvim();
        if let Some(mut nvim) = nvim {
            nvim.ui_detach().expect("Error in ui_detach");
        }
    }

    pub fn edit_paste(&self) {
        let state = self.state.borrow();
        let paste_command = if state.mode.is(&mode::NvimMode::Normal) {
            "\"*p"
        } else {
            "<Esc>\"*pa"
        };

        let nvim = state.nvim();
        if let Some(mut nvim) = nvim {
            nvim.input(paste_command).report_err(&mut *nvim);
        }
    }

    pub fn edit_save_all(&self) {
        let state = self.state.borrow();

        let nvim = state.nvim();
        if let Some(mut nvim) = nvim {
            nvim.command(":wa").report_err(&mut *nvim);
        }
    }

    pub fn set_detach_cb<F>(&self, cb: Option<F>)
    where
        F: FnMut() + Send + 'static,
    {
        let mut state = self.state.borrow_mut();
        state.set_detach_cb(cb);
    }
}

impl Deref for Shell {
    type Target = gtk::Box;

    fn deref(&self) -> &gtk::Box {
        &self.widget
    }
}

fn gtk_focus_in(state: &mut State) -> Inhibit {
    state.im_context.focus_in();
    state.cursor.as_mut().unwrap().enter_focus();
    let point = state.model.cur_point();
    state.on_redraw(&RepaintMode::Area(point));
    Inhibit(false)
}

fn gtk_focus_out(state: &mut State) -> Inhibit {
    state.im_context.focus_out();
    state.cursor.as_mut().unwrap().leave_focus();
    let point = state.model.cur_point();
    state.on_redraw(&RepaintMode::Area(point));

    Inhibit(false)
}

fn gtk_scroll_event(state: &mut State, ev: &EventScroll) -> Inhibit {
    if !state.mouse_enabled {
        return Inhibit(false);
    }

    state.close_popup_menu();

    match ev.as_ref().direction {
        gdk_sys::GdkScrollDirection::Right => {
            mouse_input(state, "ScrollWheelRight", ev.get_state(), ev.get_position())
        }
        gdk_sys::GdkScrollDirection::Left => {
            mouse_input(state, "ScrollWheelLeft", ev.get_state(), ev.get_position())
        }
        gdk_sys::GdkScrollDirection::Up => {
            mouse_input(state, "ScrollWheelUp", ev.get_state(), ev.get_position())
        }
        gdk_sys::GdkScrollDirection::Down => {
            mouse_input(state, "ScrollWheelDown", ev.get_state(), ev.get_position())
        }
        _ => (),
    }
    Inhibit(false)
}

fn gtk_button_press(shell: &mut State, ui_state: &mut UiState, ev: &EventButton) -> Inhibit {
    if ev.get_event_type() != EventType::ButtonPress {
        return Inhibit(false);
    }

    if shell.mouse_enabled {
        ui_state.mouse_pressed = true;

        mouse_input(shell, "LeftMouse", ev.get_state(), ev.get_position());
    }
    Inhibit(false)
}

fn mouse_input(shell: &mut State, input: &str, state: ModifierType, position: (f64, f64)) {
    let &CellMetrics {
        line_height,
        char_width,
        ..
    } = shell.font_ctx.cell_metrics();
    let (x, y) = position;
    let col = (x / char_width).trunc() as u64;
    let row = (y / line_height).trunc() as u64;
    let input_str = format!("{}<{},{}>", keyval_to_input_string(input, state), col, row);

    let nvim = shell.nvim();
    if let Some(mut nvim) = nvim {
        nvim.input(&input_str).expect(
            "Can't send mouse input event",
        );
    }
}

fn gtk_button_release(ui_state: &mut UiState) -> Inhibit {
    ui_state.mouse_pressed = false;
    Inhibit(false)
}

fn gtk_motion_notify(shell: &mut State, ui_state: &mut UiState, ev: &EventMotion) -> Inhibit {
    if shell.mouse_enabled && ui_state.mouse_pressed {
        mouse_input(shell, "LeftDrag", ev.get_state(), ev.get_position());
    }
    Inhibit(false)
}

fn gtk_draw(state_arc: &Arc<UiMutex<State>>, ctx: &cairo::Context) -> Inhibit {

    let state = state_arc.borrow();
    if state.nvim.borrow().is_initialized() {
        render::render(
            ctx,
            state.cursor.as_ref().unwrap(),
            &state.font_ctx,
            &state.model,
            &state.color_model,
            &state.mode,
        );
    } else if state.nvim.borrow().is_initializing() {
        draw_initializing(&*state, ctx);
    }

    Inhibit(false)
}

fn show_nvim_start_error(err: &nvim::NvimInitError, state_arc: Arc<UiMutex<State>>) {
    let source = err.source();
    let cmd = err.cmd().unwrap().to_owned();

    glib::idle_add(move || {
        let state = state_arc.borrow();
        state.nvim.borrow_mut().set_error();
        state.error_area.show_nvim_start_error(&source, &cmd);
        state.show_error_area();

        Continue(false)
    });
}

fn show_nvim_init_error(err: &nvim::NvimInitError, state_arc: Arc<UiMutex<State>>) {
    let source = err.source();

    glib::idle_add(move || {
        let state = state_arc.borrow();
        state.nvim.borrow_mut().set_error();
        state.error_area.show_nvim_init_error(&source);
        state.show_error_area();

        Continue(false)
    });
}

fn init_nvim_async(
    state_arc: Arc<UiMutex<State>>,
    options: ShellOptions,
    cols: usize,
    rows: usize,
) {
    // execute nvim
    let mut nvim = match nvim::start(state_arc.clone(), options.nvim_bin_path.as_ref()) {
        Ok(nvim) => nvim,
        Err(err) => {
            show_nvim_start_error(&err, state_arc);
            return;
        }
    };

    // add callback on session end
    let guard = nvim.session.take_dispatch_guard();
    let state_ref = state_arc.clone();
    thread::spawn(move || {
        guard.join().expect("Can't join dispatch thread");

        idle_cb_call!(state_ref.detach_cb());
    });

    // attach ui
    let mut nvim = Some(nvim);
    glib::idle_add(move || {
        let mut nvim = nvim.take().unwrap();
        if let Err(err) = nvim::post_start_init(
            &mut nvim,
            options.open_path.as_ref(),
            cols as u64,
            rows as u64,
        )
        {
            show_nvim_init_error(&err, state_arc.clone());
        } else {
            let mut state = state_arc.borrow_mut();
            state.nvim.borrow_mut().set_initialized(nvim);
            state.cursor.as_mut().unwrap().start();
        }

        Continue(false)
    });
}

fn draw_initializing(state: &State, ctx: &cairo::Context) {
    let layout = ctx.create_pango_layout();
    let desc = state.get_font_desc();
    let alloc = state.drawing_area.get_allocation();

    ctx.set_source_rgb(
        state.color_model.bg_color.0,
        state.color_model.bg_color.1,
        state.color_model.bg_color.2,
    );
    ctx.paint();

    layout.set_font_description(desc);
    layout.set_text("Loading->");
    let (width, height) = layout.get_pixel_size();

    let x = alloc.width as f64 / 2.0 - width as f64 / 2.0;
    let y = alloc.height as f64 / 2.0 - height as f64 / 2.0;

    ctx.move_to(x, y);
    ctx.set_source_rgb(
        state.color_model.fg_color.0,
        state.color_model.fg_color.1,
        state.color_model.fg_color.2,
    );
    ctx.update_pango_layout(&layout);
    ctx.show_pango_layout(&layout);


    ctx.move_to(x + width as f64, y);
    state.cursor.as_ref().unwrap().draw(
        ctx,
        &state.font_ctx,
        &state.mode,
        y,
        false,
        &state.color_model.bg_color,
    );
}

fn init_nvim(state_ref: &Arc<UiMutex<State>>) {
    let mut state = state_ref.borrow_mut();
    if state.start_nvim_initialization() {
        let (cols, rows) = state.calc_nvim_size();
        state.model = UiModel::new(rows as u64, cols as u64);
        state.resize_state.set(
            ResizeState::NvimResizeRequest(cols, rows),
        );

        let state_arc = state_ref.clone();
        let options = state.options.clone();
        thread::spawn(move || init_nvim_async(state_arc, options, cols, rows));
    }
}


impl RedrawEvents for State {
    fn on_cursor_goto(&mut self, row: u64, col: u64) -> RepaintMode {
        let repaint_area = self.model.set_cursor(row as usize, col as usize);
        self.set_im_location();
        RepaintMode::AreaList(repaint_area)
    }

    fn on_put(&mut self, text: &str) -> RepaintMode {
        RepaintMode::Area(self.model.put(text, self.cur_attrs.as_ref()))
    }

    fn on_clear(&mut self) -> RepaintMode {
        self.model.clear();
        RepaintMode::All
    }

    fn on_eol_clear(&mut self) -> RepaintMode {
        RepaintMode::Area(self.model.eol_clear())
    }

    fn on_resize(&mut self, columns: u64, rows: u64) -> RepaintMode {
        match self.resize_state.get() {
            ResizeState::NvimResizeTimer(..) => {
                self.model = UiModel::new(rows, columns);
            }
            ResizeState::Wait |
            ResizeState::NvimResizeRequest(..) => {
                if self.model.columns != columns as usize || self.model.rows != rows as usize {
                    self.resize_state.set(ResizeState::Wait);
                    self.model = UiModel::new(rows, columns);
                    self.resize_main_window();
                }
            }
        }
        RepaintMode::Nothing
    }

    fn on_redraw(&mut self, mode: &RepaintMode) {
        match *mode {
            RepaintMode::All => {
                self.update_dirty_glyphs();
                self.drawing_area.queue_draw();
            }
            RepaintMode::Area(ref rect) => self.queue_draw_area(&[rect]),
            RepaintMode::AreaList(ref list) => self.queue_draw_area(&list.list),
            RepaintMode::Nothing => (),
        }
    }

    fn on_set_scroll_region(&mut self, top: u64, bot: u64, left: u64, right: u64) -> RepaintMode {
        self.model.set_scroll_region(top, bot, left, right);
        RepaintMode::Nothing
    }

    fn on_scroll(&mut self, count: i64) -> RepaintMode {
        RepaintMode::Area(self.model.scroll(count))
    }

    fn on_highlight_set(&mut self, attrs: &[(Value, Value)]) -> RepaintMode {
        let mut model_attrs = Attrs::new();

        for &(ref key_val, ref val) in attrs {
            if let Some(key) = key_val.as_str() {
                match key {
                    "foreground" => {
                        if let Some(fg) = val.as_u64() {
                            model_attrs.foreground = Some(Color::from_indexed_color(fg));
                        }
                    }
                    "background" => {
                        if let Some(bg) = val.as_u64() {
                            model_attrs.background = Some(Color::from_indexed_color(bg));
                        }
                    }
                    "special" => {
                        if let Some(bg) = val.as_u64() {
                            model_attrs.special = Some(Color::from_indexed_color(bg));
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
        RepaintMode::Nothing
    }

    fn on_update_bg(&mut self, bg: i64) -> RepaintMode {
        if bg >= 0 {
            self.color_model.bg_color = Color::from_indexed_color(bg as u64);
        } else {
            self.color_model.bg_color = COLOR_BLACK;
        }
        RepaintMode::Nothing
    }

    fn on_update_fg(&mut self, fg: i64) -> RepaintMode {
        if fg >= 0 {
            self.color_model.fg_color = Color::from_indexed_color(fg as u64);
        } else {
            self.color_model.fg_color = COLOR_WHITE;
        }
        RepaintMode::Nothing
    }

    fn on_update_sp(&mut self, sp: i64) -> RepaintMode {
        if sp >= 0 {
            self.color_model.sp_color = Color::from_indexed_color(sp as u64);
        } else {
            self.color_model.sp_color = COLOR_RED;
        }
        RepaintMode::Nothing
    }

    fn on_mode_change(&mut self, mode: &str, idx: u64) -> RepaintMode {
        self.mode.update(mode, idx as usize);
        RepaintMode::Area(self.model.cur_point())
    }

    fn on_mouse(&mut self, on: bool) -> RepaintMode {
        self.mouse_enabled = on;
        RepaintMode::Nothing
    }

    fn on_busy(&mut self, busy: bool) -> RepaintMode {
        if busy {
            self.cursor.as_mut().unwrap().busy_on();
        } else {
            self.cursor.as_mut().unwrap().busy_off();
        }
        RepaintMode::Area(self.model.cur_point())
    }

    fn popupmenu_show(
        &mut self,
        menu: &[Vec<&str>],
        selected: i64,
        row: u64,
        col: u64,
    ) -> RepaintMode {
        let point = ModelRect::point(col as usize, row as usize);
        let (x, y, width, height) = point.to_area(self.font_ctx.cell_metrics());

        self.popup_menu.borrow_mut().show(
            self,
            menu,
            selected,
            x,
            y,
            width,
            height,
        );

        RepaintMode::Nothing
    }

    fn popupmenu_hide(&mut self) -> RepaintMode {
        self.popup_menu.borrow_mut().hide();
        RepaintMode::Nothing
    }

    fn popupmenu_select(&mut self, selected: i64) -> RepaintMode {
        self.popup_menu.borrow().select(selected);
        RepaintMode::Nothing
    }


    fn tabline_update(
        &mut self,
        selected: Tabpage,
        tabs: Vec<(Tabpage, Option<String>)>,
    ) -> RepaintMode {
        self.tabs.update_tabs(&self.nvim, &selected, &tabs);

        RepaintMode::Nothing
    }

    fn mode_info_set(
        &mut self,
        cursor_style_enabled: bool,
        mode_info: Vec<nvim::ModeInfo>,
    ) -> RepaintMode {
        self.mode.set_info(cursor_style_enabled, mode_info);
        RepaintMode::Nothing
    }
}

impl GuiApi for State {
    fn set_font(&mut self, font_desc: &str) {
        {
            let mut settings = self.settings.borrow_mut();
            settings.set_font_source(FontSource::Rpc);
        }

        self.set_font_desc(font_desc);
    }
}
