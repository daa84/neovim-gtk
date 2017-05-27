use std::ops::Deref;

use gtk;
use gtk::prelude::*;

use neovim_lib::neovim_api::Tabpage;

pub struct Tabline {
    tabs: gtk::Notebook,
}

impl Tabline {
    pub fn new() -> Self {
        let tabs = gtk::Notebook::new();

        tabs.set_can_focus(false);
        tabs.set_scrollable(true);
        tabs.set_show_border(false);
        tabs.set_border_width(0);

        Tabline {  
            tabs,
        }
    }

    pub fn update_tabs(&self, selected: &Tabpage, tabs: &Vec<(Tabpage, Option<&str>)>) {
        let count = self.tabs.get_n_pages() as usize;
        if count < tabs.len() {
            for _ in count..tabs.len() {
                let empty = gtk::Box::new(gtk::Orientation::Vertical, 0);
                empty.show_all();
                self.tabs.append_page(&empty, Some(&gtk::Label::new("AA")));
            }
        }
        else if count > tabs.len() {
            for _ in tabs.len()..count {
                self.tabs.remove_page(None);
            }
        }

        // TODO: current page
        for (idx, tab) in tabs.iter().enumerate() {
            let tab_child = self.tabs.get_nth_page(Some(idx as u32));
            self.tabs.set_tab_label_text(&tab_child.unwrap(), &tab.1.unwrap_or("??"));
        }
    }
}

impl Deref for Tabline {
    type Target = gtk::Notebook;

    fn deref(&self) -> &gtk::Notebook {
        &self.tabs
    }
}
