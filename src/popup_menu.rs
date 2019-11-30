use std::cell::RefCell;
use std::cmp::min;
use std::iter;
use std::rc::Rc;

use gdk::{EventButton, EventType};
use glib;
use gtk;
use gtk::prelude::*;
use pango;

use neovim_lib::{Neovim, NeovimApi};

use crate::highlight::HighlightMap;
use crate::input;
use crate::nvim::{self, ErrorReport, NeovimClient};
use crate::render;

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
    preview: bool,
}

impl State {
    pub fn new() -> Self {
        let tree = gtk::TreeView::new();
        tree.get_selection().set_mode(gtk::SelectionMode::Single);
        let css_provider = gtk::CssProvider::new();

        let style_context = tree.get_style_context();
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

        let scroll = gtk::ScrolledWindow::new(
            Option::<&gtk::Adjustment>::None,
            Option::<&gtk::Adjustment>::None,
        );

        tree.connect_size_allocate(
            clone!(scroll, renderer => move |tree, _| on_treeview_allocate(&scroll, tree, &renderer)),
        );

        State {
            nvim: None,
            tree,
            renderer,
            scroll,
            css_provider,
            info_label,
            word_column,
            kind_column,
            menu_column,
            preview: true,
        }
    }

    fn before_show(&mut self, ctx: PopupMenuContext) {
        if self.nvim.is_none() {
            self.nvim = Some(ctx.nvim.clone());
        }

        self.scroll.set_max_content_width(ctx.max_width);
        self.scroll.set_propagate_natural_width(true);
        self.scroll.set_propagate_natural_height(true);
        self.update_tree(&ctx);
        self.select(ctx.selected);
    }

    fn limit_column_widths(&self, ctx: &PopupMenuContext) {
        const DEFAULT_PADDING: i32 = 5;

        let layout = ctx.font_ctx.create_layout();
        let kind_exists = ctx.menu_items.iter().any(|i| !i.kind.is_empty());
        let max_width = self.scroll.get_max_content_width();
        let (xpad, _) = self.renderer.get_padding();

        let max_word_line = ctx.menu_items.iter().max_by_key(|m| m.word.len()).unwrap();
        layout.set_text(max_word_line.word);
        let (word_max_width, _) = layout.get_pixel_size();
        let word_column_width = word_max_width + xpad * 2 + DEFAULT_PADDING;

        if kind_exists {
            layout.set_text("[v]");
            let (kind_width, _) = layout.get_pixel_size();

            self.kind_column
                .set_fixed_width(kind_width + xpad * 2 + DEFAULT_PADDING);
            self.kind_column.set_visible(true);

            self.word_column
                .set_fixed_width(min(max_width - kind_width, word_column_width));
        } else {
            self.kind_column.set_visible(false);
            self.word_column
                .set_fixed_width(min(max_width, word_column_width));
        }

        let max_menu_line = ctx.menu_items.iter().max_by_key(|m| m.menu.len()).unwrap();

        if !max_menu_line.menu.is_empty() {
            layout.set_text(max_menu_line.menu);
            let (menu_max_width, _) = layout.get_pixel_size();
            self.menu_column
                .set_fixed_width(menu_max_width + xpad * 2 + DEFAULT_PADDING);
            self.menu_column.set_visible(true);
        } else {
            self.menu_column.set_visible(false);
        }
    }

    fn update_tree(&self, ctx: &PopupMenuContext) {
        if ctx.menu_items.is_empty() {
            return;
        }

        self.limit_column_widths(ctx);

        self.renderer
            .set_property_font(Some(ctx.font_ctx.font_description().to_string().as_str()));

        let hl = &ctx.hl;
        self.renderer
            .set_property_foreground_rgba(Some(&hl.pmenu_fg().into()));

        update_css(&self.css_provider, hl);

        let list_store = gtk::ListStore::new(&[gtk::Type::String; 4]);
        let all_column_ids: Vec<u32> = (0..4).map(|i| i as u32).collect();

        for line in ctx.menu_items {
            let line_array: [&dyn glib::ToValue; 4] = [&line.word, &line.kind, &line.menu, &line.info];
            list_store.insert_with_values(None, &all_column_ids, &line_array[..]);
        }

        self.tree.set_model(Some(&list_store));
    }

