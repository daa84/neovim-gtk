use std::ops::Deref;
use std::rc::Rc;
use std::cell::RefCell;

use gtk;
use gtk::prelude::*;

use glib;
use glib::signal;

use pango;

use neovim_lib::NeovimApi;
use neovim_lib::neovim_api::Tabpage;

use nvim;
use nvim::ErrorReport;

struct State {
    data: Vec<Tabpage>,
    selected: Option<Tabpage>,
    nvim: Option<Rc<nvim::NeovimClient>>,
}

impl State {
    pub fn new() -> Self {
        State {
            data: Vec::new(),
            selected: None,
            nvim: None,
        }
    }

    fn switch_page(&self, idx: u32) {
        let target = &self.data[idx as usize];
        if Some(target) != self.selected.as_ref() {
            if let Some(mut nvim) = self.nvim.as_ref().unwrap().nvim() {
                nvim.set_current_tabpage(target).report_err();
            }
        }
    }

    fn close_tab(&self, idx: u32) {
        if let Some(mut nvim) = self.nvim.as_ref().unwrap().nvim() {
            nvim.command(&format!(":tabc {}", idx + 1)).report_err();
        }
    }
}

pub struct Tabline {
    tabs: gtk::Notebook,
    state: Rc<RefCell<State>>,
    switch_handler_id: glib::SignalHandlerId,
}

impl Tabline {
    pub fn new() -> Self {
        let tabs = gtk::Notebook::new();

        tabs.set_can_focus(false);
        tabs.set_scrollable(true);
        tabs.set_show_border(false);
        tabs.set_border_width(0);
        tabs.set_hexpand(true);
        tabs.hide();

        let state = Rc::new(RefCell::new(State::new()));

        let state_ref = state.clone();
        let switch_handler_id =
            tabs.connect_switch_page(move |_, _, idx| state_ref.borrow().switch_page(idx));

        Tabline {
            tabs,
            state,
            switch_handler_id,
        }
    }

    fn update_state(
        &self,
        nvim: &Rc<nvim::NeovimClient>,
        selected: &Tabpage,
        tabs: &[(Tabpage, Option<String>)],
    ) {
        let mut state = self.state.borrow_mut();

        if state.nvim.is_none() {
            state.nvim = Some(nvim.clone());
        }

        state.selected = Some(selected.clone());

        state.data = tabs.iter().map(|item| item.0.clone()).collect();
    }

    pub fn update_tabs(
        &self,
        nvim: &Rc<nvim::NeovimClient>,
        selected: &Tabpage,
        tabs: &[(Tabpage, Option<String>)],
    ) {
        if tabs.len() <= 1 {
            self.tabs.hide();
            return;
        } else {
            self.tabs.show();
        }

        self.update_state(nvim, selected, tabs);


        signal::signal_handler_block(&self.tabs, &self.switch_handler_id);

        let count = self.tabs.get_n_pages() as usize;
        if count < tabs.len() {
            for _ in count..tabs.len() {
                let empty = gtk::Box::new(gtk::Orientation::Vertical, 0);
                empty.show_all();
                let title = gtk::Label::new(None);
                title.set_ellipsize(pango::EllipsizeMode::Middle);
                title.set_width_chars(25);
                let close_btn = gtk::Button::new_from_icon_name(
                    "window-close-symbolic",
                    gtk::IconSize::Menu.into(),
                );
                close_btn.set_relief(gtk::ReliefStyle::None);
                close_btn.get_style_context().unwrap().add_class("small-button");
                close_btn.set_focus_on_click(false);
                let label_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
                label_box.pack_start(&title, true, false, 0);
                label_box.pack_start(&close_btn, false, false, 0);
                title.show();
                close_btn.show();
                self.tabs.append_page(&empty, Some(&label_box));

                let tabs = self.tabs.clone();
                let state_ref = Rc::clone(&self.state);
                close_btn.connect_clicked(move |btn| {
                    let current_label = btn
                        .get_parent().unwrap();
                    for i in 0..tabs.get_n_pages() {
                        let page = tabs.get_nth_page(Some(i)).unwrap();
                        let label = tabs.get_tab_label(&page).unwrap();
                        if label == current_label {
                            state_ref.borrow().close_tab(i);
                        }
                    }
                });
            }
        } else if count > tabs.len() {
            for _ in tabs.len()..count {
                self.tabs.remove_page(None);
            }
        }

        for (idx, tab) in tabs.iter().enumerate() {
            let tab_child = self.tabs.get_nth_page(Some(idx as u32));
            let tab_label = self.tabs
                .get_tab_label(&tab_child.unwrap())
                .unwrap()
                .downcast::<gtk::Box>()
                .unwrap()
                .get_children()
                .into_iter()
                .next()
                .unwrap()
                .downcast::<gtk::Label>()
                .unwrap();
            tab_label.set_text(tab.1.as_ref().unwrap_or(&"??".to_owned()));

            if *selected == tab.0 {
                self.tabs.set_current_page(Some(idx as u32));
            }
        }

        signal::signal_handler_unblock(&self.tabs, &self.switch_handler_id);
    }
}

impl Deref for Tabline {
    type Target = gtk::Notebook;

    fn deref(&self) -> &gtk::Notebook {
        &self.tabs
    }
}
