extern crate gtk;
extern crate cairo;

use gtk::prelude::*;
use gtk::{Window, WindowType, DrawingArea};

fn main() {
    gtk::init().expect("Failed to initialize GTK");
    let window = Window::new(WindowType::Toplevel);
    let drawing_area = DrawingArea::new();
    drawing_area.set_size_request(500, 500);
    drawing_area.connect_draw(_gtk_draw);
    window.add(&drawing_area);
    window.show_all();
    window.connect_delete_event(|_,_| {
        gtk::main_quit();
        Inhibit(false)
    });

    gtk::main();
}

fn _gtk_draw(drawing_area: &DrawingArea, ctx: &cairo::Context) -> Inhibit {
    let width = drawing_area.get_allocated_width() as f64;
    let height = drawing_area.get_allocated_height() as f64;
    ctx.set_source_rgb(1.0, 0.0, 0.0);
    ctx.arc(width / 2.0, height / 2.0,
             width / 2.0,
             0.0, 2.0 * 3.14);
    ctx.fill();
    Inhibit(true)
}
