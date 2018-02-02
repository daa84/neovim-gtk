use std::cell::{Ref, RefCell, RefMut};
use std::{env, thread};
use std::rc::Rc;
use std::sync::Arc;

use gdk;
use gtk;
use gtk_sys;
use gtk::prelude::*;
use gtk::{AboutDialog, ApplicationWindow, HeaderBar, Image, SettingsExt, ToolButton};
use gio::prelude::*;
use gio::{Menu, MenuExt, MenuItem, SimpleAction};
use toml;

use settings::{Settings, SettingsLoader};
use shell::{self, Shell, ShellOptions};
use shell_dlg;
use project::Projects;
use plug_manager;

macro_rules! clone {
    (@param _) => ( _ );
    (@param $x:ident) => ( $x );
    ($($n:ident),+ => move || $body:expr) => (
        {
            $( let $n = $n.clone(); )+
                move || $body
        }
    );
    ($($n:ident),+ => move |$($p:tt),+| $body:expr) => (
        {
            $( let $n = $n.clone(); )+
                move |$(clone!(@param $p),)+| $body
        }
    );
}

pub struct Ui {
    initialized: bool,
    comps: Arc<UiMutex<Components>>,
    settings: Rc<RefCell<Settings>>,
    shell: Rc<RefCell<Shell>>,
    projects: Rc<RefCell<Projects>>,
    plug_manager: Arc<UiMutex<plug_manager::Manager>>,
}

pub struct Components {
    window: Option<ApplicationWindow>,
    window_state: WindowState,
    open_btn: ToolButton,
}

impl Components {
    fn new() -> Components {
        let save_image =
            Image::new_from_icon_name("document-open", gtk_sys::GTK_ICON_SIZE_SMALL_TOOLBAR as i32);

        Components {
            open_btn: ToolButton::new(Some(&save_image), "Open"),
            window: None,
            window_state: WindowState::load(),
        }
    }

    pub fn close_window(&self) {
        self.window.as_ref().unwrap().destroy();
    }

    pub fn window(&self) -> &ApplicationWindow {
        self.window.as_ref().unwrap()
    }
}

impl Ui {
    pub fn new(options: ShellOptions) -> Ui {
        let plug_manager = plug_manager::Manager::new();

        let plug_manager = Arc::new(UiMutex::new(plug_manager));
        let comps = Arc::new(UiMutex::new(Components::new()));
        let settings = Rc::new(RefCell::new(Settings::new()));
        let shell = Rc::new(RefCell::new(Shell::new(settings.clone(), options)));
        settings.borrow_mut().set_shell(Rc::downgrade(&shell));

        let projects = Projects::new(&comps.borrow().open_btn, shell.clone());

        Ui {
            initialized: false,
            comps,
            shell,
            settings,
            projects,
            plug_manager,
        }
    }

    pub fn init(&mut self, app: &gtk::Application) {
        if self.initialized {
            return;
        }
        self.initialized = true;

        let mut settings = self.settings.borrow_mut();
        settings.init();

        let window = ApplicationWindow::new(app);

        {
            // initialize window from comps
            // borrowing of comps must be leaved
            // for event processing
            let mut comps = self.comps.borrow_mut();

            self.shell.borrow_mut().init();

            comps.window = Some(window.clone());

            let prefer_dark_theme = env::var("NVIM_GTK_PREFER_DARK_THEME")
                .map(|opt| opt.trim() == "1")
                .unwrap_or(false);
            if prefer_dark_theme {
                if let Some(settings) = window.get_settings() {
                    settings.set_property_gtk_application_prefer_dark_theme(true);
                }
            }

            // Client side decorations including the toolbar are disabled via NVIM_GTK_NO_HEADERBAR=1
            let use_header_bar = env::var("NVIM_GTK_NO_HEADERBAR")
                .map(|opt| opt.trim() != "1")
                .unwrap_or(true);

            if app.prefers_app_menu() || use_header_bar {
                self.create_main_menu(app, &window);
            }

            if use_header_bar {
                let header_bar = HeaderBar::new();

                let projects = self.projects.clone();
                header_bar.pack_start(&comps.open_btn);
                comps
                    .open_btn
                    .connect_clicked(move |_| projects.borrow_mut().show());

                let save_image = Image::new_from_icon_name(
                    "document-save",
                    gtk_sys::GTK_ICON_SIZE_SMALL_TOOLBAR as i32,
                );
                let save_btn = ToolButton::new(Some(&save_image), "Save");

                let shell = self.shell.clone();
                save_btn.connect_clicked(move |_| shell.borrow_mut().edit_save_all());
                header_bar.pack_start(&save_btn);

                let paste_image = Image::new_from_icon_name(
                    "edit-paste",
                    gtk_sys::GTK_ICON_SIZE_SMALL_TOOLBAR as i32,
                );
                let paste_btn = ToolButton::new(Some(&paste_image), "Paste");
                let shell = self.shell.clone();
                paste_btn.connect_clicked(move |_| shell.borrow_mut().edit_paste());
                header_bar.pack_start(&paste_btn);

                header_bar.set_show_close_button(true);

                window.set_titlebar(Some(&header_bar));
            }

            window.set_default_size(
                comps.window_state.current_width,
                comps.window_state.current_height,
            );
            if comps.window_state.is_maximized {
                window.maximize();
            }
        }

        let comps_ref = self.comps.clone();
        window.connect_size_allocate(move |window, _| {
            gtk_window_size_allocate(window, &mut *comps_ref.borrow_mut())
        });

        let comps_ref = self.comps.clone();
        window.connect_window_state_event(move |_, event| {
            gtk_window_state_event(event, &mut *comps_ref.borrow_mut());
            Inhibit(false)
        });

        let comps_ref = self.comps.clone();
        window.connect_destroy(move |_| {
            comps_ref.borrow().window_state.save();
        });

        let shell = self.shell.borrow();
        window.add(&**shell);

        window.show_all();
        window.set_title("NeovimGtk");

        let comps_ref = self.comps.clone();
        let shell_ref = self.shell.clone();
        window.connect_delete_event(move |_, _| gtk_delete(&*comps_ref, &*shell_ref));

        shell.grab_focus();

        let comps_ref = self.comps.clone();
        shell.set_detach_cb(Some(move || {
            let comps_ref = comps_ref.clone();
            gtk::idle_add(move || {
                comps_ref.borrow().close_window();
                Continue(false)
            });
        }));

        let state_ref = self.shell.borrow().state.clone();
        let plug_manager_ref = self.plug_manager.clone();
        shell.set_nvim_started_cb(Some(move || {
            plug_manager_ref
                .borrow_mut()
                .init_nvim_client(state_ref.borrow().nvim_clone());
        }));
    }

