use gtk;
use gtk::prelude::*;

use super::store;

pub struct Builder<'a> {
    title: &'a str
}

impl <'a> Builder <'a> {
    pub fn new(title: &'a str) -> Self {
        Builder { title }
    }

    pub fn show<F: IsA<gtk::Window>>(&self, parent: &F) -> Option<store::PlugInfo> {
        let dlg = gtk::Dialog::new_with_buttons(
            Some(self.title),
            Some(parent),
            gtk::DIALOG_USE_HEADER_BAR | gtk::DIALOG_DESTROY_WITH_PARENT,
            &[("Cancel", gtk::ResponseType::Cancel.into()),
            ("Ok", gtk::ResponseType::Accept.into())],
        );

        let content = dlg.get_content_area();
        let grid = gtk::Grid::new();

        let label = gtk::Label::new("Path:");
        let entry = gtk::Entry::new();
        
        grid.attach(&label, 0, 0, 1, 1);
        grid.attach(&entry, 1, 0, 1, 1);

        content.add(&grid);
        content.show_all();

        if dlg.run() == gtk::ResponseType::Ok.into() {
        }

        dlg.destroy();

        None
    }
}

