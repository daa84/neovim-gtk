extern crate gtk;
extern crate gdk;
extern crate gdk_sys;
extern crate glib;
//extern crate glib_sys;
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

fn main() {
    gtk::init().expect("Failed to initialize GTK");
    ui::UI.with(|ui_cell| {
        let mut ui = ui_cell.borrow_mut();
        ui.init();

        nvim::initialize(&mut *ui).expect("Can't start nvim instance");

        guard_dispatch_thread(&mut *ui);
    });

    gtk::main();
}

fn guard_dispatch_thread(ui: &mut ui::Ui) {
    let guard = ui.nvim().session.take_dispatch_guard();
    thread::spawn(move || {
        guard.join().expect("Can't join dispatch thread");
        glib::idle_add(move || {
            gtk::main_quit();
            glib::Continue(false)
        });
    });
}
