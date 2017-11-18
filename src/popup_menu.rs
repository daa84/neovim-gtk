use std::rc::Rc;
use std::cell::RefCell;
use std::cmp::min;

use gtk;
use gtk::prelude::*;
use glib;
use gdk::{EventButton, EventType};

use neovim_lib::{Neovim, NeovimApi};

use color::ColorModel;
use nvim::{self, ErrorReport, NeovimClient};
use input;
use render;

const MAX_VISIBLE_ROWS: i32 = 10;

struct State {
    nvim: Option<Rc<nvim::NeovimClient>>,
    renderer: gtk::CellRendererText,
    tree: gtk::TreeView,
    scroll: gtk::ScrolledWindow,
    css_provider: gtk::CssProvider,
}

impl State {
    pub fn new() -> Self {
        let tree = gtk::TreeView::new();
        let css_provider = gtk::CssProvider::new();

        let style_context = tree.get_style_context().unwrap();
        style_context.add_provider(&css_provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);

        State {
            nvim: None,
            renderer: gtk::CellRendererText::new(),
            tree,
            scroll: gtk::ScrolledWindow::new(None, None),
            css_provider,
        }
    }

    fn before_show(&mut self, ctx: PopupMenuContext) {
        if self.nvim.is_none() {
            self.nvim = Some(ctx.nvim.clone());
        }

        self.update_tree(&ctx);
        self.select(ctx.selected);
    }

    fn update_tree(&self, ctx: &PopupMenuContext) {
        if ctx.menu_items.is_empty() {
            return;
        }

        self.renderer.set_property_font(Some(
            &ctx.font_ctx.font_description().to_string(),
        ));

        let color_model = &ctx.color_model;
        self.renderer.set_property_foreground_rgba(
            Some(&color_model.pmenu_fg().into()),
        );
        self.renderer.set_property_background_rgba(
            Some(&color_model.pmenu_bg().into()),
        );

        self.update_css(color_model);

        let col_count = ctx.menu_items[0].len();
        let columns = self.tree.get_columns();

        if columns.len() != col_count {
            for col in columns {
                self.tree.remove_column(&col);
            }

            for i in 0..col_count {
                self.append_column(i as i32);
            }
        }

        let list_store = gtk::ListStore::new(&vec![gtk::Type::String; col_count]);
        let all_column_ids: Vec<u32> = (0..col_count).map(|i| i as u32).collect();

        for line in ctx.menu_items {
            let line_array: Vec<&glib::ToValue> =
                line.iter().map(|v| v as &glib::ToValue).collect();
            list_store.insert_with_values(None, &all_column_ids, &line_array[..]);
        }

        self.tree.set_model(Some(&list_store));
    }

    fn update_css(&self, color_model: &ColorModel) {
        let bg = color_model.pmenu_bg_sel();
        let fg = color_model.pmenu_fg_sel();

        match gtk::CssProviderExtManual::load_from_data(
            &self.css_provider,
            &format!(
                ".view {{ color: {}; background-color: {};}}",
                fg.to_hex(),
                bg.to_hex()
            ),
        ) {
            Err(e) => error!("Can't update css {}", e),
            Ok(_) => (),
        };
    }

    fn append_column(&self, id: i32) {
        let renderer = &self.renderer;

        let column = gtk::TreeViewColumn::new();
        column.pack_start(renderer, true);
        column.add_attribute(renderer, "text", id);
        self.tree.append_column(&column);
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
        } else {
            self.tree.get_selection().unselect_all();
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

        state.tree.set_headers_visible(false);
        state.tree.set_can_focus(false);


        state.scroll.set_policy(
            gtk::PolicyType::Never,
            gtk::PolicyType::Automatic,
        );

        state.scroll.add(&state.tree);
        state.scroll.show_all();
        popover.add(&state.scroll);

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
}

pub struct PopupMenuContext<'a> {
    pub nvim: &'a Rc<NeovimClient>,
    pub color_model: &'a ColorModel,
    pub font_ctx: &'a render::Context,
    pub menu_items: &'a [Vec<String>],
    pub selected: i64,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
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
