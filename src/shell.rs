use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

use cairo;
use gdk;
use gdk::{EventButton, EventMotion, EventScroll, EventType, ModifierType, WindowExt};
use glib;
use gtk;
use gtk::prelude::*;
use pango;
use pango::prelude::*;
use pango::{FontDescription, LayoutExt};
use pangocairo;

use neovim_lib::neovim_api::Tabpage;
use neovim_lib::{Neovim, NeovimApi, NeovimApiAsync, Value};

use color::{Color, ColorModel, COLOR_BLACK, COLOR_RED, COLOR_WHITE};
use misc::{decode_uri, escape_filename, split_at_comma};
use nvim::{
    self, CompleteItem, ErrorReport, NeovimClient, NeovimClientAsync, NeovimRef, NvimHandler,
    RepaintMode,
};
use settings::{FontSource, Settings};
use ui_model::{Attrs, ModelRect, UiModel};

use cmd_line::{CmdLine, CmdLineContext};
use cursor::{BlinkCursor, Cursor, CursorRedrawCb};
use error;
use input;
use input::keyval_to_input_string;
use mode;
use popup_menu::{self, PopupMenu};
use render;
use render::CellMetrics;
use subscriptions::{SubscriptionHandle, Subscriptions};
use tabline::Tabline;
use ui::UiMutex;

const DEFAULT_FONT_NAME: &str = "DejaVu Sans Mono 12";
pub const MINIMUM_SUPPORTED_NVIM_VERSION: &str = "0.2.2";

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

pub struct RenderState {
    pub font_ctx: render::Context,
    pub color_model: ColorModel,
    pub mode: mode::Mode,
}

impl RenderState {
    pub fn new(pango_context: pango::Context) -> Self {
        RenderState {
            font_ctx: render::Context::new(pango_context),
            color_model: ColorModel::new(),
            mode: mode::Mode::new(),
        }
    }
}

pub struct TransparencySettigns {
    background_alpha: f64,
    filled_alpha: f64,
    enabled: bool,
}

impl TransparencySettigns {
    pub fn new() -> Self {
        TransparencySettigns {
            background_alpha: 1.0,
            filled_alpha: 1.0,
            enabled: false,
        }
    }

    fn filled_alpha(&self) -> Option<f64> {
        if self.enabled {
            Some(self.filled_alpha)
        } else {
            None
        }
    }

    fn background_alpha(&self) -> Option<f64> {
        if self.enabled {
            Some(self.background_alpha)
        } else {
            None
        }
    }
}

pub struct State {
    pub model: UiModel,
    cur_attrs: Option<Attrs>,
    mouse_enabled: bool,
    nvim: Rc<NeovimClient>,
    cursor: Option<BlinkCursor<State>>,
    popup_menu: PopupMenu,
    cmd_line: CmdLine,
    settings: Rc<RefCell<Settings>>,
    render_state: Rc<RefCell<RenderState>>,

    resize_request: (i64, i64),
    resize_timer: Rc<Cell<Option<glib::SourceId>>>,

    pub clipboard_clipboard: gtk::Clipboard,
    pub clipboard_primary: gtk::Clipboard,

    stack: gtk::Stack,
    pub drawing_area: gtk::DrawingArea,
    tabs: Tabline,
    im_context: gtk::IMMulticontext,
    update_im_location: bool,
    error_area: error::ErrorArea,

    options: ShellOptions,
    transparency_settings: TransparencySettigns,

    detach_cb: Option<Box<RefCell<FnMut() + Send + 'static>>>,
    nvim_started_cb: Option<Box<RefCell<FnMut() + Send + 'static>>>,
    command_cb: Option<Box<FnMut(&mut State, nvim::NvimCommand) + Send + 'static>>,

    subscriptions: RefCell<Subscriptions>,
}

impl State {
    pub fn new(settings: Rc<RefCell<Settings>>, options: ShellOptions) -> State {
        let drawing_area = gtk::DrawingArea::new();

        let pango_context = drawing_area.create_pango_context().unwrap();
        pango_context.set_font_description(&FontDescription::from_string(DEFAULT_FONT_NAME));

        let render_state = Rc::new(RefCell::new(RenderState::new(pango_context)));
        let popup_menu = PopupMenu::new(&drawing_area);
        let cmd_line = CmdLine::new(&drawing_area, render_state.clone());

        State {
            model: UiModel::empty(),
            nvim: Rc::new(NeovimClient::new()),
            cur_attrs: None,
            mouse_enabled: true,
            cursor: None,
            popup_menu,
            cmd_line,
            settings,
            render_state,

            resize_request: (-1, -1),
            resize_timer: Rc::new(Cell::new(None)),

            clipboard_clipboard: gtk::Clipboard::get(&gdk::Atom::intern("CLIPBOARD")),
            clipboard_primary: gtk::Clipboard::get(&gdk::Atom::intern("PRIMARY")),

            // UI
            stack: gtk::Stack::new(),
            drawing_area,
            tabs: Tabline::new(),
            im_context: gtk::IMMulticontext::new(),
            update_im_location: false,
            error_area: error::ErrorArea::new(),

            options,
            transparency_settings: TransparencySettigns::new(),

            detach_cb: None,
            nvim_started_cb: None,
            command_cb: None,

            subscriptions: RefCell::new(Subscriptions::new()),
        }
    }

    /// Return NeovimRef only if vim in non blocking state
    ///
    /// Note that this call also do neovim api call get_mode
    #[allow(dead_code)]
    pub fn nvim_non_blocked(&self) -> Option<NeovimRef> {
        self.nvim().and_then(NeovimRef::non_blocked)
    }

    pub fn nvim(&self) -> Option<NeovimRef> {
        self.nvim.nvim()
    }

