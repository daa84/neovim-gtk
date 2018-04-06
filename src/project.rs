use std::rc::Rc;
use std::cell::RefCell;
use std::path::Path;

use pango;
use gtk;
use gtk::prelude::*;
use gtk::{TreeView, ScrolledWindow, PolicyType, ListStore, TreeViewColumn, CellRendererText,
          CellRendererPixbuf, CellRendererToggle, Type, Orientation, TreeModel, TreeIter, Popover};

use neovim_lib::{Neovim, NeovimApi, Value};
use nvim::ErrorReport;
use shell::Shell;

use htmlescape::encode_minimal;

const MAX_VISIBLE_ROWS: usize = 5;

const BOOKMARKED_PIXBUF: &str = "user-bookmarks";
const CURRENT_DIR_PIXBUF: &str = "folder";
const PLAIN_FILE_PIXBUF: &str = "text-x-generic";

enum ProjectViewColumns {
    Name,
    Path,
    Uri,
    Pixbuf,
    Project,
    ProjectStored,
}

const COLUMN_COUNT: usize = 6;
const COLUMN_TYPES: [Type; COLUMN_COUNT] = [
    Type::String,
    Type::String,
    Type::String,
    Type::String,
    Type::Bool,
    Type::Bool,
];
const COLUMN_IDS: [u32; COLUMN_COUNT] = [
    ProjectViewColumns::Name as u32,
    ProjectViewColumns::Path as u32,
    ProjectViewColumns::Uri as u32,
    ProjectViewColumns::Pixbuf as u32,
    ProjectViewColumns::Project as u32,
    ProjectViewColumns::ProjectStored as u32,
];

pub struct Projects {
    shell: Rc<RefCell<Shell>>,
    popup: Popover,
    tree: TreeView,
    scroll: ScrolledWindow,
    store: Option<EntryStore>,
    name_renderer: CellRendererText,
    path_renderer: CellRendererText,
    toggle_renderer: CellRendererToggle,
}

impl Projects {
    pub fn new(ref_widget: &gtk::Button, shell: Rc<RefCell<Shell>>) -> Rc<RefCell<Projects>> {
        let projects = Projects {
            shell,
            popup: Popover::new(Some(ref_widget)),
            tree: TreeView::new(),
            scroll: ScrolledWindow::new(None, None),
            store: None,
            name_renderer: CellRendererText::new(),
            path_renderer: CellRendererText::new(),
            toggle_renderer: CellRendererToggle::new(),
        };

        projects.setup_tree();

        projects.tree.set_activate_on_single_click(true);
        projects.tree.set_hover_selection(true);
        projects.tree.set_grid_lines(gtk::TreeViewGridLines::Horizontal);

        let vbox = gtk::Box::new(Orientation::Vertical, 5);
        vbox.set_border_width(5);

        let search_box = gtk::Entry::new();
        search_box.set_icon_from_icon_name(gtk::EntryIconPosition::Primary, "edit-find-symbolic");

        vbox.pack_start(&search_box, false, true, 0);


        projects.scroll.set_policy(
            PolicyType::Never,
            PolicyType::Automatic,
        );

        projects.scroll.add(&projects.tree);
        projects.scroll.set_shadow_type(gtk::ShadowType::In);

        vbox.pack_start(&projects.scroll, true, true, 0);

        let open_btn = gtk::Button::new_with_label("Other Documents…");
        vbox.pack_start(&open_btn, true, true, 5);

        vbox.show_all();
        projects.popup.add(&vbox);


        let projects = Rc::new(RefCell::new(projects));

        let prj_ref = projects.clone();
        projects.borrow().tree.connect_size_allocate(move |_, _| {
            on_treeview_allocate(prj_ref.clone())
        });

        let prj_ref = projects.clone();
        search_box.connect_changed(move |search_box| {
            let projects = prj_ref.borrow();
            let list_store = projects.get_list_store();

            list_store.clear();
            if let Some(ref store) = projects.store {
                store.populate(&list_store, search_box.get_text().as_ref());
            }
        });

        let prj_ref = projects.clone();
        search_box.connect_activate(move |_| {
            let model = prj_ref.borrow().tree.get_model().unwrap();
            if let Some(iter) = model.get_iter_first() {
                prj_ref.borrow().open_uri(&model, &iter);
                let popup = prj_ref.borrow().popup.clone();
                popup.popdown();
            }
        });

        let prj_ref = projects.clone();
        projects.borrow().tree.connect_row_activated(
            move |tree, _, column| {
                // Don't activate if the user clicked the checkbox.
                let toggle_column = tree.get_column(2).unwrap();
                if *column == toggle_column {
                    return;
                }
                let selection = tree.get_selection();
                if let Some((model, iter)) = selection.get_selected() {
                    prj_ref.borrow().open_uri(&model, &iter);
                    let popup = prj_ref.borrow().popup.clone();
                    popup.popdown();
                }
            },
        );

        let prj_ref = projects.clone();
        open_btn.connect_clicked(move |_| {
            prj_ref.borrow().show_open_file_dlg();
            let popup = prj_ref.borrow().popup.clone();
            popup.popdown();
        });

        let prj_ref = projects.clone();
        projects.borrow().popup.connect_closed(
            move |_| prj_ref.borrow_mut().clear(),
        );

        let prj_ref = projects.clone();
        projects.borrow().toggle_renderer.connect_toggled(
            move |_, path| {
                prj_ref.borrow_mut().toggle_stored(&path)
            },
        );
        projects
    }

