extern crate gtk;
extern crate cairo;

mod ui;

use ui::Ui;

fn main() {
    Ui::new().start();
}

