use std::cell::RefCell;
use std::cmp::Ordering;
use std::io;
use std::fs;
use std::fs::DirEntry;
use std::path::{Path, Component};
use std::rc::Rc;
use std::ops::Deref;

use gio;
use gio::prelude::*;
use gtk;
use gtk::{MenuExt};
use gtk::prelude::*;

use neovim_lib::NeovimApi;

use nvim::{NeovimClient, ErrorReport, NeovimRef};
use shell;

struct Components {
    go_up_btn: gtk::Button,
    cwd_label: gtk::Label,
    context_menu: gtk::Menu,
    show_hidden_checkbox: gtk::CheckMenuItem,
}

struct State {
    current_dir: String,
    show_hidden: bool,
}


pub struct FileBrowserWidget {
    store: gtk::TreeStore,
    tree: gtk::TreeView,
    widget: gtk::Box,
    nvim: Option<Rc<NeovimClient>>,
    comps: Components,
    state: Rc<RefCell<State>>,
}

impl Deref for FileBrowserWidget {
    type Target = gtk::Box;

    fn deref(&self) -> &gtk::Box {
        &self.widget
    }
}

#[derive(Copy, Clone, Debug)]
enum FileType {
    File,
    Dir,
}

#[allow(dead_code)]
enum Column {
    Filename,
    Path,
    FileType,
    IconName,
}

impl FileBrowserWidget {
    pub fn new() -> Self {
        let builder = gtk::Builder::new_from_string(include_str!("../resources/side-panel.ui"));
        let widget: gtk::Box = builder.get_object("file_browser").unwrap();
        let tree: gtk::TreeView = builder.get_object("file_browser_tree_view").unwrap();
        let store: gtk::TreeStore = builder.get_object("file_browser_tree_store").unwrap();
        let go_up_btn: gtk::Button = builder.get_object("file_browser_go_up_button").unwrap();
        let cwd_label: gtk::Label = builder.get_object("file_browser_current_dir").unwrap();
        let context_menu: gtk::Menu = builder.get_object("file_browser_context_menu").unwrap();
        let show_hidden_checkbox: gtk::CheckMenuItem =
            builder.get_object("file_browser_show_hidden_checkbox").unwrap();

        let file_browser = FileBrowserWidget {
            store,
            tree,
            widget,
            nvim: None,
            comps: Components {
                go_up_btn,
                cwd_label,
                context_menu,
                show_hidden_checkbox,
            },
            state: Rc::new(RefCell::new(State {
                current_dir: "".to_owned(),
                show_hidden: false,
            })),
        };
        file_browser
    }

    fn nvim(&self) -> Option<NeovimRef> {
        self.nvim.as_ref().unwrap().nvim()
    }

    pub fn init(&mut self, shell_state: &shell::State) {
        // Initialize values.
        let nvim = shell_state.nvim_clone();
        self.nvim = Some(nvim);
        let dir = get_current_dir(&mut self.nvim().unwrap());
        update_toolbar(&dir, &self.comps.go_up_btn, &self.comps.cwd_label);
        self.state.borrow_mut().current_dir = dir;

        // Populate tree.
        tree_reload(&self.store, &self.state.borrow());

        // We cannot recursively populate all directories. Instead, we have prepared a single empty
        // child entry for all non-empty directories, so the little expand arrow will be shown. Now,
        // when a directory is expanded, populate its children.
        let store = self.store.clone();
        let state_ref = Rc::clone(&self.state);
        self.tree.connect_test_expand_row(move |_, iter, _| {
            let state = state_ref.borrow();
            if let Some(child) = store.iter_children(iter) {
                let filename = store.get_value(&child, Column::Filename as i32);
                if filename.get::<&str>().is_none() {
                    store.remove(&child);
                    let dir_value = store.get_value(&iter, Column::Path as i32);
                    if let Some(dir) = dir_value.get() {
                        populate_tree_nodes(&store, &state, dir, Some(iter));
                    }
                }
            }
            Inhibit(false)
        });

        // Further initialization.
        self.init_actions(&self.comps.context_menu);
        self.init_subscriptions(shell_state);
        self.connect_events();
    }

