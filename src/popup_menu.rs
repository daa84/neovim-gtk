use std::rc::Rc;
use std::cell::RefCell;
use std::cmp::min;

use gtk;
use gtk::prelude::*;
use glib;
use gdk::{EventButton, EventType};
use pango::{self, LayoutExt};

use neovim_lib::{Neovim, NeovimApi};

use color::ColorModel;
use nvim::{self, ErrorReport, CompleteItem};
use shell;
use input;

const MAX_VISIBLE_ROWS: i32 = 10;

struct State {
    nvim: Option<Rc<nvim::NeovimClient>>,
    renderer: gtk::CellRendererText,
    tree: gtk::TreeView,
    scroll: gtk::ScrolledWindow,
    css_provider: gtk::CssProvider,
    info_label: gtk::Label,
    word_column: gtk::TreeViewColumn,
    kind_column: gtk::TreeViewColumn,
    menu_column: gtk::TreeViewColumn,
}

impl State {
    pub fn new() -> Self {
        let tree = gtk::TreeView::new();
        let css_provider = gtk::CssProvider::new();

        let style_context = tree.get_style_context().unwrap();
        style_context.add_provider(&css_provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);

        let renderer = gtk::CellRendererText::new();
        renderer.set_property_ellipsize(pango::EllipsizeMode::End);

        // word
        let word_column = gtk::TreeViewColumn::new();
        word_column.pack_start(&renderer, true);
        word_column.add_attribute(&renderer, "text", 0);
        tree.append_column(&word_column);

        // kind
        let kind_column = gtk::TreeViewColumn::new();
        kind_column.pack_start(&renderer, true);
        kind_column.add_attribute(&renderer, "text", 1);
        tree.append_column(&kind_column);

        // menu
        let menu_column = gtk::TreeViewColumn::new();
        menu_column.pack_start(&renderer, true);
        menu_column.add_attribute(&renderer, "text", 2);
        tree.append_column(&menu_column);

        let info_label = gtk::Label::new(None);
        info_label.set_line_wrap(true);

        State {
            nvim: None,
            tree,
            scroll: gtk::ScrolledWindow::new(None, None),
            renderer,
            css_provider,
            info_label,
            word_column,
            kind_column,
            menu_column,
        }
    }

    fn before_show(&mut self, shell: &shell::State, menu_items: &[CompleteItem], selected: i64) {
        if self.nvim.is_none() {
            self.nvim = Some(shell.nvim_clone());
        }

        let max_width = shell.drawing_area.get_allocated_width();
        self.scroll.set_max_content_width(max_width - 20);
        self.scroll.set_propagate_natural_width(true);
        self.update_tree(menu_items, shell);
        self.select(selected);
    }

    fn limit_column_widths(&self, menu: &[CompleteItem], shell: &shell::State) {
        let layout = shell.font_ctx.create_layout();
        let kind_chars = menu.iter().map(|i| i.kind.len()).max().unwrap();
        let max_width = self.scroll.get_max_content_width();
        let (xpad, _) = self.renderer.get_padding();

        const DEFAULT_PADDING: i32 = 5;

        if kind_chars > 0 {
            layout.set_text("[v]");
            let (kind_width, _) = layout.get_pixel_size();

            self.word_column.set_fixed_width(max_width - kind_width);

            self.kind_column.set_fixed_width(kind_width + xpad * 2 + DEFAULT_PADDING);
            self.kind_column.set_visible(true);
        } else {
            let max_line = menu.iter().max_by_key(|m| m.word.len()).unwrap();
            layout.set_text(max_line.word);
            let (word_max_width, _) = layout.get_pixel_size();

            self.kind_column.set_visible(false);

            let word_column_width = word_max_width + xpad * 2 + DEFAULT_PADDING;
            if word_column_width > max_width {
                self.word_column.set_fixed_width(max_width);
            } else {
                self.word_column.set_fixed_width(word_column_width);
            }
        }


        let max_line = menu.iter().max_by_key(|m| m.menu.len()).unwrap();

        if max_line.menu.len() > 0 {
            layout.set_text(max_line.menu);
            let (menu_max_width, _) = layout.get_pixel_size();
            self.menu_column.set_fixed_width(menu_max_width + xpad * 2 + DEFAULT_PADDING);
            self.menu_column.set_visible(true);
        } else {
            self.menu_column.set_visible(false);
        }
    }

    fn update_tree(&self, menu: &[CompleteItem], shell: &shell::State) {
        if menu.is_empty() {
            return;
        }

        self.limit_column_widths(menu, shell);

        self.renderer.set_property_font(
            Some(&shell.get_font_desc().to_string()),
        );

        let color_model = &shell.color_model;
        self.renderer.set_property_foreground_rgba(
            Some(&color_model.pmenu_fg().into()),
        );
        self.renderer.set_property_background_rgba(
            Some(&color_model.pmenu_bg().into()),
        );

        self.update_css(color_model);

        let list_store = gtk::ListStore::new(&vec![gtk::Type::String; 4]);
        let all_column_ids: Vec<u32> = (0..4).map(|i| i as u32).collect();

        for line in menu {
            let line_array: [&glib::ToValue; 4] = [&line.word, &line.kind, &line.menu, &line.info];
            list_store.insert_with_values(None, &all_column_ids, &line_array[..]);
        }

        self.tree.set_model(Some(&list_store));
    }