    fn toggle_stored(&mut self, path: &gtk::TreePath) {
        let list_store = self.get_list_store();
        if let Some(iter) = list_store.get_iter(path) {
            let value: bool = list_store
                .get_value(&iter, ProjectViewColumns::ProjectStored as i32)
                .get()
                .unwrap();

            list_store.set_value(
                &iter,
                ProjectViewColumns::ProjectStored as u32,
                &ToValue::to_value(&!value),
            );

            let pixbuf = if value {
                CURRENT_DIR_PIXBUF
            } else {
                BOOKMARKED_PIXBUF
            };

            list_store.set_value(
                &iter,
                ProjectViewColumns::Pixbuf as u32,
                &ToValue::to_value(pixbuf),
            );

            let uri_value = list_store.get_value(&iter, ProjectViewColumns::Uri as i32);
            let uri: String = uri_value.get().unwrap();

            let store = self.store.as_mut().unwrap();
            if let Some(entry) = store.find_mut(&uri) {
                entry.stored = !value;
            }

            store.changed();
        }

    }


    fn open_uri(&self, model: &TreeModel, iter: &TreeIter) {
        let uri: String = model
            .get_value(iter, ProjectViewColumns::Uri as i32)
            .get()
            .unwrap();
        let project: bool = model
            .get_value(iter, ProjectViewColumns::Project as i32)
            .get()
            .unwrap();

        let shell = self.shell.borrow();
        if project {
            shell.cd(&uri);
        }
        shell.open_file(&uri);
    }

    fn get_list_store(&self) -> ListStore {
        self.tree
            .get_model()
            .unwrap()
            .downcast::<ListStore>()
            .unwrap()
    }

    fn show_open_file_dlg(&self) {
        let window = self.popup
            .get_toplevel()
            .unwrap()
            .downcast::<gtk::Window>()
            .ok();
        let dlg = gtk::FileChooserDialog::new(
            Some("Open Document"),
            window.as_ref(),
            gtk::FileChooserAction::Open,
        );

        const OPEN_ID: i32 = 0;
        const CANCEL_ID: i32 = 1;

        dlg.add_buttons(&[("_Open", OPEN_ID), ("_Cancel", CANCEL_ID)]);
        if dlg.run() == OPEN_ID {
            if let Some(filename) = dlg.get_filename() {
                if let Some(filename) = filename.to_str() {
                    self.shell.borrow().open_file(filename);
                }
            }
        }
        dlg.destroy();
    }

    pub fn show(&mut self) {
        self.load_oldfiles();

        self.popup.popup();
    }

    fn load_oldfiles(&mut self) {
        let shell_borrow = self.shell.borrow();
        let shell_state = shell_borrow.state.borrow_mut();

        let nvim = shell_state.nvim();
        if let Some(mut nvim) = nvim {
            let store = EntryStore::load(&mut nvim);
            store.populate(&self.get_list_store(), None);
            self.store = Some(store);
        }
    }

    pub fn clear(&mut self) {
        self.store.take().map(|s| s.save());
        self.get_list_store().clear();
    }

