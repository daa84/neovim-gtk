use std::sync::Arc;

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
        let top_panel = gtk::Box::new(gtk::Orientation::Horizontal, 3);

        let tabs = gtk::Notebook::new();
        tabs.set_tab_pos(gtk::PositionType::Left);

        let enable_swc = gtk::Switch::new();

        match self.manager.borrow_mut().plug_manage_state {
            manager::PlugManageState::Unknown => {
                let help = gtk::Box::new(gtk::Orientation::Vertical, 3);
                let warn_lbl = gtk::Label::new(None);
                warn_lbl.set_markup("<span foreground=\"red\">Note:</span> NeovimGtk plugin manager <b>disabled</b>!");
                help.pack_start(&warn_lbl, true, false, 0);

                let get_plugins_lbl = gtk::Label::new("Help");
                tabs.append_page(&help, Some(&get_plugins_lbl));
            }
            manager::PlugManageState::Configuration(ref store) => {
                let help = gtk::Box::new(gtk::Orientation::Vertical, 3);
                let warn_lbl = gtk::Label::new(None);
                warn_lbl.set_markup("<span foreground=\"red\">Note:</span> NeovimGtk plugin manager <b>disabled</b>!\n\
                                               NeovimGtk manages plugins use vim-plug as backend, so enable it disables vim-plug configuration.");
                help.pack_start(&warn_lbl, true, false, 0);

                let get_plugins_lbl = gtk::Label::new("Help");
                tabs.append_page(&help, Some(&get_plugins_lbl));


                self.add_plugin_list_tab(&tabs, store);
            }
            manager::PlugManageState::NvimGtk(ref store) => {
                let get_plugins = gtk::Box::new(gtk::Orientation::Vertical, 0);
                let get_plugins_lbl = gtk::Label::new("Get Plugins");
                tabs.append_page(&get_plugins, Some(&get_plugins_lbl));

                self.add_plugin_list_tab(&tabs, store);
            }
        }


        top_panel.pack_end(&enable_swc, false, false, 0);

        content.pack_start(&top_panel, false, true, 3);
        content.pack_start(&tabs, true, true, 0);
        content.show_all();


        match dlg.run() {
            OK_ID => {
                println!("TODO:");
            }
            _ => (),
        }

        dlg.destroy();
    }

    fn add_plugin_list_tab(&self, tabs: &gtk::Notebook, store: &Store) {
        // Plugins
        let plugins = gtk::Box::new(gtk::Orientation::Vertical, 3);
        self.fill_plugin_list(&plugins, store);

        let plugins_lbl = gtk::Label::new("Plugins");
        tabs.append_page(&plugins, Some(&plugins_lbl));
    }

    fn fill_plugin_list(&self, panel: &gtk::Box, store: &Store) {
        let scroll = gtk::ScrolledWindow::new(None, None);
        let plugs_panel = gtk::ListBox::new();
        plugs_panel.set_selection_mode(gtk::SelectionMode::None);

        for (idx, plug_info) in store.get_plugs().iter().enumerate() {
            let row = gtk::ListBoxRow::new();
            let row_container = gtk::Box::new(gtk::Orientation::Vertical, 5);
            let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 5);
            let vbox = gtk::Box::new(gtk::Orientation::Vertical, 5);

            let name_lbl = gtk::Label::new(None);
            name_lbl.set_markup(&format!("<b>{}</b>", plug_info.name.as_str()));
            name_lbl.set_halign(gtk::Align::Start);
            let url_lbl = gtk::Label::new(Some(plug_info.url.as_str()));
            url_lbl.set_halign(gtk::Align::Start);
            let remove_btn = gtk::Button::new_with_label("Remove");
            remove_btn.set_halign(gtk::Align::End);

            let store_ref = store.clone();
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
            row_container.pack_start(
                &gtk::Separator::new(gtk::Orientation::Horizontal),
                true,
                true,
                0,
            );
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