    fn update_css(&self, color_model: &ColorModel) {
        let bg = color_model.pmenu_bg_sel();
        let fg = color_model.pmenu_fg_sel();

        match gtk::CssProviderExt::load_from_data(
            &self.css_provider,
            &format!(
                ".view {{ color: {}; background-color: {};}}",
                fg.to_hex(),
                bg.to_hex()
            ).as_bytes(),
        ) {
            Err(e) => error!("Can't update css {}", e),
            Ok(_) => (),
        };
    }

    fn select(&self, selected: i64) {
        if selected >= 0 {
            let selected_path = gtk::TreePath::new_from_string(&format!("{}", selected));
            self.tree.get_selection().select_path(&selected_path);
            self.tree.scroll_to_cell(
                Some(&selected_path),
                None,
                false,
                0.0,
                0.0,
            );

            self.show_info_column(&selected_path);

        } else {
            self.tree.get_selection().unselect_all();
            self.info_label.hide();
        }
    }

    fn show_info_column(&self, selected_path: &gtk::TreePath) {
        let model = self.tree.get_model().unwrap();
        let iter = model.get_iter(selected_path);

        if let Some(iter) = iter {
            let info_value = model.get_value(&iter, 3);
            let info: &str = info_value.get().unwrap();

            if !info.trim().is_empty() {
                self.info_label.show();
                self.info_label.set_text(&info);
            } else {
                self.info_label.hide();
            }
        } else {
            self.info_label.hide();
        }
    }

    fn calc_treeview_height(&self) -> i32 {
        let (_, natural_size) = self.renderer.get_preferred_height(&self.tree);
        let (_, ypad) = self.renderer.get_padding();

        let row_height = natural_size + ypad;

        let actual_count = self.tree.get_model().unwrap().iter_n_children(None);

        row_height * min(actual_count, MAX_VISIBLE_ROWS) as i32
    }
}

pub struct PopupMenu {
    popover: gtk::Popover,
    open: bool,

    state: Rc<RefCell<State>>,
}

impl PopupMenu {
    pub fn new(drawing: &gtk::DrawingArea) -> PopupMenu {
        let state = State::new();
        let popover = gtk::Popover::new(Some(drawing));
        popover.set_modal(false);

        let content = gtk::Box::new(gtk::Orientation::Vertical, 0);

        state.tree.set_headers_visible(false);
        state.tree.set_can_focus(false);


        state.scroll.set_policy(
            gtk::PolicyType::Automatic,
            gtk::PolicyType::Automatic,
        );

        state.scroll.add(&state.tree);
        state.scroll.show_all();

        content.pack_start(&state.scroll, true, true, 0);
        content.pack_start(&state.info_label, false, true, 0);
        content.show();
        popover.add(&content);

        let state = Rc::new(RefCell::new(state));
        let state_ref = state.clone();
        state.borrow().tree.connect_button_press_event(
            move |tree, ev| {
                let state = state_ref.borrow();
                let nvim = state.nvim.as_ref().unwrap().nvim();
                if let Some(mut nvim) = nvim {
                    tree_button_press(tree, ev, &mut *nvim)
                } else {
                    Inhibit(false)
                }
            },
        );

        let state_ref = state.clone();
        state.borrow().tree.connect_size_allocate(move |_, _| {
            on_treeview_allocate(state_ref.clone())
        });

        let state_ref = state.clone();
        popover.connect_key_press_event(move |_, ev| {
            let state = state_ref.borrow();
            let nvim = state.nvim.as_ref().unwrap().nvim();
            if let Some(mut nvim) = nvim {
                input::gtk_key_press(&mut *nvim, ev)
            } else {
                Inhibit(false)
            }
        });

        PopupMenu {
            popover,
            state,
            open: false,
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn show(
        &mut self,
        shell: &shell::State,
        menu_items: &[CompleteItem],
        selected: i64,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) {

        self.open = true;

        self.popover.set_pointing_to(&gtk::Rectangle {
            x,
            y,
            width,
            height,
        });
        self.state.borrow_mut().before_show(
            shell,
            menu_items,
            selected,
        );
        self.popover.popup()
    }

    pub fn hide(&mut self) {
        self.open = false;
        // popdown() in case of fast hide/show
        // situation does not work and just close popup window
        // so hide() is important here
        self.popover.hide();
    }

    pub fn select(&self, selected: i64) {
        self.state.borrow().select(selected);
    }
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

        nvim.input(&apply_command).report_err();
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


fn on_treeview_allocate(state: Rc<RefCell<State>>) {
    let treeview_height = state.borrow().calc_treeview_height();

    idle_add(move || {
        let state = state.borrow();

        // strange solution to make gtk assertions happy
        let previous_height = state.scroll.get_max_content_height();
        if previous_height < treeview_height {
            state.scroll.set_max_content_height(treeview_height);
            state.scroll.set_min_content_height(treeview_height);
        } else if previous_height > treeview_height {
            state.scroll.set_min_content_height(treeview_height);
            state.scroll.set_max_content_height(treeview_height);
        }
        Continue(false)
    });
}