    fn setup_tree(&self) {
        self.tree.set_model(Some(&ListStore::new(&COLUMN_TYPES)));
        self.tree.set_headers_visible(false);

        let image_column = TreeViewColumn::new();

        let icon_renderer = CellRendererPixbuf::new();
        icon_renderer.set_padding(5, 0);
        image_column.pack_start(&icon_renderer, true);

        image_column.add_attribute(
            &icon_renderer,
            "icon-name",
            ProjectViewColumns::Pixbuf as i32,
        );

        self.tree.append_column(&image_column);

        let text_column = TreeViewColumn::new();

        self.name_renderer.set_property_width_chars(45);
        self.path_renderer.set_property_width_chars(45);
        self.name_renderer.set_property_ellipsize(
            pango::EllipsizeMode::Middle,
        );
        self.path_renderer.set_property_ellipsize(
            pango::EllipsizeMode::Start,
        );
        self.name_renderer.set_padding(0, 5);
        self.path_renderer.set_padding(0, 5);

        text_column.pack_start(&self.name_renderer, true);
        text_column.pack_start(&self.path_renderer, true);

        text_column.add_attribute(
            &self.name_renderer,
            "text",
            ProjectViewColumns::Name as i32,
        );
        text_column.add_attribute(
            &self.path_renderer,
            "markup",
            ProjectViewColumns::Path as i32,
        );

        let area = text_column
            .get_area()
            .unwrap()
            .downcast::<gtk::CellAreaBox>()
            .expect("Error build tree view");
        area.set_orientation(gtk::Orientation::Vertical);

        self.tree.append_column(&text_column);


        let toggle_column = TreeViewColumn::new();
        self.toggle_renderer.set_activatable(true);
        self.toggle_renderer.set_padding(10, 0);

        toggle_column.pack_start(&self.toggle_renderer, true);
        toggle_column.add_attribute(
            &self.toggle_renderer,
            "visible",
            ProjectViewColumns::Project as i32,
        );
        toggle_column.add_attribute(
            &self.toggle_renderer,
            "active",
            ProjectViewColumns::ProjectStored as i32,
        );

        self.tree.append_column(&toggle_column);
    }


    fn calc_treeview_height(&self) -> i32 {
        let (_, name_renderer_natural_size) = self.name_renderer.get_preferred_height(&self.tree);
        let (_, path_renderer_natural_size) = self.path_renderer.get_preferred_height(&self.tree);
        let (_, ypad) = self.name_renderer.get_padding();

        let row_height = name_renderer_natural_size + path_renderer_natural_size + ypad;

        row_height * MAX_VISIBLE_ROWS as i32
    }
}


fn on_treeview_allocate(projects: Rc<RefCell<Projects>>) {
    let treeview_height = projects.borrow().calc_treeview_height();

    idle_add(move || {
        let prj = projects.borrow();

        // strange solution to make gtk assertions happy
        let previous_height = prj.scroll.get_max_content_height();
        if previous_height < treeview_height {
            prj.scroll.set_max_content_height(treeview_height);
            prj.scroll.set_min_content_height(treeview_height);
        } else if previous_height > treeview_height {
            prj.scroll.set_min_content_height(treeview_height);
            prj.scroll.set_max_content_height(treeview_height);
        }
        Continue(false)
    });
}


fn list_old_files(nvim: &mut Neovim) -> Vec<String> {

    let oldfiles_var = nvim.get_vvar("oldfiles");

    match oldfiles_var {
        Ok(files) => {
            if let Some(files) = files.as_array() {
                files
                    .iter()
                    .map(Value::as_str)
                    .filter(Option::is_some)
                    .map(|path| path.unwrap().to_owned())
                    .filter(|path| !path.starts_with("term:"))
                    .collect()
            } else {
                vec![]
            }
        }
        err @ Err(_) => {
            err.report_err();
            vec![]
        }
    }
}

pub struct EntryStore {
    entries: Vec<Entry>,
    changed: bool,
}

impl EntryStore {
    pub fn find_mut(&mut self, uri: &str) -> Option<&mut Entry> {
        self.entries.iter_mut().find(|e| e.project && e.uri == uri)
    }

    pub fn load(nvim: &mut Neovim) -> EntryStore {
        let mut entries = Vec::new();

        for project in ProjectSettings::load().projects {
            entries.push(project.to_entry());
        }

        match nvim.call_function("getcwd", vec![]) {
            Ok(pwd) => {
                if let Some(pwd) = pwd.as_str() {
                    if entries.iter().find(|e| e.project && e.uri == pwd).is_none() {
                        entries.insert(0, Entry::new_current_project(pwd));
                    }
                } else {
                    error!("Error get current directory");
                }
            }
            err @ Err(_) => err.report_err(),
        }

        let old_files = list_old_files(nvim);
        entries.extend(old_files.iter().map(|p| Entry::new_from_path(p)));

        EntryStore {
            entries,
            changed: false,
        }
    }