    fn select(&self, selected: i64) {
        if selected >= 0 {
            let selected_path = gtk::TreePath::new_from_string(&format!("{}", selected));
            self.tree.get_selection().select_path(&selected_path);
            self.tree.scroll_to_cell(
                Some(&selected_path),
                Option::<&gtk::TreeViewColumn>::None,
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

            if self.preview && !info.trim().is_empty() {
                self.info_label.show();
                self.info_label.set_text(&info);
            } else {
                self.info_label.hide();
            }
        } else {
            self.info_label.hide();
        }
    }

    fn set_preview(&mut self, preview: bool) {
        self.preview = preview;
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

        state
            .scroll
            .set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);

        state.scroll.add(&state.tree);
        state.scroll.show_all();

        content.pack_start(&state.scroll, true, true, 0);
        content.pack_start(&state.info_label, false, true, 0);
        content.show();
        popover.add(&content);

        let state = Rc::new(RefCell::new(state));
        let state_ref = state.clone();
        state
            .borrow()
            .tree
            .connect_button_press_event(move |tree, ev| {
                let state = state_ref.borrow();
                let nvim = state.nvim.as_ref().unwrap().nvim();
                if let Some(mut nvim) = nvim {
                    tree_button_press(tree, ev, &mut *nvim, "<C-y>");
                }
                Inhibit(false)
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

    pub fn show(&mut self, ctx: PopupMenuContext) {
        self.open = true;

        self.popover.set_pointing_to(&gtk::Rectangle {
            x: ctx.x,
            y: ctx.y,
            width: ctx.width,
            height: ctx.height,
        });
        self.state.borrow_mut().before_show(ctx);
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

    pub fn set_preview(&self, preview: bool) {
        self.state.borrow_mut().set_preview(preview);
    }
}

pub struct PopupMenuContext<'a> {
    pub nvim: &'a Rc<NeovimClient>,
    pub hl: &'a HighlightMap,
    pub font_ctx: &'a render::Context,
    pub menu_items: &'a [nvim::CompleteItem<'a>],
    pub selected: i64,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub max_width: i32,
}

pub fn tree_button_press(
    tree: &gtk::TreeView,
    ev: &EventButton,
    nvim: &mut Neovim,
    last_command: &str,
) {
    if ev.get_event_type() != EventType::ButtonPress {
        return;
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

        let apply_command: String = if target_idx > selected_idx {
            (0..scroll_count)
                .map(|_| "<C-n>")
                .chain(iter::once(last_command))
                .collect()
        } else {
            (0..scroll_count)
                .map(|_| "<C-p>")
                .chain(iter::once(last_command))
                .collect()
        };

        nvim.input(&apply_command).report_err();
    }
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

fn on_treeview_allocate(
    scroll: &gtk::ScrolledWindow,
    tree: &gtk::TreeView,
    renderer: &gtk::CellRendererText,
) {
    let treeview_height = calc_treeview_height(tree, renderer);

    idle_add(clone!(scroll => move || {
            scroll
            .set_max_content_height(treeview_height);
        Continue(false)
    }));
}

pub fn update_css(css_provider: &gtk::CssProvider, hl: &HighlightMap) {
    let bg = hl.pmenu_bg_sel();
    let fg = hl.pmenu_fg_sel();

    if let Err(e) = gtk::CssProviderExt::load_from_data(
        css_provider,
        &format!(
            ".view :selected {{ color: {}; background-color: {};}}\n
                .view {{ background-color: {}; }}",
            fg.to_hex(),
            bg.to_hex(),
            hl.pmenu_bg().to_hex(),
        )
        .as_bytes(),
    ) {
        error!("Can't update css {}", e)
    };
}

pub fn calc_treeview_height(tree: &gtk::TreeView, renderer: &gtk::CellRendererText) -> i32 {
    let (_, natural_size) = renderer.get_preferred_height(tree);
    let (_, ypad) = renderer.get_padding();

    let row_height = natural_size + ypad;

    let actual_count = tree.get_model().unwrap().iter_n_children(None);

    row_height * min(actual_count, MAX_VISIBLE_ROWS) as i32
}
