use std::sync::Arc;
use std::rc::Rc;
use std::cell::RefCell;
use std::ops::Deref;

use ui::UiMutex;

use gtk;
use gtk::prelude::*;

use super::manager;
use super::store::Store;

pub struct Ui<'a> {
    manager: &'a Arc<UiMutex<manager::Manager>>,
}

impl<'a> Ui<'a> {
    pub fn new(manager: &'a Arc<UiMutex<manager::Manager>>) -> Ui<'a> {
        manager.borrow_mut().update_state();

        Ui { manager }
    }

    pub fn show<T: IsA<gtk::Window>>(&mut self, parent: &T) {
        const OK_ID: i32 = 0;

        let dlg = gtk::Dialog::new_with_buttons(
            Some("Plug"),
            Some(parent),
            gtk::DialogFlags::empty(),
            &[("Ok", OK_ID)],
        );

        dlg.set_default_size(800, 600);
        let content = dlg.get_content_area();

        let header_bar = gtk::HeaderBar::new();

        let enable_swc = gtk::Switch::new();
        enable_swc.set_valign(gtk::Align::Center);

        let manager_ref = self.manager.clone();
        header_bar.pack_end(&enable_swc);

        header_bar.set_title("Plug");
        header_bar.set_show_close_button(true);
        header_bar.show_all();

        dlg.set_titlebar(&header_bar);

        let pages = SettingsPages::new();

        match self.manager.borrow_mut().plug_manage_state {
            manager::PlugManageState::Unknown => {
                add_help_tab(
                    &pages,
                    "<span foreground=\"red\">Note:</span> NeovimGtk plugin manager <b>disabled</b>!",
                );
            }
            manager::PlugManageState::VimPlug(ref store) => {
                enable_swc.set_state(store.is_enabled());
                add_help_tab(
                    &pages,
                    "<span foreground=\"red\">Note:</span> NeovimGtk plugin manager <b>disabled</b>!\n\
                                               NeovimGtk manages plugins use vim-plug as backend, so enable it disables vim-plug configuration.\n\
                                               Current configuration taken from your vim-plug",
                );
                self.add_plugin_list_tab(&pages, store);
            }
            manager::PlugManageState::NvimGtk(ref store) => {
                enable_swc.set_state(store.is_enabled());
                let get_plugins = gtk::Box::new(gtk::Orientation::Vertical, 0);
                // TODO:
                let get_plugins_lbl = gtk::Label::new("Get Plugins");
                pages.add_page(&get_plugins_lbl, &get_plugins, "get_plugins");

                self.add_plugin_list_tab(&pages, store);
            }
        }


        enable_swc.connect_state_set(move |_, state| {
            manager_ref.borrow_mut().store_mut().map(|s| {
                s.set_enabled(state)
            });
            Inhibit(false)
        });

        content.pack_start(&*pages, true, true, 0);
        content.show_all();


        match dlg.run() {
            OK_ID => {
                self.manager.borrow().save();
            }
            _ => (),
        }

        dlg.destroy();
    }

    fn add_plugin_list_tab(&self, pages: &SettingsPages, store: &Store) {
        let plugins = gtk::Box::new(gtk::Orientation::Vertical, 3);
        self.fill_plugin_list(&plugins, store);

        let plugins_lbl = gtk::Label::new("Plugins");
        pages.add_page(&plugins_lbl, &plugins, "plugins");
    }

    fn fill_plugin_list(&self, panel: &gtk::Box, store: &Store) {
        let scroll = gtk::ScrolledWindow::new(None, None);
        scroll.get_style_context().map(|c| c.add_class("view"));
        let plugs_panel = gtk::ListBox::new();
        plugs_panel.set_selection_mode(gtk::SelectionMode::None);

        for (idx, plug_info) in store.get_plugs().iter().enumerate() {
            let row = gtk::ListBoxRow::new();
            let row_container = gtk::Box::new(gtk::Orientation::Vertical, 5);
            row_container.set_border_width(5);
            let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 5);
            let vbox = gtk::Box::new(gtk::Orientation::Vertical, 5);

            let name_lbl = gtk::Label::new(None);
            name_lbl.set_markup(&format!("<b>{}</b>", plug_info.name.as_str()));
            name_lbl.set_halign(gtk::Align::Start);
            let url_lbl = gtk::Label::new(Some(plug_info.url.as_str()));
            url_lbl.set_halign(gtk::Align::Start);
            let remove_btn = gtk::Button::new_with_label("Remove");
            remove_btn.set_halign(gtk::Align::End);

            //let store_ref = store.clone();
            let panel_ref = panel.clone();
            let row_ref = row.clone();
            remove_btn.connect_clicked(move |_| {
                // store_ref.borrow_mut().remove(idx);
                row_ref.remove(row_ref.get_child().as_ref().unwrap());
                let undo_btn = gtk::Button::new_with_label("Undo");
                let row_container = gtk::Box::new(gtk::Orientation::Horizontal, 5);
                row_container.pack_end(&undo_btn, false, true, 0);
                row_ref.add(&row_container);
                row_container.show_all();
            });

            row_container.pack_start(&hbox, true, true, 0);
            vbox.pack_start(&name_lbl, true, true, 0);
            vbox.pack_start(&url_lbl, true, true, 0);
            hbox.pack_start(&vbox, true, true, 0);
            hbox.pack_start(&remove_btn, false, true, 0);

            row.add(&row_container);
            plugs_panel.add(&row);
        }

        scroll.add(&plugs_panel);
        panel.pack_start(&scroll, true, true, 0);
    }
}

