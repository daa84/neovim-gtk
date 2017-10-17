use gtk;
use gtk::prelude::*;

use super::manager;

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

        match self.get_state() {
            manager::State::AlreadyLoaded => {
                let get_plugins = gtk::Box::new(gtk::Orientation::Vertical, 0);
                let warn_lbl = gtk::Label::new(
                    "Plug manager already loaded.\n\
                                               NeovimGtk manages plugins using vim-plug as backend.\n\
                                               To allow NeovimGtk manage plugins please disable vim-plug in your configuration",
                );
                get_plugins.add(&warn_lbl);
                let get_plugins_lbl = gtk::Label::new("Help");
                tabs.append_page(&get_plugins, Some(&get_plugins_lbl));
            }
            manager::State::Unknown => {
                let get_plugins = gtk::Box::new(gtk::Orientation::Vertical, 0);
                let get_plugins_lbl = gtk::Label::new("Get Plugins");
                tabs.append_page(&get_plugins, Some(&get_plugins_lbl));
            }
        }

        let plugins = gtk::Box::new(gtk::Orientation::Vertical, 0);
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

    fn get_state(&self) -> manager::State {
        self.manager.get_state()
    }
}
