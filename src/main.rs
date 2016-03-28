extern crate gtk;
extern crate cairo;
extern crate neovim_lib;
extern crate rmp;

mod ui_mutex;
mod nvim;
mod ui_model;
mod ui;

use ui::Ui;

fn main() {
    let ui = Ui::new();
    ui.show();

    let nvim = nvim::initialize(ui).expect("Can't start nvim instance");

    gtk::main();       
}