fn add_help_tab(pages: &SettingsPages, markup: &str) {
    let help = gtk::Box::new(gtk::Orientation::Vertical, 3);
    let label = gtk::Label::new(None);
    label.set_markup(markup);
    help.pack_start(&label, true, false, 0);

    let help_lbl = gtk::Label::new("Help");
    pages.add_page(&help_lbl, &help, "help");
}

struct SettingsPages {
    categories: gtk::ListBox,
    stack: gtk::Stack,
    content: gtk::Box,
    rows: Rc<RefCell<Vec<(gtk::ListBoxRow, &'static str)>>>,
}

impl SettingsPages {
    pub fn new() -> Self {
        let content = gtk::Box::new(gtk::Orientation::Horizontal, 5);
        let categories = gtk::ListBox::new();
        categories.get_style_context().map(|c| c.add_class("view"));
        let stack = gtk::Stack::new();
        stack.set_transition_type(gtk::StackTransitionType::Crossfade);
        let rows: Rc<RefCell<Vec<(gtk::ListBoxRow, &'static str)>>> =
            Rc::new(RefCell::new(Vec::new()));

        content.pack_start(&categories, false, true, 0);
        content.pack_start(&stack, true, true, 0);

        let rows_ref = rows.clone();
        let stack_ref = stack.clone();
        categories.connect_row_selected(move |_, row| if let &Some(ref row) = row {
            if let Some(ref r) = rows_ref.borrow().iter().find(|r| r.0 == *row) {
                if let Some(child) = stack_ref.get_child_by_name(&r.1) {
                    stack_ref.set_visible_child(&child);
                }

            }
        });

        SettingsPages {
            categories,
            stack,
            content,
            rows,
        }
    }

    fn add_page<W: gtk::IsA<gtk::Widget>>(
        &self,
        label: &gtk::Label,
        widget: &W,
        name: &'static str,
    ) {
        let row = gtk::ListBoxRow::new();

        let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        hbox.set_border_width(12);
        hbox.pack_start(label, false, true, 0);
        row.add(&hbox);

        self.categories.add(&row);
        self.stack.add_named(widget, name);
        self.rows.borrow_mut().push((row, name));
    }
}

impl Deref for SettingsPages {
    type Target = gtk::Box;

    fn deref(&self) -> &gtk::Box {
        &self.content
    }
}
