use std::rc::Rc;
use std::cell::RefCell;

use gtk;
use gtk::prelude::*;
use glib;
use pango::FontDescription;
use gdk::{EventButton, EventType};

use neovim_lib::{Neovim, NeovimApi};

use nvim::ErrorReport;

use input;

const MIN_CONTENT_HEIGHT: i32 = 250;

pub struct PopupMenu {
    popover: gtk::Popover,
    tree: gtk::TreeView,
}

impl PopupMenu {
    pub fn new(drawing: &gtk::DrawingArea,
               nvim: Rc<RefCell<Neovim>>,
               font_desc: &FontDescription,
               menu_items: &Vec<Vec<&str>>,
               selected: i64,
               x: i32,
               y: i32,
               width: i32,
               height: i32)
               -> PopupMenu {
        let popover = gtk::Popover::new(Some(drawing));
        popover.set_modal(false);

        let tree = create_list(menu_items, font_desc);
        tree.set_can_focus(false);

        let nvim_ref = nvim.clone();
        tree.connect_button_press_event(move |tree, ev| {
                                            tree_button_press(tree, ev, &mut *nvim_ref.borrow_mut())
                                        });

        let scroll = gtk::ScrolledWindow::new(None, None);
        scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        scroll.set_min_content_height(MIN_CONTENT_HEIGHT);

        scroll.add(&tree);
        scroll.show_all();
        popover.add(&scroll);
        popover.set_pointing_to(&gtk::Rectangle {
                                     x,
                                     y,
                                     width,
                                     height,
                                 });

        popover.connect_key_press_event(move |_, ev| {
                                            input::gtk_key_press(&mut *nvim.borrow_mut(), ev)
                                        });

        let popup = PopupMenu { popover, tree };

        popup.select(selected);

        popup
    }

    pub fn show(&self) {
        self.popover.popup();
    }

    pub fn hide(self) {
        self.popover.destroy();
    }

    pub fn select(&self, selected: i64) {
        if selected >= 0 {
            let selected_path = gtk::TreePath::new_from_string(&format!("{}", selected));
            self.tree.get_selection().select_path(&selected_path);
            self.tree
                .scroll_to_cell(Some(&selected_path), None, false, 0.0, 0.0);
        } else {
            self.tree.get_selection().unselect_all();
        }
    }
}

fn create_list(menu: &Vec<Vec<&str>>, font_desc: &FontDescription) -> gtk::TreeView {
    let tree = gtk::TreeView::new();

    if menu.is_empty() {
        return tree;
    }
    let columns = menu.get(0).unwrap().len();

    let font_str = font_desc.to_string();
    for i in 0..columns {
        append_column(&tree, i as i32, &font_str);
    }

    let list_store = gtk::ListStore::new(&vec![gtk::Type::String; columns]);
    let all_column_ids: Vec<u32> = (0..columns).map(|i| i as u32).collect();

    for line in menu {
        let line_array: Vec<&glib::ToValue> = line.iter().map(|v| v as &glib::ToValue).collect();
        list_store.insert_with_values(None, &all_column_ids, &line_array[..]);
    }

    tree.set_model(Some(&list_store));
    tree.set_headers_visible(false);

    tree
}

fn append_column(tree: &gtk::TreeView, id: i32, font_str: &str) {
    let renderer = gtk::CellRendererText::new();
    renderer.set_property_font(Some(font_str));

    let column = gtk::TreeViewColumn::new();
    column.pack_start(&renderer, true);
    column.add_attribute(&renderer, "text", id);
    tree.append_column(&column);
}

fn tree_button_press(tree: &gtk::TreeView, ev: &EventButton, nvim: &mut Neovim) -> Inhibit {
    if ev.get_event_type() != EventType::ButtonPress {
        return Inhibit(false);
    }

    let (paths, ..) = tree.get_selection().get_selected_rows();
    let selected_idx = if !paths.is_empty() {
        let ids = paths[0].get_indices();
        if !ids.is_empty() { ids[0] } else { -1 }
    } else {
        -1
    };

    let (x, y) = ev.get_position();
    if let Some((Some(tree_path), ..)) = tree.get_path_at_pos(x as i32, y as i32) {
        let target_idx = tree_path.get_indices()[0];

        let scroll_count = find_scroll_count(selected_idx, target_idx);

        let mut apply_command = String::new();

        for _ in 0..scroll_count {
            if target_idx > selected_idx {
                apply_command.push_str("<C-n>");
            } else {
                apply_command.push_str("<C-p>");
            }
        }
        apply_command.push_str("<C-y>");

        nvim.input(&apply_command).report_err(nvim);
    }

    Inhibit(false)
}

fn find_scroll_count(selected_idx: i32, target_idx: i32) -> i32 {
    if selected_idx < 0 {
        target_idx + 1
    } else if target_idx > selected_idx {
        target_idx - selected_idx
    } else {
        selected_idx - target_idx
    }
}