    pub fn try_nvim(&self) -> Option<NeovimRef> {
        self.nvim.try_nvim()
    }

    pub fn nvim_clone(&self) -> Rc<NeovimClient> {
        self.nvim.clone()
    }

    pub fn start_nvim_initialization(&self) -> bool {
        if self.nvim.is_uninitialized() {
            self.nvim.set_in_progress();
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

    pub fn set_nvim_started_cb<F>(&mut self, cb: Option<F>)
    where
        F: FnMut() + Send + 'static,
    {
        if cb.is_some() {
            self.nvim_started_cb = Some(Box::new(RefCell::new(cb.unwrap())));
        } else {
            self.nvim_started_cb = None;
        }
    }

    pub fn set_nvim_command_cb<F>(&mut self, cb: Option<F>)
    where
        F: FnMut(&mut State, nvim::NvimCommand) + Send + 'static,
    {
        if cb.is_some() {
            self.command_cb = Some(Box::new(cb.unwrap()));
        } else {
            self.command_cb = None;
        }
    }

    pub fn set_font_desc(&mut self, desc: &str) {
        let font_description = FontDescription::from_string(desc);

        if font_description.get_size() <= 0 {
            error!("Font size must be > 0");
            return;
        }

        let pango_context = self.drawing_area.create_pango_context().unwrap();
        pango_context.set_font_description(&font_description);

        self.render_state
            .borrow_mut()
            .font_ctx
            .update(pango_context);
        self.model.clear_glyphs();
        self.try_nvim_resize();
        self.on_redraw(&RepaintMode::All);
    }

    pub fn set_font_features(&mut self, font_features: String) {
        let font_features = render::FontFeatures::from(font_features);

        self.render_state
            .borrow_mut()
            .font_ctx
            .update_font_features(font_features);
        self.model.clear_glyphs();
        self.on_redraw(&RepaintMode::All);
    }

    pub fn set_line_space(&mut self, line_space: String) {
        let line_space: i32 = match line_space.parse() {
            Ok(line_space) => line_space,
            Err(e) => {
                error!("Can't convert argument to integer: {}", e.to_string());
                return;
            }
        };

        self.render_state
            .borrow_mut()
            .font_ctx
            .update_line_space(line_space);
        self.model.clear_glyphs();
        self.try_nvim_resize();
        self.on_redraw(&RepaintMode::All);
    }

    /// return true if transparency enabled
    pub fn set_transparency(&mut self, background_alpha: f64, filled_alpha: f64) -> bool {
        if background_alpha < 1.0 || filled_alpha < 1.0 {
            self.transparency_settings.background_alpha = background_alpha;
            self.transparency_settings.filled_alpha = filled_alpha;
            self.transparency_settings.enabled = true;
        } else {
            self.transparency_settings.background_alpha = 1.0;
            self.transparency_settings.filled_alpha = 1.0;
            self.transparency_settings.enabled = false;
        }

        self.on_redraw(&RepaintMode::All);

        self.transparency_settings.enabled
    }

    pub fn open_file(&self, path: &str) {
        if let Some(mut nvim) = self.nvim() {
            nvim.command_async(&format!("e {}", path))
                .cb(|r| r.report_err())
                .call();
        }
    }

    pub fn cd(&self, path: &str) {
        if let Some(mut nvim) = self.nvim() {
            nvim.command_async(&format!("cd {}", path))
                .cb(|r| r.report_err())
                .call();
        }
    }

    pub fn clipboard_clipboard_set(&self, text: &str) {
        self.clipboard_clipboard.set_text(text);
    }

    pub fn clipboard_primary_set(&self, text: &str) {
        self.clipboard_primary.set_text(text);
    }

    fn close_popup_menu(&self) {
        if self.popup_menu.is_open() {
            if let Some(mut nvim) = self.nvim() {
                nvim.input("<Esc>").report_err();
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
            }).collect();

        self.update_dirty_glyphs();

        let render_state = self.render_state.borrow();
        let cell_metrics = render_state.font_ctx.cell_metrics();

        for mut rect in rects {
            rect.extend_by_items(&self.model);

            let (x, y, width, height) = rect.to_area_extend_ink(&self.model, cell_metrics);
            self.drawing_area.queue_draw_area(x, y, width, height);
        }
    }

    #[inline]
    fn update_dirty_glyphs(&mut self) {
        let render_state = self.render_state.borrow();
        render::shape_dirty(
            &render_state.font_ctx,
            &mut self.model,
            &render_state.color_model,
        );
    }

    fn im_commit(&self, ch: &str) {
        if let Some(mut nvim) = self.nvim() {
            input::im_input(&mut nvim, ch);
        }
    }

    fn calc_nvim_size(&self) -> (usize, usize) {
        let &CellMetrics {
            line_height,
            char_width,
            ..
        } = self.render_state.borrow().font_ctx.cell_metrics();
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
            ModelRect::point(col, row).to_area(self.render_state.borrow().font_ctx.cell_metrics());

        self.im_context.set_cursor_location(&gdk::Rectangle {
            x,
            y,
            width,
            height,
        });

        self.im_context.reset();
    }

    fn try_nvim_resize(&mut self) {
        let (columns, rows) = self.calc_nvim_size();

        let (requested_rows, requested_cols) = self.resize_request;

        if requested_rows == rows as i64 && requested_cols == columns as i64 {
            return;
        }

        let resize_timer = self.resize_timer.take();
        if let Some(resize_timer) = resize_timer {
            glib::source_remove(resize_timer);
        }

        self.resize_request = (rows as i64, columns as i64);

        let nvim = self.nvim.clone();
        let resize_timer = self.resize_timer.clone();

        let resize_id = gtk::timeout_add(200, move || {
            if let Some(mut nvim) = nvim.try_nvim() {
                debug!("ui_try_resize {}/{}", columns, rows);
                resize_timer.set(None);

                nvim.ui_try_resize_async(columns as u64, rows as u64)
                    .cb(|r| r.report_err())
                    .call();

                return Continue(false);
            }

            Continue(true)
        });

        self.resize_timer.set(Some(resize_id));
    }

    fn edit_paste(&self, clipboard: &str) {
        let nvim = self.nvim();
        if let Some(mut nvim) = nvim {
            let render_state = self.render_state.borrow();
            if render_state.mode.is(&mode::NvimMode::Insert)
                || render_state.mode.is(&mode::NvimMode::Normal)
            {
                let paste_code = format!("normal! \"{}P", clipboard);
                nvim.command_async(&paste_code)
                    .cb(|r| r.report_err())
                    .call();
            } else {
                let paste_code = format!("<C-r>{}", clipboard);
                nvim.input_async(&paste_code).cb(|r| r.report_err()).call();
            };
        }
    }

    fn edit_copy(&self, clipboard: &str) {
        let nvim = self.nvim();
        if let Some(mut nvim) = nvim {
            let paste_code = format!("normal! \"{}y", clipboard);
            nvim.command_async(&paste_code)
                .cb(|r| r.report_err())
                .call();
        }
    }

    fn max_popup_width(&self) -> i32 {
        self.drawing_area.get_allocated_width() - 20
    }

    pub fn subscribe<F>(&self, event_name: &str, args: &[&str], cb: F) -> SubscriptionHandle
    where
        F: Fn(Vec<String>) + 'static,
    {
        self.subscriptions
            .borrow_mut()
            .subscribe(event_name, args, cb)
    }

    pub fn set_autocmds(&self) {
        self.subscriptions
            .borrow()
            .set_autocmds(&mut self.nvim().unwrap());
    }

    pub fn notify(&self, params: Vec<Value>) -> Result<(), String> {
        self.subscriptions.borrow().notify(params)
    }

    pub fn run_now(&self, handle: &SubscriptionHandle) {
        self.subscriptions
            .borrow()
            .run_now(handle, &mut self.nvim().unwrap());
    }

    pub fn set_font(&mut self, font_desc: String) {
        self.set_font_rpc(&font_desc);
    }

    pub fn set_font_rpc(&mut self, font_desc: &str) {
        {
            let mut settings = self.settings.borrow_mut();
            settings.set_font_source(FontSource::Rpc);
        }

        self.set_font_desc(font_desc);
    }

    pub fn on_command(&mut self, command: nvim::NvimCommand) {
        let mut cb = self.command_cb.take();

        if let Some(ref mut cb) = cb {
            cb(self, command);
        }

        self.command_cb = cb;
    }
}

#[derive(PartialEq)]
enum MouseCursor {
    None,
    Text,
    Default,
}

pub struct UiState {
    mouse_pressed: bool,
    scroll_delta: (f64, f64),

    mouse_cursor: MouseCursor,
}

impl UiState {
    pub fn new() -> UiState {
        UiState {
            mouse_pressed: false,
            scroll_delta: (0.0, 0.0),

            mouse_cursor: MouseCursor::None,
        }
    }

    fn apply_mouse_cursor(&mut self, cursor: MouseCursor, window: Option<gdk::Window>) {
        if self.mouse_cursor == cursor {
            return;
        }

        self.mouse_cursor = cursor;

        if let Some(window) = window {
            let cursor = match self.mouse_cursor {
                MouseCursor::Default => "default",
                MouseCursor::None => "none",
                MouseCursor::Text => "text",
            };

            window.set_cursor(&gdk::Cursor::new_from_name(&window.get_display(), cursor));
        }
    }
}

#[derive(Clone)]
pub struct ShellOptions {
    nvim_bin_path: Option<String>,
    open_paths: Vec<String>,
    timeout: Option<Duration>,
    args_for_neovim: Vec<String>,
    input_data: Option<String>,
    enable_swap: bool,
}

impl ShellOptions {
    pub fn new(
        nvim_bin_path: Option<String>,
        open_paths: Vec<String>,
        timeout: Option<Duration>,
        args_for_neovim: Vec<String>,
        input_data: Option<String>,
        enable_swap: bool,
    ) -> Self {
        ShellOptions {
            nvim_bin_path,
            open_paths,
            timeout,
            args_for_neovim,
            input_data,
            enable_swap,
        }
    }

    // remove input data from original
    // shell option, as it need to be used only once
    pub fn take(&mut self) -> Self {
        let input_data = self.input_data.take();
        let mut clone = self.clone();
        clone.input_data = input_data;

        clone
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
        shell.state.borrow_mut().cursor = Some(BlinkCursor::new(shell_ref));

        shell
    }

    pub fn is_nvim_initialized(&self) -> bool {
        let state = self.state.borrow();
        state.nvim.is_initialized()
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

        state.drawing_area.add_events(
            (gdk::EventMask::BUTTON_RELEASE_MASK
                | gdk::EventMask::BUTTON_PRESS_MASK
                | gdk::EventMask::BUTTON_MOTION_MASK
                | gdk::EventMask::SCROLL_MASK
                | gdk::EventMask::SMOOTH_SCROLL_MASK
                | gdk::EventMask::ENTER_NOTIFY_MASK
                | gdk::EventMask::LEAVE_NOTIFY_MASK
                | gdk::EventMask::POINTER_MOTION_MASK)
                .bits() as i32,
        );

        let menu = self.create_context_menu();
        let ref_state = self.state.clone();
        let ref_ui_state = self.ui_state.clone();
        state.drawing_area.connect_button_press_event(move |_, ev| {
            gtk_button_press(&mut *ref_state.borrow_mut(), &ref_ui_state, ev, &menu)
        });

        let ref_state = self.state.clone();
        let ref_ui_state = self.ui_state.clone();
        state
            .drawing_area
            .connect_button_release_event(move |_, ev| {
                gtk_button_release(
                    &mut *ref_state.borrow_mut(),
                    &mut *ref_ui_state.borrow_mut(),
                    ev,
                )
            });

        let ref_state = self.state.clone();
        let ref_ui_state = self.ui_state.clone();
        state
            .drawing_area
            .connect_motion_notify_event(move |_, ev| {
                gtk_motion_notify(
                    &mut *ref_state.borrow_mut(),
                    &mut *ref_ui_state.borrow_mut(),
                    ev,
                )
            });

        let ref_state = self.state.clone();
        state
            .drawing_area
            .connect_draw(move |_, ctx| gtk_draw(&ref_state, ctx));

        let ref_ui_state = self.ui_state.clone();
        let ref_state = self.state.clone();
        state.drawing_area.connect_key_press_event(move |_, ev| {
            ref_state
                .borrow_mut()
                .cursor
                .as_mut()
                .unwrap()
                .reset_state();

            if ref_state.borrow().im_context.filter_keypress(ev) {
                Inhibit(true)
            } else {
                let state = ref_state.borrow();
                let nvim = state.try_nvim();
                if let Some(mut nvim) = nvim {
                    input::gtk_key_press(&mut nvim, ev)
                } else {
                    Inhibit(false)
                }
            }
        });
        let ref_state = self.state.clone();
        state.drawing_area.connect_key_release_event(move |da, ev| {
            ref_state.borrow().im_context.filter_keypress(ev);
            ref_ui_state
                .borrow_mut()
                .apply_mouse_cursor(MouseCursor::None, da.get_window());
            Inhibit(false)
        });

        let ref_state = self.state.clone();
        let ref_ui_state = self.ui_state.clone();
        state.drawing_area.connect_scroll_event(move |_, ev| {
            gtk_scroll_event(
                &mut *ref_state.borrow_mut(),
                &mut *ref_ui_state.borrow_mut(),
                ev,
            )
        });

        let ref_state = self.state.clone();
        state
            .drawing_area
            .connect_focus_in_event(move |_, _| gtk_focus_in(&mut *ref_state.borrow_mut()));

        let ref_state = self.state.clone();
        state
            .drawing_area
            .connect_focus_out_event(move |_, _| gtk_focus_out(&mut *ref_state.borrow_mut()));

        let ref_state = self.state.clone();
        state.drawing_area.connect_realize(move |w| {
            // sometime set_client_window does not work without idle_add
            // and looks like not enabled im_context
            gtk::idle_add(clone!(ref_state, w => move || {
                ref_state.borrow().im_context.set_client_window(
                    w.get_window().as_ref(),
                );
                Continue(false)
            }));
        });

        let ref_state = self.state.clone();
        state
            .im_context
            .connect_commit(move |_, ch| ref_state.borrow().im_commit(ch));

        let ref_state = self.state.clone();
        state.drawing_area.connect_configure_event(move |_, ev| {
            debug!("configure_event {:?}", ev.get_size());

            let mut state = ref_state.borrow_mut();
            state.try_nvim_resize();

            false
        });

        let ref_state = self.state.clone();
        state.drawing_area.connect_size_allocate(move |_, _| {
            init_nvim(&ref_state);
        });

        let ref_state = self.state.clone();
        let targets = vec![gtk::TargetEntry::new(
            "text/uri-list",
            gtk::TargetFlags::OTHER_APP,
            0,
        )];
        state
            .drawing_area
            .drag_dest_set(gtk::DestDefaults::ALL, &targets, gdk::DragAction::COPY);
        state
            .drawing_area
            .connect_drag_data_received(move |_, _, _, _, s, _, _| {
                let uris = s.get_uris();
                let command = uris.iter().filter_map(|uri| decode_uri(uri)).fold(
                    ":ar".to_owned(),
                    |command, filename| {
                        let filename = escape_filename(&filename);
                        command + " " + &filename
                    },
                );
                let state = ref_state.borrow_mut();
                let mut nvim = state.nvim().unwrap();
                nvim.command_async(&command).cb(|r| r.report_err()).call()
            });

        let ui_state_ref = self.ui_state.clone();
        state.drawing_area.connect_enter_notify_event(move |_, ev| {
            ui_state_ref
                .borrow_mut()
                .apply_mouse_cursor(MouseCursor::Text, ev.get_window());
            gtk::Inhibit(false)
        });

        let ui_state_ref = self.ui_state.clone();
        state.drawing_area.connect_leave_notify_event(move |_, ev| {
            ui_state_ref
                .borrow_mut()
                .apply_mouse_cursor(MouseCursor::Default, ev.get_window());
            gtk::Inhibit(false)
        });
    }

    fn create_context_menu(&self) -> gtk::Menu {
        let menu = gtk::Menu::new();
        let copy = gtk::MenuItem::new_with_label("Copy");
        let ref_state = self.state.clone();
        copy.connect_activate(move |_| ref_state.borrow().edit_copy("+"));
        copy.show_all();

        let paste = gtk::MenuItem::new_with_label("Paste");
        let ref_state = self.state.clone();
        paste.connect_activate(move |_| ref_state.borrow().edit_paste("+"));
        paste.show_all();

        menu.append(&copy);
        menu.append(&paste);

        menu
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
        self.state.borrow().edit_paste("+");
    }

    pub fn edit_save_all(&self) {
        let state = self.state.borrow();

        let nvim = state.nvim();
        if let Some(mut nvim) = nvim {
            nvim.command_async(":wa").cb(|r| r.report_err()).call();
        }
    }

    pub fn new_tab(&self) {
        let state = self.state.borrow();

        let nvim = state.nvim();
        if let Some(mut nvim) = nvim {
            nvim.command_async(":tabe").cb(|r| r.report_err()).call();
        }
    }

    pub fn set_detach_cb<F>(&self, cb: Option<F>)
    where
        F: FnMut() + Send + 'static,
    {
        let mut state = self.state.borrow_mut();
        state.set_detach_cb(cb);
    }

    pub fn set_nvim_started_cb<F>(&self, cb: Option<F>)
    where
        F: FnMut() + Send + 'static,
    {
        let mut state = self.state.borrow_mut();
        state.set_nvim_started_cb(cb);
    }

    pub fn set_nvim_command_cb<F>(&self, cb: Option<F>)
    where
        F: FnMut(&mut State, nvim::NvimCommand) + Send + 'static,
    {
        let mut state = self.state.borrow_mut();
        state.set_nvim_command_cb(cb);
    }
}

impl Deref for Shell {
    type Target = gtk::Box;

    fn deref(&self) -> &gtk::Box {
        &self.widget
    }
}

fn gtk_focus_in(state: &mut State) -> Inhibit {
    if let Some(mut nvim) = state.try_nvim() {
        nvim.command_async("if exists('#FocusGained') | doautocmd FocusGained | endif")
            .cb(|r| r.report_err())
            .call();
    }

    state.im_context.focus_in();
    state.cursor.as_mut().unwrap().enter_focus();
    let point = state.model.cur_point();
    state.on_redraw(&RepaintMode::Area(point));
    Inhibit(false)
}

fn gtk_focus_out(state: &mut State) -> Inhibit {
    if let Some(mut nvim) = state.try_nvim() {
        nvim.command_async("if exists('#FocusLost') | doautocmd FocusLost | endif")
            .cb(|r| r.report_err())
            .call();
    }

    state.im_context.focus_out();
    state.cursor.as_mut().unwrap().leave_focus();
    let point = state.model.cur_point();
    state.on_redraw(&RepaintMode::Area(point));

    Inhibit(false)
}

fn gtk_scroll_event(state: &mut State, ui_state: &mut UiState, ev: &EventScroll) -> Inhibit {
    if !state.mouse_enabled && !state.nvim.is_initializing() {
        return Inhibit(false);
    }

    state.close_popup_menu();

    match ev.get_direction() {
        gdk::ScrollDirection::Right => {
            mouse_input(state, "ScrollWheelRight", ev.get_state(), ev.get_position())
        }
        gdk::ScrollDirection::Left => {
            mouse_input(state, "ScrollWheelLeft", ev.get_state(), ev.get_position())
        }
        gdk::ScrollDirection::Up => {
            mouse_input(state, "ScrollWheelUp", ev.get_state(), ev.get_position())
        }
        gdk::ScrollDirection::Down => {
            mouse_input(state, "ScrollWheelDown", ev.get_state(), ev.get_position())
        }
        gdk::ScrollDirection::Smooth => {
            // Remember and accumulate scroll deltas, so slow scrolling still
            // works.
            ui_state.scroll_delta.0 += ev.as_ref().delta_x;
            ui_state.scroll_delta.1 += ev.as_ref().delta_y;
            // Perform scroll action for deltas with abs(delta) >= 1.
            let x = ui_state.scroll_delta.0 as isize;
            let y = ui_state.scroll_delta.1 as isize;
            for _ in 0..x {
                mouse_input(state, "ScrollWheelRight", ev.get_state(), ev.get_position())
            }
            for _ in 0..-x {
                mouse_input(state, "ScrollWheelLeft", ev.get_state(), ev.get_position())
            }
            for _ in 0..y {
                mouse_input(state, "ScrollWheelDown", ev.get_state(), ev.get_position())
            }
            for _ in 0..-y {
                mouse_input(state, "ScrollWheelUp", ev.get_state(), ev.get_position())
            }
            // Subtract performed scroll deltas.
            ui_state.scroll_delta.0 -= x as f64;
            ui_state.scroll_delta.1 -= y as f64;
        }
        _ => (),
    }
    Inhibit(false)
}

fn gtk_button_press(
    shell: &mut State,
    ui_state: &Rc<RefCell<UiState>>,
    ev: &EventButton,
    menu: &gtk::Menu,
) -> Inhibit {
    if ev.get_event_type() != EventType::ButtonPress {
        return Inhibit(false);
    }

    if shell.mouse_enabled {
        ui_state.borrow_mut().mouse_pressed = true;

        match ev.get_button() {
            1 => mouse_input(shell, "LeftMouse", ev.get_state(), ev.get_position()),
            2 => mouse_input(shell, "MiddleMouse", ev.get_state(), ev.get_position()),
            3 => menu.popup_at_pointer(None),

            _ => (),
        }
    }
    Inhibit(false)
}

fn mouse_input(shell: &mut State, input: &str, state: ModifierType, position: (f64, f64)) {
    if let Some(mut nvim) = shell.try_nvim() {
        let &CellMetrics {
            line_height,
            char_width,
            ..
        } = shell.render_state.borrow().font_ctx.cell_metrics();
        let (x, y) = position;
        let col = (x / char_width).trunc() as u64;
        let row = (y / line_height).trunc() as u64;
        let input_str = format!("{}<{},{}>", keyval_to_input_string(input, state), col, row);

        nvim.input(&input_str)
            .expect("Can't send mouse input event");
    }
}

fn gtk_button_release(shell: &mut State, ui_state: &mut UiState, ev: &EventButton) -> Inhibit {
    ui_state.mouse_pressed = false;

    if shell.mouse_enabled && !shell.nvim.is_initializing() {
        match ev.get_button() {
            1 => mouse_input(shell, "LeftRelease", ev.get_state(), ev.get_position()),
            2 => mouse_input(shell, "MiddleRelease", ev.get_state(), ev.get_position()),
            3 => mouse_input(shell, "RightRelease", ev.get_state(), ev.get_position()),
            _ => (),
        }
    }

    Inhibit(false)
}

fn gtk_motion_notify(shell: &mut State, ui_state: &mut UiState, ev: &EventMotion) -> Inhibit {
    if shell.mouse_enabled && ui_state.mouse_pressed {
        mouse_input(shell, "LeftDrag", ev.get_state(), ev.get_position());
    }

    ui_state.apply_mouse_cursor(MouseCursor::Text, shell.drawing_area.get_window());
    Inhibit(false)
}

fn draw_content(state: &State, ctx: &cairo::Context) {
    ctx.push_group();

    let render_state = state.render_state.borrow();
    render::render(
        ctx,
        state.cursor.as_ref().unwrap(),
        &render_state.font_ctx,
        &state.model,
        &render_state.color_model,
        state.transparency_settings.filled_alpha(),
    );
    render::fill_background(
        ctx,
        &render_state.color_model,
        state.transparency_settings.background_alpha(),
    );

    ctx.pop_group_to_source();
    ctx.paint();
}

fn gtk_draw(state_arc: &Arc<UiMutex<State>>, ctx: &cairo::Context) -> Inhibit {
    let state = state_arc.borrow();
    if state.nvim.is_initialized() {
        draw_content(&*state, ctx);
    } else if state.nvim.is_initializing() {
        draw_initializing(&*state, ctx);
    }

    Inhibit(false)
}

fn show_nvim_start_error(err: &nvim::NvimInitError, state_arc: Arc<UiMutex<State>>) {
    let source = err.source();
    let cmd = err.cmd().unwrap().to_owned();

    glib::idle_add(move || {
        let state = state_arc.borrow();
        state.nvim.set_error();
        state.error_area.show_nvim_start_error(&source, &cmd);
        state.show_error_area();

        Continue(false)
    });
}

fn show_nvim_init_error(err: &nvim::NvimInitError, state_arc: Arc<UiMutex<State>>) {
    let source = err.source();

    glib::idle_add(move || {
        let state = state_arc.borrow();
        state.nvim.set_error();
        state.error_area.show_nvim_init_error(&source);
        state.show_error_area();

        Continue(false)
    });
}

fn init_nvim_async(
    state_arc: Arc<UiMutex<State>>,
    nvim_handler: NvimHandler,
    options: ShellOptions,
    cols: usize,
    rows: usize,
) {
    // execute nvim
    let nvim = match nvim::start(
        nvim_handler,
        options.nvim_bin_path.as_ref(),
        options.timeout,
        options.args_for_neovim,
        options.enable_swap,
    ) {
        Ok(nvim) => nvim,
        Err(err) => {
            show_nvim_start_error(&err, state_arc);
            return;
        }
    };

    let nvim = set_nvim_to_state(state_arc.clone(), nvim);

    // add callback on session end
    let guard = nvim.borrow().unwrap().session.take_dispatch_guard();
    let state_ref = state_arc.clone();
    thread::spawn(move || {
        guard.join().expect("Can't join dispatch thread");

        glib::idle_add(move || {
            state_ref.borrow().nvim.clear();
            if let Some(ref cb) = state_ref.borrow().detach_cb {
                (&mut *cb.borrow_mut())();
            }

            glib::Continue(false)
        });
    });

    // attach ui
    if let Err(err) = nvim::post_start_init(
        nvim,
        options.open_paths,
        cols as u64,
        rows as u64,
        options.input_data,
    ) {
        show_nvim_init_error(&err, state_arc.clone());
    } else {
        set_nvim_initialized(state_arc);
    }
}

fn set_nvim_to_state(state_arc: Arc<UiMutex<State>>, nvim: Neovim) -> NeovimClientAsync {
    let pair = Arc::new((Mutex::new(None), Condvar::new()));
    let pair2 = pair.clone();
    let mut nvim = Some(nvim);

    glib::idle_add(move || {
        let nvim_aync = state_arc.borrow().nvim.set_nvim_async(nvim.take().unwrap());

        let &(ref lock, ref cvar) = &*pair2;
        let mut started = lock.lock().unwrap();
        *started = Some(nvim_aync);
        cvar.notify_one();

        Continue(false)
    });

    // Wait idle set nvim properly
    let &(ref lock, ref cvar) = &*pair;
    let mut started = lock.lock().unwrap();
    while started.is_none() {
        started = cvar.wait(started).unwrap();
    }

    started.take().unwrap()
}

fn set_nvim_initialized(state_arc: Arc<UiMutex<State>>) {
    glib::idle_add(clone!(state_arc => move || {
        let mut state = state_arc.borrow_mut();
        state.nvim.async_to_sync();
        state.nvim.set_initialized();
        // in some case resize can happens while initilization in progress
        // so force resize here
        state.try_nvim_resize();
        state.cursor.as_mut().unwrap().start();

        Continue(false)
    }));

    idle_cb_call!(state_arc.nvim_started_cb());
}

fn draw_initializing(state: &State, ctx: &cairo::Context) {
    let render_state = state.render_state.borrow();
    let color_model = &render_state.color_model;
    let layout = pangocairo::functions::create_layout(ctx).unwrap();
    let alloc = state.drawing_area.get_allocation();

    ctx.set_source_rgb(
        color_model.bg_color.0,
        color_model.bg_color.1,
        color_model.bg_color.2,
    );
    ctx.paint();

    layout.set_text("Loading->");
    let (width, height) = layout.get_pixel_size();

    let x = alloc.width as f64 / 2.0 - width as f64 / 2.0;
    let y = alloc.height as f64 / 2.0 - height as f64 / 2.0;

    ctx.move_to(x, y);
    ctx.set_source_rgb(
        color_model.fg_color.0,
        color_model.fg_color.1,
        color_model.fg_color.2,
    );
    pangocairo::functions::update_layout(ctx, &layout);
    pangocairo::functions::show_layout(ctx, &layout);

    ctx.move_to(x + width as f64, y);
    state
        .cursor
        .as_ref()
        .unwrap()
        .draw(ctx, &render_state.font_ctx, y, false, &color_model);
}

fn init_nvim(state_ref: &Arc<UiMutex<State>>) {
    let mut state = state_ref.borrow_mut();
    if state.start_nvim_initialization() {
        let (cols, rows) = state.calc_nvim_size();

        debug!("Init nvim {}/{}", cols, rows);

        state.model = UiModel::new(rows as u64, cols as u64);

        let state_arc = state_ref.clone();
        let nvim_handler = NvimHandler::new(state_ref.clone());
        let options = state.options.take();
        thread::spawn(move || init_nvim_async(state_arc, nvim_handler, options, cols, rows));
    }
}

// Neovim redraw events
impl State {
    pub fn redraw_handler_finish(&mut self) {
        if self.update_im_location {
            self.set_im_location();
            self.update_im_location = false;
        }
    }

    pub fn on_cursor_goto(&mut self, row: u64, col: u64) -> RepaintMode {
        let repaint_area = self.model.set_cursor(row as usize, col as usize);
        self.update_im_location = true;
        RepaintMode::AreaList(repaint_area)
    }

    pub fn on_put(&mut self, text: String) -> RepaintMode {
        let double_width = text.is_empty();
        RepaintMode::Area(self.model.put(&text, double_width, self.cur_attrs.as_ref()))
    }

    pub fn on_clear(&mut self) -> RepaintMode {
        debug!("clear model");

        self.model.clear();
        RepaintMode::All
    }

    pub fn on_eol_clear(&mut self) -> RepaintMode {
        RepaintMode::Area(self.model.eol_clear())
    }

    pub fn on_resize(&mut self, columns: u64, rows: u64) -> RepaintMode {
        debug!("on_resize {}/{}", columns, rows);

        if self.model.columns != columns as usize || self.model.rows != rows as usize {
            self.model = UiModel::new(rows, columns);
        }

        if let Some(mut nvim) = self.nvim.nvim() {
            let mut render_state = self.render_state.borrow_mut();
            render_state.color_model.theme.queue_update(&mut *nvim);
        }
        RepaintMode::Nothing
    }

    pub fn on_redraw(&mut self, mode: &RepaintMode) {
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

    pub fn on_set_scroll_region(
        &mut self,
        top: u64,
        bot: u64,
        left: u64,
        right: u64,
    ) -> RepaintMode {
        self.model.set_scroll_region(top, bot, left, right);
        RepaintMode::Nothing
    }

    pub fn on_scroll(&mut self, count: i64) -> RepaintMode {
        RepaintMode::Area(self.model.scroll(count))
    }

    pub fn on_highlight_set(&mut self, attrs: HashMap<String, Value>) -> RepaintMode {
        let model_attrs = Attrs::from_value_map(&attrs);

        self.cur_attrs = Some(model_attrs);
        RepaintMode::Nothing
    }

    pub fn on_update_bg(&mut self, bg: i64) -> RepaintMode {
        let mut render_state = self.render_state.borrow_mut();
        if bg >= 0 {
            render_state.color_model.bg_color = Color::from_indexed_color(bg as u64);
        } else {
            render_state.color_model.bg_color = COLOR_BLACK;
        }
        RepaintMode::Nothing
    }

    pub fn on_update_fg(&mut self, fg: i64) -> RepaintMode {
        let mut render_state = self.render_state.borrow_mut();
        if fg >= 0 {
            render_state.color_model.fg_color = Color::from_indexed_color(fg as u64);
        } else {
            render_state.color_model.fg_color = COLOR_WHITE;
        }
        RepaintMode::Nothing
    }

    pub fn on_update_sp(&mut self, sp: i64) -> RepaintMode {
        let mut render_state = self.render_state.borrow_mut();
        if sp >= 0 {
            render_state.color_model.sp_color = Color::from_indexed_color(sp as u64);
        } else {
            render_state.color_model.sp_color = COLOR_RED;
        }
        RepaintMode::Nothing
    }

    pub fn on_mode_change(&mut self, mode: String, idx: u64) -> RepaintMode {
        let mut render_state = self.render_state.borrow_mut();
        render_state.mode.update(&mode, idx as usize);
        self.cursor
            .as_mut()
            .unwrap()
            .set_mode_info(render_state.mode.mode_info().cloned());
        self.cmd_line
            .set_mode_info(render_state.mode.mode_info().cloned());
        RepaintMode::Area(self.model.cur_point())
    }

    pub fn on_mouse(&mut self, on: bool) -> RepaintMode {
        self.mouse_enabled = on;
        RepaintMode::Nothing
    }

    pub fn on_busy(&mut self, busy: bool) -> RepaintMode {
        if busy {
            self.cursor.as_mut().unwrap().busy_on();
        } else {
            self.cursor.as_mut().unwrap().busy_off();
        }
        RepaintMode::Area(self.model.cur_point())
    }

    pub fn popupmenu_show(
        &mut self,
        menu: &[CompleteItem],
        selected: i64,
        row: u64,
        col: u64,
    ) -> RepaintMode {
        let point = ModelRect::point(col as usize, row as usize);
        let render_state = self.render_state.borrow();
        let (x, y, width, height) = point.to_area(render_state.font_ctx.cell_metrics());

        let context = popup_menu::PopupMenuContext {
            nvim: &self.nvim,
            color_model: &render_state.color_model,
            font_ctx: &render_state.font_ctx,
            menu_items: &menu,
            selected,
            x,
            y,
            width,
            height,
            max_width: self.max_popup_width(),
        };

        self.popup_menu.show(context);

        RepaintMode::Nothing
    }

    pub fn popupmenu_hide(&mut self) -> RepaintMode {
        self.popup_menu.hide();
        RepaintMode::Nothing
    }

    pub fn popupmenu_select(&mut self, selected: i64) -> RepaintMode {
        self.popup_menu.select(selected);
        RepaintMode::Nothing
    }

    pub fn tabline_update(
        &mut self,
        selected: Tabpage,
        tabs: Vec<(Tabpage, Option<String>)>,
    ) -> RepaintMode {
        self.tabs.update_tabs(&self.nvim, &selected, &tabs);

        RepaintMode::Nothing
    }

    pub fn option_set(&mut self, name: String, val: Value) -> RepaintMode {
        match name.as_str() {
            "guifont" => self.set_font_from_value(val),
            _ => (),
        };
        RepaintMode::Nothing
    }

    fn set_font_from_value(&mut self, val: Value) {
        if let Value::String(val) = val {
            if let Some(val) = val.into_str() {
                if !val.is_empty() {
                    let exists_fonts = self.render_state.borrow().font_ctx.font_families();
                    let fonts = split_at_comma(&val);
                    for font in &fonts {
                        let desc = FontDescription::from_string(&font);
                        if desc.get_size() > 0
                            && exists_fonts.contains(&desc.get_family().unwrap_or("".to_owned()))
                        {
                            self.set_font_rpc(font);
                            return;
                        }
                    }

                    // font does not exists? set first one
                    if !fonts.is_empty() {
                        self.set_font_rpc(&fonts[0]);
                    }
                }
            }
        }
    }

    pub fn mode_info_set(
        &mut self,
        cursor_style_enabled: bool,
        mode_infos: Vec<HashMap<String, Value>>,
    ) -> RepaintMode {
        let mode_info_arr = mode_infos
            .iter()
            .map(|mode_info_map| mode::ModeInfo::new(mode_info_map))
            .collect();

        match mode_info_arr {
            Ok(mode_info_arr) => {
                let mut render_state = self.render_state.borrow_mut();
                render_state
                    .mode
                    .set_info(cursor_style_enabled, mode_info_arr);
            }
            Err(err) => {
                error!("Error load mode info: {}", err);
            }
        }

        RepaintMode::Nothing
    }

    pub fn cmdline_show(
        &mut self,
        content: Vec<(HashMap<String, Value>, String)>,
        pos: u64,
        firstc: String,
        prompt: String,
        indent: u64,
        level: u64,
    ) -> RepaintMode {
        {
            let cursor = self.model.cur_point();
            let render_state = self.render_state.borrow();
            let (x, y, width, height) = cursor.to_area(render_state.font_ctx.cell_metrics());
            let ctx = CmdLineContext {
                nvim: &self.nvim,
                content,
                pos,
                firstc,
                prompt,
                indent,
                level_idx: level,
                x,
                y,
                width,
                height,
                max_width: self.max_popup_width(),
            };

            self.cmd_line.show_level(&ctx);
        }

        self.on_busy(true)
    }

    pub fn cmdline_hide(&mut self, level: u64) -> RepaintMode {
        self.cmd_line.hide_level(level);
        self.on_busy(false)
    }

    pub fn cmdline_block_show(
        &mut self,
        content: Vec<Vec<(HashMap<String, Value>, String)>>,
    ) -> RepaintMode {
        let max_width = self.max_popup_width();
        self.cmd_line.show_block(&content, max_width);
        self.on_busy(true)
    }

    pub fn cmdline_block_append(
        &mut self,
        content: Vec<(HashMap<String, Value>, String)>,
    ) -> RepaintMode {
        self.cmd_line.block_append(&content);
        RepaintMode::Nothing
    }

    pub fn cmdline_block_hide(&mut self) -> RepaintMode {
        self.cmd_line.block_hide();
        self.on_busy(false)
    }

    pub fn cmdline_pos(&mut self, pos: u64, level: u64) -> RepaintMode {
        let render_state = self.render_state.borrow();
        self.cmd_line.pos(&*render_state, pos, level);
        RepaintMode::Nothing
    }

    pub fn cmdline_special_char(&mut self, c: String, shift: bool, level: u64) -> RepaintMode {
        let render_state = self.render_state.borrow();
        self.cmd_line.special_char(&*render_state, c, shift, level);
        RepaintMode::Nothing
    }

    pub fn wildmenu_show(&self, items: Vec<String>) -> RepaintMode {
        self.cmd_line
            .show_wildmenu(items, &*self.render_state.borrow(), self.max_popup_width());
        RepaintMode::Nothing
    }

    pub fn wildmenu_hide(&self) -> RepaintMode {
        self.cmd_line.hide_wildmenu();
        RepaintMode::Nothing
    }

    pub fn wildmenu_select(&self, selected: i64) -> RepaintMode {
        self.cmd_line.wildmenu_select(selected);
        RepaintMode::Nothing
    }
}

impl CursorRedrawCb for State {
    fn queue_redraw_cursor(&mut self) {
        let cur_point = self.model.cur_point();
        self.on_redraw(&RepaintMode::Area(cur_point));
    }
}
