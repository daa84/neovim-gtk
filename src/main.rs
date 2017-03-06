extern crate gtk;
extern crate gio;
extern crate gdk;
extern crate gdk_sys;
extern crate glib;
extern crate cairo;
extern crate pango;
extern crate pangocairo;
extern crate neovim_lib;
extern crate phf;

mod nvim;
mod ui_model;
mod ui;
mod input;

use std::thread;
use std::env;
use gio::ApplicationExt;

fn main() {
    let app = gtk::Application::new(Some("org.gtk.neovim-gtk"), gio::ApplicationFlags::empty()).expect("Failed to initialize GTK application");

    app.connect_activate(activate);

    let args: Vec<String> = env::args().collect();
    let argv: Vec<&str> = args.iter().map(String::as_str).collect();
    app.run(argv.len() as i32, &argv);
}

fn activate(app: &gtk::Application) {
    ui::UI.with(|ui_cell| {
        let mut ui = ui_cell.borrow_mut();
        ui.init(app);

        nvim::initialize(&mut *ui).expect("Can't start nvim instance");

        guard_dispatch_thread(&mut *ui);
    });
}

fn guard_dispatch_thread(ui: &mut ui::Ui) {
    let guard = ui.nvim().session.take_dispatch_guard();
    thread::spawn(move || {
        guard.join().expect("Can't join dispatch thread");
        glib::idle_add(move || {
            ui::UI.with(|ui_cell| {
                ui_cell.borrow().destroy();
            });
            glib::Continue(false)
        });
    });
}
