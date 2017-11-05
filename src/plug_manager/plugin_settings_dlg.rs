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
        let border = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        border.set_border_width(12);

        let list = gtk::ListBox::new();
        list.set_selection_mode(gtk::SelectionMode::None);

        let path = gtk::Box::new(gtk::Orientation::Horizontal, 5);
        path.set_border_width(5);
        let path_lbl = gtk::Label::new("Repo");
        let path_e = gtk::Entry::new();
        path_e.set_placeholder_text("user_name/repo_name");

        path.pack_start(&path_lbl, true, true, 0);
        path.pack_end(&path_e, false, true, 0);

        list.add(&path);


        let name = gtk::Box::new(gtk::Orientation::Horizontal, 5);
        name.set_border_width(5);
        let name_lbl = gtk::Label::new("Name");
        let name_e = gtk::Entry::new();

        name.pack_start(&name_lbl, true, true, 0);
        name.pack_end(&name_e, false, true, 0);

        list.add(&name);

        border.pack_start(&list, true, true, 0);
        content.add(&border);
        content.show_all();

        path_e.connect_changed(clone!(name_e => move |p| {
            if let Some(name) = p.get_text().and_then(|t| extract_name(&t)) {
                name_e.set_text(&name);
            }
        }));

        let ok: i32 = gtk::ResponseType::Ok.into();
        let res = if dlg.run() == ok {
            path_e.get_text().map(|path| {
                let name = name_e
                    .get_text()
                    .and_then(|name| if name.trim().is_empty() {
                        None
                    } else {
                        Some(name)
                    })
                    .or_else(|| extract_name(&path))
                    .unwrap_or_else(|| path.clone());

                store::PlugInfo::new(name.to_owned(), path.to_owned())
            })
        } else {
            None
        };

        dlg.destroy();

        res
    }
}

fn extract_name(path: &str) -> Option<String> {
    if let Some(idx) = path.rfind(|c| c == '/' || c == '\\') {
        if idx < path.len() - 1 {
            let path = path.trim_right_matches(".git");
            Some(path[idx + 1..].to_owned())
        } else {
            None
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_name() {
        assert_eq!(
            Some("plugin_name".to_owned()),
            extract_name("http://github.com/somebody/plugin_name.git")
        );
    }
}
