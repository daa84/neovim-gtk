use cairo;
use gtk;
use gtk::prelude::*;
use gtk::{Window, WindowType, DrawingArea, Grid, Button, ButtonBox, Orientation};

use ui_model::UiModel;

pub struct Ui;

impl Ui {
    pub fn new() -> Ui {
        Ui
    }

    pub fn start(&self) {
        gtk::init().expect("Failed to initialize GTK");
        let window = Window::new(WindowType::Toplevel);

        let grid = Grid::new();

        let button_bar = ButtonBox::new(Orientation::Horizontal);
        let save = Button::new_with_label("Save");
        button_bar.add(&save);
        grid.attach(&button_bar, 0, 0, 1, 1);

        let drawing_area = DrawingArea::new();
        drawing_area.set_size_request(500, 500);
        drawing_area.connect_draw(Self::gtk_draw);
        grid.attach(&drawing_area, 0, 1, 1, 1);

        window.add(&grid);
        window.show_all();
        window.connect_delete_event(|_,_| {
            gtk::main_quit();
            Inhibit(false)
        });

        gtk::main();       
    }

    fn gtk_draw(drawing_area: &DrawingArea, ctx: &cairo::Context) -> Inhibit {
        let width = drawing_area.get_allocated_width() as f64;
        let height = drawing_area.get_allocated_height() as f64;
        ctx.set_source_rgb(1.0, 0.0, 0.0);
        ctx.arc(width / 2.0, height / 2.0,
                width / 2.0,
                0.0, 2.0 * 3.14);
        ctx.fill();
        Inhibit(true)
    }
}
