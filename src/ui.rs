use cairo;
use gtk;
use gtk::prelude::*;
use gtk::{Window, WindowType, DrawingArea, Grid, ToolButton, ButtonBox, Orientation, Image};
use neovim_lib::Neovim;

use ui_model::UiModel;
use nvim::RedrawEvents;

pub struct Ui {
    model: UiModel,
    nvim: Option<Neovim>,
}

impl Ui {
    pub fn new() -> Ui {
        Ui {
            model: UiModel::empty(),
            nvim: None,
        }
    }

    pub fn set_nvim(&mut self, nvim: Neovim) {
        self.nvim = Some(nvim);
    }

    pub fn nvim(&mut self) -> &mut Neovim {
        self.nvim.as_mut().unwrap()
    }

    pub fn show(&self) {
        gtk::init().expect("Failed to initialize GTK");
        let window = Window::new(WindowType::Toplevel);

        let grid = Grid::new();

        let button_bar = ButtonBox::new(Orientation::Horizontal);
        button_bar.set_hexpand(true);
        button_bar.set_layout(gtk::ButtonBoxStyle::Start);

        let open_image = Image::new_from_icon_name("document-open", 50);
        let open_btn = ToolButton::new(Some(&open_image), None);
        button_bar.add(&open_btn);

        let save_image = Image::new_from_icon_name("document-save", 50);
        let save_btn = ToolButton::new(Some(&save_image), None);
        button_bar.add(&save_btn);

        let exit_image = Image::new_from_icon_name("application-exit", 50);
        let exit_btn = ToolButton::new(Some(&exit_image), None);
        button_bar.add(&exit_btn);

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

impl RedrawEvents for Ui {
}