    fn create_main_menu(&self, app: &gtk::Application, window: &gtk::ApplicationWindow) {
        let plug_manager = self.plug_manager.clone();

        let menu = Menu::new();

        let section = Menu::new();
        section.append_item(&MenuItem::new("New Window", "app.new-window"));
        menu.append_section(None, &section);

        let section = Menu::new();
        section.append_item(&MenuItem::new("Plugins", "app.Plugins"));
        section.append_item(&MenuItem::new("About", "app.HelpAbout"));
        menu.append_section(None, &section);

        menu.freeze();
        app.set_app_menu(Some(&menu));

        let plugs_action = SimpleAction::new("Plugins", None);
        plugs_action.connect_activate(
            clone!(window => move |_, _| plug_manager::Ui::new(&plug_manager).show(&window)),
        );

        let about_action = SimpleAction::new("HelpAbout", None);
        about_action.connect_activate(clone!(window => move |_, _| on_help_about(&window)));
        about_action.set_enabled(true);

        app.add_action(&about_action);
        app.add_action(&plugs_action);
    }
}

fn on_help_about(window: &gtk::ApplicationWindow) {
    let about = AboutDialog::new();
    about.set_transient_for(window);
    about.set_program_name("NeovimGtk");
    about.set_version(env!("CARGO_PKG_VERSION"));
    about.set_logo_icon_name("org.daa.NeovimGtk");
    about.set_authors(&[env!("CARGO_PKG_AUTHORS")]);
    about.set_comments(
        format!(
            "Build on top of neovim\n\
             Minimum supported neovim version: {}",
            shell::MINIMUM_SUPPORTED_NVIM_VERSION
        ).as_str(),
    );

    about.connect_response(|about, _| about.destroy());
    about.show();
}

fn gtk_delete(comps: &UiMutex<Components>, shell: &RefCell<Shell>) -> Inhibit {
    if !shell.borrow().is_nvim_initialized() {
        return Inhibit(false);
    }

    Inhibit(if shell_dlg::can_close_window(comps, shell) {
        let comps = comps.borrow();
        comps.close_window();
        shell.borrow_mut().detach_ui();
        false
    } else {
        true
    })
}

fn gtk_window_size_allocate(app_window: &gtk::ApplicationWindow, comps: &mut Components) {
    if !app_window.is_maximized() {
        let (current_width, current_height) = app_window.get_size();
        comps.window_state.current_width = current_width;
        comps.window_state.current_height = current_height;
    }
}

fn gtk_window_state_event(event: &gdk::EventWindowState, comps: &mut Components) {
    comps.window_state.is_maximized = event
        .get_new_window_state()
        .contains(gdk::WindowState::MAXIMIZED);
}

#[derive(Serialize, Deserialize)]
struct WindowState {
    current_width: i32,
    current_height: i32,
    is_maximized: bool,
}

impl WindowState {
    pub fn new() -> Self {
        WindowState {
            current_width: 800,
            current_height: 600,
            is_maximized: false,
        }
    }
}

impl SettingsLoader for WindowState {
    const SETTINGS_FILE: &'static str = "window.toml";

    fn empty() -> WindowState {
        WindowState::new()
    }

    fn from_str(s: &str) -> Result<Self, String> {
        toml::from_str(&s).map_err(|e| format!("{}", e))
    }
}


pub struct UiMutex<T: ?Sized> {
    thread: thread::ThreadId,
    data: RefCell<T>,
}

unsafe impl<T: ?Sized> Send for UiMutex<T> {}
unsafe impl<T: ?Sized> Sync for UiMutex<T> {}

impl<T> UiMutex<T> {
    pub fn new(t: T) -> UiMutex<T> {
        UiMutex {
            thread: thread::current().id(),
            data: RefCell::new(t),
        }
    }
}

impl<T: ?Sized> UiMutex<T> {
    pub fn borrow(&self) -> Ref<T> {
        self.assert_ui_thread();
        self.data.borrow()
    }

    pub fn borrow_mut(&self) -> RefMut<T> {
        self.assert_ui_thread();
        self.data.borrow_mut()
    }

    #[inline]
    fn assert_ui_thread(&self) {
        if thread::current().id() != self.thread {
            panic!("Can access to UI only from main thread");
        }
    }
}
