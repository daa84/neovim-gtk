use std::cell::RefCell;
use std::thread;

use gtk;
use gtk_sys;
use gtk::prelude::*;
use gtk::{ApplicationWindow, HeaderBar, ToolButton, Image};
use gdk::Event;

use neovim_lib::NeovimApi;

use settings;
use shell::{Shell, NvimMode};
use nvim::ErrorReport;


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

ui_thread_var!(UI, Ui, Ui::new());
ui_thread_var!(SET, settings::Settings, settings::Settings::new());


pub struct Ui {
    pub initialized: bool,
    pub window: Option<ApplicationWindow>,
    header_bar: HeaderBar,
    pub shell: Shell,
}

impl Ui {
    pub fn new() -> Ui {
        Ui {
            window: None,
            header_bar: HeaderBar::new(),
            initialized: false,
            shell: Shell::new(),
        }
    }

    pub fn close_window(&self) {
        self.window.as_ref().unwrap().destroy();
    }

    pub fn destroy(&mut self) {
        self.close_window();
        self.shell.nvim().ui_detach().expect("Error in ui_detach");
    }

    pub fn init(&mut self, app: &gtk::Application) {
        if self.initialized {
            return;
        }
        self.initialized = true;

        SET.with(|settings| {
            let mut settings = settings.borrow_mut();
            settings.init(&mut self.shell);
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

        self.shell.init();

        self.window = Some(ApplicationWindow::new(app));
        let window = self.window.as_ref().unwrap();

        window.set_titlebar(Some(&self.header_bar));
        window.add(&self.shell.drawing_area);
        window.show_all();
        window.connect_delete_event(gtk_delete);
        window.set_title("Neovim-gtk");

        self.shell.add_configure_event();
    }

}

fn edit_paste() {
    UI.with(|ui_cell| {
        let mut ui = ui_cell.borrow_mut();

        let paste_command = if ui.shell.mode == NvimMode::Normal {
            "\"*p"
        } else {
            "<Esc>\"*pa"
        };

        let mut nvim = ui.shell.nvim();
        nvim.input(paste_command).report_err(nvim);
    });
}

fn edit_save_all() {
    UI.with(|ui_cell| {
        let mut ui = ui_cell.borrow_mut();

        let mut nvim = ui.shell.nvim();
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