    fn init_actions(&self, menu: &gtk::Menu) {
        let actions = gio::SimpleActionGroup::new();

        let reload_action = gio::SimpleAction::new("reload", None);
        let store = self.store.clone();
        let state_ref = Rc::clone(&self.state);
        reload_action.connect_activate(move |_, _| {
            tree_reload(&store, &state_ref.borrow());
        });
        actions.add_action(&reload_action);

        menu.insert_action_group("filebrowser", &actions);
    }

    fn init_subscriptions(&self, shell_state: &shell::State) {
        // Always set the current working directory as the root of the file browser.
        let store = self.store.clone();
        let state_ref = Rc::clone(&self.state);
        let go_up_btn = self.comps.go_up_btn.clone();
        let cwd_label = self.comps.cwd_label.clone();
        shell_state.subscribe("DirChanged", &["getcwd()"], move |args| {
            let dir = args.into_iter().next().unwrap();
            let mut state = state_ref.borrow_mut();
            if dir != *state.current_dir {
                update_toolbar(&dir, &go_up_btn, &cwd_label);
                state.current_dir = dir;
                tree_reload(&store, &state);
            }
        });

        // Reveal the file of an entered buffer in the file browser and select the entry.
        let tree = self.tree.clone();
        let store = self.store.clone();
        let subscription = shell_state
            .subscribe("BufEnter", &["getcwd()", "expand('%:p')"], move |args: Vec<String>| {
                let mut args_iter = args.into_iter();
                let dir = args_iter.next().unwrap();
                let file_path = args_iter.next().unwrap();
                let could_reveal =
                    if let Ok(rel_path) = Path::new(&file_path).strip_prefix(&Path::new(&dir)) {
                        reveal_path_in_tree(&store, &tree, &rel_path)
                    } else {
                        false
                    };
                if !could_reveal{
                    tree.get_selection().unselect_all();
                }
            });
        shell_state.run_now(&subscription);
    }

    fn connect_events(&self) {
        // Open file / go to dir, when user clicks on an entry.
        let store = self.store.clone();
        let nvim_ref = Rc::clone(&self.nvim.as_ref().unwrap());
        self.tree.connect_row_activated(move |_, path, _| {
            let mut nvim = nvim_ref.nvim().unwrap();
            let iter = store.get_iter(path).unwrap();
            let file_type = store
                .get_value(&iter, Column::FileType as i32)
                .get::<u8>()
                .unwrap();
            let file_path = store
                .get_value(&iter, Column::Path as i32)
                .get::<String>()
                .unwrap();
            if file_type == FileType::Dir as u8 {
                nvim.set_current_dir(&file_path).report_err();
            } else { // FileType::File
                let dir = get_current_dir(&mut nvim);
                let dir = Path::new(&dir);
                let file_path = if let Some(rel_path) = Path::new(&file_path)
                    .strip_prefix(&dir)
                    .ok()
                    .and_then(|p| p.to_str())
                {
                    rel_path
                } else {
                    &file_path
                };
                nvim.command(&format!(":e {}", file_path)).report_err();
            }
        });

        // Connect go-up button.
        let state_ref = Rc::clone(&self.state);
        let nvim_ref = Rc::clone(&self.nvim.as_ref().unwrap());
        self.comps.go_up_btn.connect_clicked(move |_| {
            let dir = &state_ref.borrow().current_dir;
            let parent = Path::new(&dir).parent().unwrap().to_str().unwrap();
            let mut nvim = nvim_ref.nvim().unwrap();
            nvim.set_current_dir(parent).report_err();
        });

        // Open context menu on right click.
        let context_menu_ref = self.comps.context_menu.clone();
        self.tree.connect_button_press_event(move |_, ev_btn| {
            if ev_btn.get_button() == 3 {
                context_menu_ref.popup_at_pointer(&**ev_btn);
            }
            Inhibit(false)
        });

        // Show / hide hidden files when corresponding menu item is toggled.
        let state_ref = Rc::clone(&self.state);
        let store = self.store.clone();
        self.comps.show_hidden_checkbox.connect_toggled(move |ev| {
            let mut state = state_ref.borrow_mut();
            state.show_hidden = ev.get_active();
            tree_reload(&store, &state);
        });
    }
}

