extern crate gtk;
extern crate gdk;
extern crate glib;
extern crate glib_sys;
extern crate cairo;
extern crate neovim_lib;
extern crate rmp;
extern crate phf;

mod nvim;
mod ui_model;
mod ui;
mod input;

fn main() {
    gtk::init().expect("Failed to initialize GTK");
    ui::UI.with(|ui_cell| {
        let mut ui = ui_cell.borrow_mut();
        ui.init();

        nvim::initialize(&mut *ui).expect("Can't start nvim instance");
    });

    gtk::main();       
}

