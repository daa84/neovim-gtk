use std::sync::Arc;
use std::rc::Rc;
use std::cell::RefCell;
use std::ops::Deref;

use ui::UiMutex;

use gtk;
use gtk::prelude::*;

use super::manager;
use super::store::{Store, PlugInfo};
use super::plugin_settings_dlg;
use super::vimawesome;

pub struct Ui<'a> {
    manager: &'a Arc<UiMutex<manager::Manager>>,
}

impl<'a> Ui<'a> {
    pub fn new(manager: &'a Arc<UiMutex<manager::Manager>>) -> Ui<'a> {
        manager.borrow_mut().update_state();

        Ui { manager }
    }

    pub fn show<T: IsA<gtk::Window>>(&mut self, parent: &T) {
        let dlg = gtk::Dialog::new_with_buttons(
            Some("Plug"),
            Some(parent),
            gtk::DIALOG_DESTROY_WITH_PARENT,
            &[("Ok", gtk::ResponseType::Ok.into())],
        );

        dlg.set_default_size(800, 600);
        let content = dlg.get_content_area();

        let header_bar = gtk::HeaderBar::new();

        let add_plug_btn = gtk::Button::new_with_label("Add..");
        header_bar.pack_end(&add_plug_btn);


        let enable_swc = gtk::Switch::new();
        enable_swc.set_valign(gtk::Align::Center);
        enable_swc.show();

        header_bar.pack_end(&enable_swc);

        header_bar.set_title("Plug");
        header_bar.set_show_close_button(true);
        header_bar.show();

        dlg.set_titlebar(&header_bar);

        let pages = SettingsPages::new(
            clone!(add_plug_btn => move |row_name| if row_name == "plugins" {
            add_plug_btn.show();
        } else {
            add_plug_btn.hide();
        }),
        );

        enable_swc.set_state(self.manager.borrow().store.is_enabled());

        let get_plugins = add_get_plugins_tab(&pages);

        match self.manager.borrow().plug_manage_state {
            manager::PlugManageState::Unknown => {
                add_help_tab(
                    &pages,
                    "<span foreground=\"red\">Note:</span> NeovimGtk plugin manager <b>disabled</b>!",
                );
            }
            manager::PlugManageState::VimPlug => {
                add_help_tab(
                    &pages,
                    "<span foreground=\"red\">Note:</span> NeovimGtk plugin manager <b>disabled</b>!\n\
                                               NeovimGtk manages plugins use vim-plug as backend, so enable it disables vim-plug configuration.\n\
                                               Current configuration taken from your vim-plug",
                );
            }
            manager::PlugManageState::NvimGtk => {}
        }

        let plugs_panel = self.add_plugin_list_tab(&pages, &self.manager.borrow().store);

        let manager_ref = self.manager.clone();
        enable_swc.connect_state_set(move |_, state| {
            manager_ref.borrow_mut().store.set_enabled(state);
            Inhibit(false)
        });

        let manager_ref = self.manager.clone();
        add_plug_btn.connect_clicked(clone!(dlg => move |_| {
            add_plugin(&dlg, &manager_ref, &plugs_panel);
        }));

        content.pack_start(&*pages, true, true, 0);
        content.show_all();

        let get_plugins = UiMutex::new(get_plugins);
        vimawesome::call(move |res| {
            let panel = get_plugins.borrow();
            for child in panel.get_children() {
                panel.remove(&child);
            }
            match res {
                Ok(list) => {
                    let result = vimawesome::build_result_panel(&list);
                    panel.pack_start(&result, true, true, 0);
                }
                Err(e) => {
                    panel.pack_start(&gtk::Label::new(format!("{}", e).as_str()), false, true, 0);
                    error!("{}", e)
                },
            }
        });

        let ok: i32 = gtk::ResponseType::Ok.into();
        if dlg.run() == ok {
            let mut manager = self.manager.borrow_mut();
            manager.clear_removed();
            manager.save();
        }

        dlg.destroy();
    }

    fn add_plugin_list_tab(&self, pages: &SettingsPages, store: &Store) -> gtk::ListBox {
        let plugins = gtk::Box::new(gtk::Orientation::Vertical, 3);
        let plugs_panel = self.fill_plugin_list(&plugins, store);

        let plugins_lbl = gtk::Label::new("Plugins");
        pages.add_page(&plugins_lbl, &plugins, "plugins");
        plugs_panel
    }

    fn fill_plugin_list(&self, panel: &gtk::Box, store: &Store) -> gtk::ListBox {
        let scroll = gtk::ScrolledWindow::new(None, None);
        scroll.get_style_context().map(|c| c.add_class("view"));
        let plugs_panel = gtk::ListBox::new();

        for (idx, plug_info) in store.get_plugs().iter().enumerate() {
            let row = create_plug_row(idx, plug_info, &self.manager);

            plugs_panel.add(&row);
        }

        scroll.add(&plugs_panel);
        panel.pack_start(&scroll, true, true, 0);

        plugs_panel
    }
}

fn create_plug_row(
    plug_idx: usize,
    plug_info: &PlugInfo,
    manager: &Arc<UiMutex<manager::Manager>>,
) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    let row_container = gtk::Box::new(gtk::Orientation::Vertical, 5);
    row_container.set_border_width(5);
    let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 5);
    let label_box = create_plug_label(plug_info);


    let button_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    button_box.set_halign(gtk::Align::End);

    let exists_button_box = gtk::Box::new(gtk::Orientation::Horizontal, 5);

    let remove_btn = gtk::Button::new_with_label("Remove");
    exists_button_box.pack_start(&remove_btn, false, true, 0);

    let undo_btn = gtk::Button::new_with_label("Undo");


    row_container.pack_start(&hbox, true, true, 0);
    hbox.pack_start(&label_box, true, true, 0);
    button_box.pack_start(&exists_button_box, false, true, 0);
    hbox.pack_start(&button_box, false, true, 0);

    row.add(&row_container);


    remove_btn.connect_clicked(
        clone!(manager, label_box, button_box, exists_button_box, undo_btn => move |_| {
                    label_box.set_sensitive(false);
                    button_box.remove(&exists_button_box);
                    button_box.pack_start(&undo_btn, false, true, 0);
                    button_box.show_all();
                    manager.borrow_mut().store.remove_plug(plug_idx);
                }),
    );

    undo_btn.connect_clicked(
        clone!(manager, label_box, button_box, exists_button_box, undo_btn => move |_| {
                    label_box.set_sensitive(true);
                    button_box.remove(&undo_btn);
                    button_box.pack_start(&exists_button_box, false, true, 0);
                    button_box.show_all();
                    manager.borrow_mut().store.restore_plug(plug_idx);
                }),
    );

    row
}

