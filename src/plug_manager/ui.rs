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

        let content = dlg.get_content_area();
        let tabs = gtk::Notebook::new();

        let vim_plug_state = self.get_state();
        match vim_plug_state {
            vim_plug::State::AlreadyLoaded => {
                let get_plugins = gtk::Box::new(gtk::Orientation::Vertical, 0);
                let warn_lbl = gtk::Label::new(
                    "vim-plug manager already loaded.\n\
                                               NeovimGtk manages plugins using vim-plug as backend.\n\
                                               To allow NeovimGtk manage plugins please disable vim-plug in your configuration.\n\
                                               You can convert vim-plug configuration to NeovimGtk conviguration using button below.\n\
                                               List of current vim-plug plugins can be found in 'Plugins' tab.",
                );
                get_plugins.add(&warn_lbl);

                let copy_btn = gtk::Button::new_with_label("Copy plugins from current vim-plug configuration");
                get_plugins.add(&copy_btn);

                let get_plugins_lbl = gtk::Label::new("Help");
                tabs.append_page(&get_plugins, Some(&get_plugins_lbl));
            }
            vim_plug::State::Unknown => {
                let get_plugins = gtk::Box::new(gtk::Orientation::Vertical, 0);
                let get_plugins_lbl = gtk::Label::new("Get Plugins");
                tabs.append_page(&get_plugins, Some(&get_plugins_lbl));
            }
        }

        let plugins = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let store = self.manager.load_store(&vim_plug_state);

        self.fill_plugin_list(&plugins, &store);

        let plugins_lbl = gtk::Label::new("Plugins");
        tabs.append_page(&plugins, Some(&plugins_lbl));

        tabs.set_tab_pos(gtk::PositionType::Left);
        content.add(&tabs);
        content.show_all();


        match dlg.run() {
            OK_ID => {
                println!("TODO:");
            }
            _ => (),
        }

        dlg.destroy();
    }

    fn fill_plugin_list(&self, panel: &gtk::Box, store: &Store) {
        let tree = gtk::TreeView::new();
        let scroll = gtk::ScrolledWindow::new(None, None);

        tree.set_headers_visible(false);
        tree.set_can_focus(false);
        scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        scroll.add(&tree);

        panel.add(&scroll);

        let copy_btn = gtk::Button::new_with_label("Copy plugins from current vim-plug configuration");
        panel.add(&copy_btn);
    }

    fn get_state(&self) -> vim_plug::State {
        self.manager.vim_plug.get_state()
    }
}

