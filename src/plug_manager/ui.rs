use std::rc::Rc;
use std::cell::RefCell;

use gtk;
use gtk::prelude::*;

use super::manager;
use super::vim_plug;
use super::store::Store;

pub struct Ui<'a> {
    manager: &'a manager::Manager,
}

impl<'a> Ui<'a> {
    pub fn new(manager: &'a manager::Manager) -> Ui<'a> {
        Ui { manager }
    }

    pub fn show<T: IsA<gtk::Window>>(&self, parent: &T) {
        const OK_ID: i32 = 0;

        let dlg = gtk::Dialog::new_with_buttons(
            Some("Plug"),
            Some(parent),
            gtk::DialogFlags::empty(),
            &[("Ok", OK_ID)],
        );

        dlg.set_default_size(800, 600);
        let content = dlg.get_content_area();
        let tabs = gtk::Notebook::new();

        let vim_plug_state = self.get_state();
        match vim_plug_state {
            vim_plug::State::AlreadyLoaded => {
                let help = gtk::Box::new(gtk::Orientation::Vertical, 3);
                let warn_lbl = gtk::Label::new(None);
                warn_lbl.set_markup("<span foreground=\"red\">Note:</span> <b>vim-plug</b> manager already loaded!\n\
                                               NeovimGtk plugins manager will be <b>disabled</b>.\n\
                                               To enable it please disable vim-plug in your configuration.\n\
                                               NeovimGtk manages plugins use vim-plug as backend.\n\
                                               You can convert vim-plug configuration to NeovimGtk configuration using button below.\n\
                                               List of current vim-plug plugins can be found in 'Plugins' tab.");
                help.pack_start(&warn_lbl, true, false, 0);

                let copy_btn =
                    gtk::Button::new_with_label("Copy plugins from current vim-plug configuration");
                help.pack_start(&copy_btn, false, false, 0);

                let get_plugins_lbl = gtk::Label::new("Help");
                tabs.append_page(&help, Some(&get_plugins_lbl));
            }
            vim_plug::State::Unknown => {
                let get_plugins = gtk::Box::new(gtk::Orientation::Vertical, 0);
                let get_plugins_lbl = gtk::Label::new("Get Plugins");
                tabs.append_page(&get_plugins, Some(&get_plugins_lbl));
            }
        }

        let plugins = gtk::Box::new(gtk::Orientation::Vertical, 3);
        let store = self.manager.load_store(&vim_plug_state);

        let store = Rc::new(RefCell::new(store));
        Ui::fill_plugin_list(&plugins, &store);

        let plugins_lbl = gtk::Label::new("Plugins");
        tabs.append_page(&plugins, Some(&plugins_lbl));

        tabs.set_tab_pos(gtk::PositionType::Left);
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

    fn fill_plugin_list(panel: &gtk::Box, store: &Rc<RefCell<Store>>) {
        let scroll = gtk::ScrolledWindow::new(None, None);
        let plugs_panel = gtk::ListBox::new();
        plugs_panel.set_selection_mode(gtk::SelectionMode::None);

        for (idx, plug_info) in store.borrow().get_plugs().iter().enumerate() {
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
                panel_ref.remove(&row_ref);
            });

            row_container.pack_start(&hbox, true, true, 0);
            row_container.pack_start(&gtk::Separator::new(gtk::Orientation::Horizontal), true, true, 0);
            vbox.pack_start(&name_lbl, true, true, 0);
            vbox.pack_start(&url_lbl, true, true, 0);
            hbox.pack_start(&vbox, true, true, 0);
            hbox.pack_start(&remove_btn, false, true, 0);

            row.add(&row_container);
            plugs_panel.add(&row);
        }

        scroll.add(&plugs_panel);
        panel.pack_start(&scroll, true, true, 0);

        let copy_btn =
            gtk::Button::new_with_label("Copy plugins from current vim-plug configuration");
        panel.add(&copy_btn);
    }

    fn get_state(&self) -> vim_plug::State {
        self.manager.vim_plug.get_state()
    }
}