/// Compare function for dir entries.
///
/// Sorts directories above files.
fn cmp_dirs_first(lhs: &DirEntry, rhs: &DirEntry) -> io::Result<Ordering> {
    let lhs_metadata = lhs.metadata()?;
    let rhs_metadata = rhs.metadata()?;
    if lhs_metadata.file_type() == rhs_metadata.file_type() {
        Ok(lhs.path().cmp(&rhs.path()))
    } else {
        if lhs_metadata.is_dir() {
            Ok(Ordering::Less)
        } else {
            Ok(Ordering::Greater)
        }
    }
}

/// Clears an repopulate the entire tree.
fn tree_reload(store: &gtk::TreeStore, state: &State) {
    let dir = &state.current_dir;
    store.clear();
    populate_tree_nodes(store, state, dir, None);
}

/// Updates the tool bar on top of the file browser.
fn update_toolbar(dir: &str, go_up_btn: &gtk::Button, cwd_label: &gtk::Label) {
    let path = Path::new(dir).canonicalize().unwrap();
    // Disable go-up button if current directory is `/`.
    go_up_btn.set_sensitive(path.parent().is_some());
    // Display current directory name.
    let dir_name = path.components().last().unwrap().as_os_str();
    cwd_label.set_label(&*dir_name.to_string_lossy());
}

/// Populates one level, i.e. one directory of the file browser tree.
fn populate_tree_nodes(
    store: &gtk::TreeStore,
    state: &State,
    dir: &str,
    parent: Option<&gtk::TreeIter>
) {
    let path = Path::new(dir);
    let iter = path.read_dir()
        .expect("read dir failed")
        .filter_map(Result::ok);
    let mut entries: Vec<DirEntry> = if state.show_hidden {
        iter.collect()
    } else {
        iter.filter(|entry| !entry.file_name().to_string_lossy().starts_with("."))
            .collect()
    };
    entries.sort_unstable_by(|lhs, rhs| {
        cmp_dirs_first(lhs, rhs).unwrap_or(Ordering::Equal)
    });
    for entry in entries {
        let path = if let Some(path) = entry.path().to_str() {
            path.to_owned()
        } else {
            // Skip paths that contain invalid unicode.
            continue;
        };
        let filename = entry.file_name().to_str().unwrap().to_owned();
        let file_type = if let Ok(metadata) = fs::metadata(entry.path()) {
            let file_type = metadata.file_type();
            if file_type.is_dir() {
                FileType::Dir
            } else if file_type.is_file() {
                FileType::File
            } else {
                continue;
            }
        } else {
            // In case of invalid symlinks, we cannot obtain metadata.
            continue;
        };
        let icon = match file_type {
            FileType::Dir => "folder",
            FileType::File => "text-x-generic",
        };
        // When we get until here, we want to show the entry. Append it to the tree.
        let iter = store.append(parent);
        store.set(&iter, &[0, 1, 2, 3], &[&filename, &path, &(file_type as u8), &icon]);
        // For directories, check whether the directory is empty. If not, append a single empty
        // entry, so the expand arrow is shown. Its contents are dynamically populated when
        // expanded (see `init`).
        if let FileType::Dir = file_type {
            let not_empty = if let Ok(mut dir) = entry.path().read_dir() {
                dir.next().is_some()
            } else {
                false
            };
            if not_empty {
                let iter = store.append(&iter);
                store.set(&iter, &[], &[]);
            }
        }
    }
}

fn get_current_dir(nvim: &mut NeovimRef) -> String {
    nvim.eval("getcwd()")
        .as_ref()
        .ok()
        .and_then(|s| s.as_str())
        .expect("Couldn't get working directory")
        .to_owned()
}

/// Reveals and selects the given file in the file browser.
///
/// Returns `true` if the file could be successfully revealed.
fn reveal_path_in_tree(
    store: &gtk::TreeStore,
    tree: &gtk::TreeView,
    rel_file_path: &Path,
) -> bool {
    let mut tree_path = gtk::TreePath::new();
    'components: for component in rel_file_path.components() {
        if let Component::Normal(component) = component {
            tree_path.down();
            while let Some(iter) = store.get_iter(&tree_path) {
                let entry_value = store.get_value(&iter, Column::Filename as i32);
                let entry = entry_value.get::<&str>().unwrap();
                if component == entry {
                    tree.expand_row(&tree_path, false);
                    continue 'components;
                }
                tree_path.next();
            }
            return false;
        } else {
            return false;
        }
    }
    if tree_path.get_depth() == 0 {
        return false;
    }
    tree.set_cursor(&tree_path, None, false);
    true
}