fn add_plugin<F: IsA<gtk::Window>>(
    parent: &F,
    manager: &Arc<UiMutex<manager::Manager>>,
    plugs_panel: &gtk::ListBox,
) {
    if let Some(new_plugin) = plugin_settings_dlg::Builder::new("Add plugin").show(parent) {
        let row = create_plug_row(manager.borrow().store.plugs_count(), &new_plugin, manager);
        row.show_all();
        plugs_panel.add(&row);

        manager.borrow_mut().add_plug(new_plugin);
    }
}

fn create_plug_label(plug_info: &PlugInfo) -> gtk::Box {
    let label_box = gtk::Box::new(gtk::Orientation::Vertical, 5);

    let name_lbl = gtk::Label::new(None);
    name_lbl.set_markup(&format!("<b>{}</b>", plug_info.name));
    name_lbl.set_halign(gtk::Align::Start);
    let url_lbl = gtk::Label::new(Some(plug_info.get_plug_path().as_str()));
    url_lbl.set_halign(gtk::Align::Start);


    label_box.pack_start(&name_lbl, true, true, 0);
    label_box.pack_start(&url_lbl, true, true, 0);
    label_box
}

fn add_get_plugins_tab(pages: &SettingsPages) -> gtk::Box {
    let get_plugins = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let spinner = gtk::Spinner::new();
    let get_plugins_lbl = gtk::Label::new("Get Plugins");
    pages.add_page(&get_plugins_lbl, &get_plugins, "get_plugins");

    let list_panel = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let link_button = gtk::Label::new(None);
    link_button.set_markup("Plugins source: <a href=\"https://vimawesome.com\">https://vimawesome.com</a>");
    get_plugins.pack_start(&link_button, false, true, 15);
    get_plugins.pack_start(&list_panel, true, true, 0);
    list_panel.pack_start(&spinner, true, true, 0);
    spinner.start();

    list_panel
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
    pub fn new<F: Fn(&str) + 'static>(row_selected: F) -> Self {
        let content = gtk::Box::new(gtk::Orientation::Horizontal, 5);
        let categories = gtk::ListBox::new();
        categories.get_style_context().map(|c| c.add_class("view"));
        let stack = gtk::Stack::new();
        stack.set_transition_type(gtk::StackTransitionType::Crossfade);
        let rows: Rc<RefCell<Vec<(gtk::ListBoxRow, &'static str)>>> =
            Rc::new(RefCell::new(Vec::new()));

        content.pack_start(&categories, false, true, 0);
        content.pack_start(&stack, true, true, 0);

        categories.connect_row_selected(
            clone!(stack, rows => move |_, row| if let &Some(ref row) = row {
            if let Some(ref r) = rows.borrow().iter().find(|r| r.0 == *row) {
                if let Some(child) = stack.get_child_by_name(&r.1) {
                    stack.set_visible_child(&child);
                    row_selected(&r.1);
                }

            }
        }),
        );

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
