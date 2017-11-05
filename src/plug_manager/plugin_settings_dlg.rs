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
        let path_lbl = gtk::Label::new("Repo");
        let path_e = gtk::Entry::new();

        path.pack_start(&path_lbl, true, true, 0);
        path.pack_end(&path_e, false, true, 0);

        list.add(&path);


        let name = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        let name_lbl = gtk::Label::new("Name");
        let name_e = gtk::Entry::new();

        name.pack_start(&name_lbl, true, true, 0);
        name.pack_end(&name_e, false, true, 0);

        list.add(&name);

        content.add(&list);
        content.show_all();

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
                    .or_else(|| Builder::extract_name(&path))
                    .unwrap_or_else(|| path.clone());

                store::PlugInfo::new(name.to_owned(), path.to_owned())
            })
        } else {
            None
        };

        dlg.destroy();

        res
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_name() {
        assert_eq!(
            Some("plugin_name".to_owned()),
            Builder::extract_name("http://github.com/somebody/plugin_name.git")
        );
    }
}
