use std::ops::Deref;
use std::rc::Rc;
use std::cell::RefCell;

use gtk;
use gtk::prelude::*;

use neovim_lib::{Neovim, NeovimApi};
use neovim_lib::neovim_api::Tabpage;

use nvim::ErrorReport;

struct State {
    data: Vec<Tabpage>,
    nvim: Option<Rc<RefCell<Neovim>>>,
}

impl State {
    pub fn new() -> Self {
        State { 
            data: Vec::new(),
            nvim: None,
        }
    }

    fn change_current_page(&self, idx: i32) -> bool {
        let mut nvim = self.nvim.as_ref().unwrap().borrow_mut();
        nvim.set_current_tabpage(&self.data[idx as usize]).report_err(&mut *nvim);
        true
    }
}

pub struct Tabline {
    tabs: gtk::Notebook,
    state: Rc<RefCell<State>>,
}

impl Tabline {
    pub fn new() -> Self {
        let tabs = gtk::Notebook::new();

        tabs.set_can_focus(false);
        tabs.set_scrollable(true);
        tabs.set_show_border(false);
        tabs.set_border_width(0);
        tabs.hide();

        let state = Rc::new(RefCell::new(State::new()));

        let state_ref = state.clone();
        tabs.connect_change_current_page(move |_, idx| state_ref.borrow().change_current_page(idx));

        Tabline {  
            tabs,
            state,
        }
    }

    pub fn update_tabs(&self, nvim: &Rc<RefCell<Neovim>>, selected: &Tabpage, tabs: &Vec<(Tabpage, Option<&str>)>) {
        if tabs.len() <= 1 {
            self.tabs.hide();
            return;
        } else {
            self.tabs.show();
        }

        let mut state = self.state.borrow_mut();

        if state.nvim.is_none() {
            state.nvim = Some(nvim.clone());
        }

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

        state.data.clear();

        for (idx, tab) in tabs.iter().enumerate() {
            let tab_child = self.tabs.get_nth_page(Some(idx as u32));
            self.tabs.set_tab_label_text(&tab_child.unwrap(), &tab.1.unwrap_or("??"));
            state.data.push(tab.0.clone());

            if *selected == tab.0 {
                self.tabs.set_current_page(Some(idx as u32));
            }
        }
    }
}

impl Deref for Tabline {
    type Target = gtk::Notebook;

    fn deref(&self) -> &gtk::Notebook {
        &self.tabs
    }
}