    pub fn save(&self) {
        if self.changed {
            ProjectSettings::new(
                self.entries
                    .iter()
                    .filter(|e| e.project && e.stored)
                    .map(|p| p.to_entry_settings())
                    .collect(),
            ).save();
        }
    }

    pub fn populate(&self, list_store: &ListStore, filter: Option<&String>) {
        for file in &self.entries {
            if match filter.map(|f| f.to_uppercase()) {
                Some(ref filter) => {
                    file.file_name.to_uppercase().contains(filter) ||
                        file.path.to_uppercase().contains(filter)
                }
                None => true,
            }
            {
                list_store.insert_with_values(None, &COLUMN_IDS, &file.to_values());
            }
        }
    }

    fn changed(&mut self) {
        self.changed = true;
    }
}

pub struct Entry {
    uri: String,
    path: String,
    file_name: String,
    name: String,
    pixbuf: &'static str,
    project: bool,
    stored: bool,
}

impl Entry {
    fn new_project(name: &str, uri: &str) -> Entry {
        let path = Path::new(uri);

        Entry {
            uri: uri.to_owned(),
            path: path.parent()
                .map(|s| {
                    format!("<small>{}</small>", encode_minimal(&s.to_string_lossy()))
                })
                .unwrap_or_else(|| "".to_owned()),
            file_name: encode_minimal(name),
            name: name.to_owned(),
            pixbuf: BOOKMARKED_PIXBUF,
            project: true,
            stored: true,
        }
    }

    fn new_current_project(uri: &str) -> Entry {
        let path = Path::new(uri);
        let name = path.file_name()
            .map(|f| f.to_string_lossy().as_ref().to_owned())
            .unwrap_or_else(|| path.to_string_lossy().as_ref().to_owned());

        Entry {
            uri: uri.to_owned(),
            path: path.parent()
                .map(|s| {
                    format!("<small>{}</small>", encode_minimal(&s.to_string_lossy()))
                })
                .unwrap_or_else(|| "".to_owned()),
            file_name: encode_minimal(&name),
            name,
            pixbuf: CURRENT_DIR_PIXBUF,
            project: true,
            stored: false,
        }
    }

    fn new_from_path(uri: &str) -> Entry {
        let path = Path::new(uri);
        let name = path.file_name()
            .map(|f| f.to_string_lossy().as_ref().to_owned())
            .unwrap_or_else(|| "<empty>".to_owned());

        Entry {
            uri: uri.to_owned(),
            path: path.parent()
                .map(|s| {
                    format!("<small>{}</small>", encode_minimal(&s.to_string_lossy()))
                })
                .unwrap_or_else(|| "".to_owned()),
            file_name: encode_minimal(&name),
            name,
            pixbuf: PLAIN_FILE_PIXBUF,
            project: false,
            stored: false,
        }
    }

    fn to_values(&self) -> Box<[&gtk::ToValue]> {
        Box::new(
            [
                &self.file_name,
                &self.path,
                &self.uri,
                &self.pixbuf,
                &self.project,
                &self.stored,
            ],
        )
    }

    fn to_entry_settings(&self) -> ProjectEntrySettings {
        ProjectEntrySettings::new(&self.name, &self.uri)
    }
}

// ----- Store / Load settings
//
use settings::SettingsLoader;
use toml;

#[derive(Serialize, Deserialize)]
struct ProjectSettings {
    projects: Vec<ProjectEntrySettings>,
}

#[derive(Serialize, Deserialize)]
struct ProjectEntrySettings {
    name: String,
    path: String,
}

impl ProjectEntrySettings {
    fn new(name: &str, path: &str) -> ProjectEntrySettings {
        ProjectEntrySettings {
            name: name.to_owned(),
            path: path.to_owned(),
        }
    }

    fn to_entry(&self) -> Entry {
        Entry::new_project(&self.name, &self.path)
    }
}

impl SettingsLoader for ProjectSettings {
    const SETTINGS_FILE: &'static str = "projects.toml";

    fn empty() -> ProjectSettings {
        ProjectSettings { projects: vec![] }
    }

    fn from_str(s: &str) -> Result<Self, String> {
        toml::from_str(&s).map_err(|e| format!("{}", e))
    }
}

impl ProjectSettings {
    fn new(projects: Vec<ProjectEntrySettings>) -> ProjectSettings {
        ProjectSettings { projects }
    }
}
