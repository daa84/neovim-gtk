use std::rc::Rc;
use std::cell::RefCell;

use gtk::prelude::*;
use gtk::{Window, WindowType, TreeView, TreeViewColumn, TreePath, CellRendererText, ListStore,
          Type, ScrolledWindow, PolicyType};
use glib;
use pango::FontDescription;
use gdk::{EventButton, EventType};

use neovim_lib::{Neovim, NeovimApi};

use nvim::ErrorReport;

use input;

const MIN_CONTENT_HEIGHT: i32 = 250;

pub struct PopupMenu {
    menu: Window,
    list: TreeView,
}

impl PopupMenu {
    pub fn new(nvim: Rc<RefCell<Neovim>>,
               font_desc: &FontDescription,
               menu: &Vec<Vec<&str>>,
               selected: i64,
               x: i32,
               y: i32,
               grow_up: bool)
               -> PopupMenu {
        let win = Window::new(WindowType::Popup);

        let tree = create_list(menu, font_desc);
        tree.set_can_focus(false);

        let nvim_ref = nvim.clone();
        tree.connect_button_press_event(move |tree, ev| tree_button_press(tree, ev, &mut *nvim_ref.borrow_mut()));

        let scroll = ScrolledWindow::new(None, None);
        scroll.set_policy(PolicyType::Never, PolicyType::Automatic);
        scroll.set_min_content_height(MIN_CONTENT_HEIGHT);

        scroll.add(&tree);
        win.add(&scroll);
        if grow_up {
            win.move_(x, y - MIN_CONTENT_HEIGHT);
        } else {
            win.move_(x, y);
        }

        win.connect_key_press_event(move |_, ev| input::gtk_key_press(&mut *nvim.borrow_mut(), ev));

        let popup = PopupMenu {
            menu: win,
            list: tree,
        };

        popup.select(selected);

        popup
    }

    pub fn show(&self) {
        self.menu.show_all();
    }

    pub fn hide(self) {
        self.menu.destroy();
    }

    pub fn select(&self, selected: i64) {
        if selected >= 0 {
            let selected_path = TreePath::new_from_string(&format!("{}", selected));
            self.list
                .get_selection()
                .select_path(&selected_path);
            self.list.scroll_to_cell(Some(&selected_path), None, false, 0.0, 0.0);
        } else {
            self.list.get_selection().unselect_all();
        }
    }
}

fn create_list(menu: &Vec<Vec<&str>>, font_desc: &FontDescription) -> TreeView {
    let tree = TreeView::new();

    if menu.is_empty() {
        return tree;
    }
    let columns = menu.get(0).unwrap().len();

    let font_str = font_desc.to_string();
    for i in 0..columns {
        append_column(&tree, i as i32, &font_str);
    }

    let list_store = ListStore::new(&vec![Type::String; columns]);
    let all_column_ids: Vec<u32> = (0..columns).map(|i| i as u32).collect();

    for line in menu {
        let line_array: Vec<&glib::ToValue> = line.iter().map(|v| v as &glib::ToValue).collect();
        list_store.insert_with_values(None, &all_column_ids, &line_array[..]);
    }

    tree.set_model(Some(&list_store));
    tree.set_headers_visible(false);

    tree
}

fn append_column(tree: &TreeView, id: i32, font_str: &str) {
    let renderer = CellRendererText::new();
    renderer.set_property_font(Some(font_str));

    let column = TreeViewColumn::new();
    column.pack_start(&renderer, true);
    column.add_attribute(&renderer, "text", id);
    tree.append_column(&column);
}

fn tree_button_press(tree: &TreeView, ev: &EventButton, nvim: &mut Neovim) -> Inhibit {
    if ev.get_event_type() != EventType::ButtonPress {
        return Inhibit(false);
    }

    let (paths, ..) = tree.get_selection().get_selected_rows();
    let selected_idx = if !paths.is_empty() {
        let ids = paths[0].get_indices();
        if !ids.is_empty() {
            ids[0]
        } else {
            -1
        }
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
