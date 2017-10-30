use gtk;
use gtk::prelude::*;

use super::store;

pub struct Builder<'a> {
    title: &'a str,
}

impl<'a> Builder<'a> {
    pub fn new(title: &'a str) -> Self {
        Builder { title }
    }

    pub fn show<F: IsA<gtk::Window>>(&self, parent: &F) -> Option<store::PlugInfo> {
        let dlg = gtk::Dialog::new_with_buttons(
            Some(self.title),
            Some(parent),
            gtk::DIALOG_USE_HEADER_BAR | gtk::DIALOG_DESTROY_WITH_PARENT,
            &[
                ("Cancel", gtk::ResponseType::Cancel.into()),
                ("Ok", gtk::ResponseType::Ok.into()),
            ],
        );

        let content = dlg.get_content_area();
        let list = gtk::ListBox::new();
        list.set_selection_mode(gtk::SelectionMode::None);

        let path = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        let label = gtk::Label::new("Repo");
        let entry = gtk::Entry::new();

        path.pack_start(&label, true, true, 0);
        path.pack_end(&entry, false, true, 0);

        list.add(&path);


        content.add(&list);
        content.show_all();

        let ok: i32 = gtk::ResponseType::Ok.into();
        let res = if dlg.run() == ok {
            entry.get_text().map(|name| {
                store::PlugInfo::new(name.to_owned(), name.to_owned())
            })
        } else {
            None
        };

        dlg.destroy();

        res
    }
}
