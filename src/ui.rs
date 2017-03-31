use std::cell::{RefCell, Ref, RefMut};
use std::thread;

use gtk;
use gtk_sys;
use gtk::prelude::*;
use gtk::{ApplicationWindow, HeaderBar, ToolButton, Image, AboutDialog};
use gdk::Event;
use gio::{Menu, MenuItem, SimpleAction};
use glib::variant::Variant;

use neovim_lib::NeovimApi;

use settings;
use shell::{Shell, NvimMode};
use nvim::ErrorReport;

macro_rules! ui_thread_var {
    ($id:ident, $ty:ty, $expr:expr) => (thread_local!(pub static $id: RefCell<$ty> = {
        assert_ui_thread();
        RefCell::new($expr)
    });)
}

ui_thread_var!(UI, Ui, Ui::new());
ui_thread_var!(SH, Shell, Shell::new());
ui_thread_var!(SET, settings::Settings, settings::Settings::new());


#[macro_export]
macro_rules! SHELL {
    (&$id:ident = $expr:expr) => (
        SH.with(|shell_cell| {
        let $id = &shell_cell.borrow();
        $expr
    });
    );
    ($id:ident = $expr:expr) => (
        SH.with(|shell_cell| {
        let mut $id = &mut shell_cell.borrow_mut();
        $expr
    });
    );
}

pub struct Ui {
    pub initialized: bool,
    pub window: Option<ApplicationWindow>,
    header_bar: HeaderBar,
}

impl Ui {
    pub fn new() -> Ui {
        Ui {
            window: None,
            header_bar: HeaderBar::new(),
            initialized: false,
        }
    }

    pub fn close_window(&self) {
        self.window.as_ref().unwrap().destroy();
    }

    pub fn destroy(&mut self) {
        self.close_window();
        SHELL!(shell = {
            shell.nvim().ui_detach().expect("Error in ui_detach");
        });
    }

    pub fn init(&mut self, app: &gtk::Application) {
        if self.initialized {
            return;
        }
        self.initialized = true;

        self.create_main_menu(app);

        SHELL!(shell = {
            SET.with(|settings| {
                let mut settings = settings.borrow_mut();
                settings.init(&mut shell);
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

            shell.init();

            self.window = Some(ApplicationWindow::new(app));
            let window = self.window.as_ref().unwrap();

            window.set_titlebar(Some(&self.header_bar));
            window.add(&shell.drawing_area);
            window.show_all();
            window.connect_delete_event(gtk_delete);
            window.set_title("Neovim-gtk");

            shell.add_configure_event();
        });
    }

    fn create_main_menu(&self, app: &gtk::Application) {
        let menu = Menu::new();
        let help = Menu::new();

        let about = MenuItem::new("About", None);
        about.set_detailed_action("app.HelpAbout");
        help.append_item(&about);


        menu.append_item(&MenuItem::new_submenu("Help", &help));

        app.set_menubar(Some(&menu));


        let about_action = SimpleAction::new("HelpAbout", None);
        about_action.connect_activate(on_help_about);
        about_action.set_enabled(true);
        app.add_action(&about_action);
    }
}

fn on_help_about(_: &SimpleAction, _: &Option<Variant>) {
    UI.with(|ui_cell| {
        let ui = ui_cell.borrow();

        let about = AboutDialog::new();
        about.set_transient_for(ui.window.as_ref());
        about.set_program_name("NeovimGtk");
        about.set_version(env!("CARGO_PKG_VERSION"));
        about.set_logo(None);
        about.set_authors(&[env!("CARGO_PKG_AUTHORS")]);

        about.connect_response(|about, _| about.destroy());
        about.show();
    });
}

fn edit_paste() {
    SHELL!(shell = {
        let paste_command = if shell.mode == NvimMode::Normal {
            "\"*p"
        } else {
            "<Esc>\"*pa"
        };

        let mut nvim = shell.nvim();
        nvim.input(paste_command).report_err(nvim);
    });
}

fn edit_save_all() {
    SHELL!(shell = {
        let mut nvim = shell.nvim();
        nvim.command(":wa").report_err(nvim);
    });
}

fn quit() {
    UI.with(|ui_cell| {
        let mut ui = ui_cell.borrow_mut();
        ui.destroy();
    });
}

fn gtk_delete(_: &ApplicationWindow, _: &Event) -> Inhibit {
    quit();
    Inhibit(false)
}


pub struct UiMutex<T: ?Sized> {
    data: RefCell<T>,
}

unsafe impl<T: ?Sized + Send> Send for UiMutex<T> {}
unsafe impl<T: ?Sized + Send> Sync for UiMutex<T> {}

impl<T> UiMutex<T> {
    pub fn new(t: T) -> UiMutex<T> {
        UiMutex { data: RefCell::new(t) }
    }
}

impl<T: ?Sized> UiMutex<T> {
    pub fn borrow(&self) -> Ref<T> {
        assert_ui_thread();
        self.data.borrow()
    }

    pub fn borrow_mut(&self) -> RefMut<T> {
        assert_ui_thread();
        self.data.borrow_mut()
    }
}


#[inline]
fn assert_ui_thread() {
    match thread::current().name() {
        Some("main") => (),
        Some(ref name) => {
            panic!("Can create UI  only from main thread, {}", name);
        }
        None => panic!("Can create UI  only from main thread, current thiread has no name"),
    }
}

