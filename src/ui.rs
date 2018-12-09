use std::cell::{Ref, RefCell, RefMut};
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;
use std::{env, thread};

use gdk::{self, ScreenExt};
use gio::prelude::*;
use gio::{Menu, MenuExt, MenuItem, SimpleAction};
use glib::variant::FromVariant;
use gtk;
use gtk::prelude::*;
use gtk::{AboutDialog, ApplicationWindow, Button, HeaderBar, Orientation, Paned, SettingsExt};

use toml;

use file_browser::FileBrowserWidget;
use misc;
use nvim::NvimCommand;
use plug_manager;
use project::Projects;
use settings::{Settings, SettingsLoader};
use shell::{self, Shell, ShellOptions};
use shell_dlg;
use subscriptions::{SubscriptionHandle, SubscriptionKey};

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

const DEFAULT_WIDTH: i32 = 800;
const DEFAULT_HEIGHT: i32 = 600;
const DEFAULT_SIDEBAR_WIDTH: i32 = 200;

pub struct Ui {
    initialized: bool,
    comps: Arc<UiMutex<Components>>,
    settings: Rc<RefCell<Settings>>,
    shell: Rc<RefCell<Shell>>,
    projects: Rc<RefCell<Projects>>,
    plug_manager: Arc<UiMutex<plug_manager::Manager>>,
    file_browser: Arc<UiMutex<FileBrowserWidget>>,
}

pub struct Components {
    window: Option<ApplicationWindow>,
    window_state: WindowState,
    open_btn: Button,
}

impl Components {
    fn new() -> Components {
        let open_btn = Button::new();
        let open_btn_box = gtk::Box::new(gtk::Orientation::Horizontal, 3);
        open_btn_box.pack_start(&gtk::Label::new("Open"), false, false, 3);
        open_btn_box.pack_start(
            &gtk::Image::new_from_icon_name("pan-down-symbolic", gtk::IconSize::Menu.into()),
            false,
            false,
            3,
        );
        open_btn.add(&open_btn_box);
        open_btn.set_can_focus(false);
        Components {
            open_btn,
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
        let file_browser = Arc::new(UiMutex::new(FileBrowserWidget::new()));
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
            file_browser,
        }
    }

    pub fn init(&mut self, app: &gtk::Application, restore_win_state: bool) {
        if self.initialized {
            return;
        }
        self.initialized = true;

        let mut settings = self.settings.borrow_mut();
        settings.init();

        let window = ApplicationWindow::new(app);

        let main = Paned::new(Orientation::Horizontal);

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

            if restore_win_state {
                if comps.window_state.is_maximized {
                    window.maximize();
                }

                window.set_default_size(
                    comps.window_state.current_width,
                    comps.window_state.current_height,
                );

                main.set_position(comps.window_state.sidebar_width);
            } else {
                window.set_default_size(DEFAULT_WIDTH, DEFAULT_HEIGHT);
                main.set_position(DEFAULT_SIDEBAR_WIDTH);
            }
        }

        // Client side decorations including the toolbar are disabled via NVIM_GTK_NO_HEADERBAR=1
        let use_header_bar = env::var("NVIM_GTK_NO_HEADERBAR")
            .map(|opt| opt.trim() != "1")
            .unwrap_or(true);

        let disable_window_decoration = env::var("NVIM_GTK_NO_WINDOW_DECORATION")
            .map(|opt| opt.trim() == "1")
            .unwrap_or(false);

        if disable_window_decoration {
            window.set_decorated(false);
        }

        let update_subtitle = if use_header_bar {
            Some(self.create_header_bar(app))
        } else {
            None
        };

        let show_sidebar_action =
            SimpleAction::new_stateful("show-sidebar", None, &false.to_variant());
        let file_browser_ref = self.file_browser.clone();
        let comps_ref = self.comps.clone();
        show_sidebar_action.connect_change_state(move |action, value| {
            if let Some(ref value) = *value {
                action.set_state(value);
                let is_active = value.get::<bool>().unwrap();
                file_browser_ref.borrow().set_visible(is_active);
                comps_ref.borrow_mut().window_state.show_sidebar = is_active;
            }
        });
        app.add_action(&show_sidebar_action);

        let comps_ref = self.comps.clone();
        window.connect_size_allocate(clone!(main => move |window, _| {
            gtk_window_size_allocate(
                window,
                &mut *comps_ref.borrow_mut(),
                &main,
            );
        }));

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
        let file_browser = self.file_browser.borrow();
        main.pack1(&**file_browser, false, false);
        main.pack2(&**shell, true, false);

        window.add(&main);

        window.show_all();

        if restore_win_state {
            // Hide sidebar, if it wasn't shown last time.
            // Has to be done after show_all(), so it won't be shown again.
            let show_sidebar = self.comps.borrow().window_state.show_sidebar;
            show_sidebar_action.change_state(&show_sidebar.to_variant());
        }

        let comps_ref = self.comps.clone();
        let update_title = shell.state.borrow().subscribe(
            SubscriptionKey::from("BufEnter,DirChanged"),
            &["expand('%:p')", "getcwd()"],
            move |args| update_window_title(&comps_ref, args),
        );

        let shell_ref = self.shell.clone();
        let update_completeopt = shell.state.borrow().subscribe(
            SubscriptionKey::with_pattern("OptionSet", "completeopt"),
            &["&completeopt"],
            move |args| set_completeopts(&*shell_ref, args),
        );

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
        let file_browser_ref = self.file_browser.clone();
        let plug_manager_ref = self.plug_manager.clone();
        shell.set_nvim_started_cb(Some(move || {
            let state = state_ref.borrow();
            plug_manager_ref
                .borrow_mut()
                .init_nvim_client(state_ref.borrow().nvim_clone());
            file_browser_ref.borrow_mut().init(&state);
            state.set_autocmds();
            state.run_now(&update_title);
            state.run_now(&update_completeopt);
            if let Some(ref update_subtitle) = update_subtitle {
                state.run_now(&update_subtitle);
            }
        }));

        let sidebar_action = UiMutex::new(show_sidebar_action);
        let comps_ref = self.comps.clone();
        shell.set_nvim_command_cb(Some(
            move |shell: &mut shell::State, command: NvimCommand| {
                Ui::nvim_command(shell, command, &sidebar_action, &comps_ref);
            },
        ));
    }

    fn nvim_command(
        shell: &mut shell::State,
        command: NvimCommand,
        sidebar_action: &UiMutex<SimpleAction>,
        comps: &UiMutex<Components>,
    ) {
        match command {
            NvimCommand::ToggleSidebar => {
                let action = sidebar_action.borrow();
                let state = !bool::from_variant(&action.get_state().unwrap()).unwrap();
                action.change_state(&state.to_variant());
            }
            NvimCommand::Transparency(background_alpha, filled_alpha) => {
                let comps = comps.borrow();
                let window = comps.window.as_ref().unwrap();

                let screen = window.get_screen().unwrap();
                if screen.is_composited() {
                    let enabled = shell.set_transparency(background_alpha, filled_alpha);
                    window.set_app_paintable(enabled);
                } else {
                    warn!("Screen is not composited");
                }
            }
            NvimCommand::PreferDarkTheme(prefer_dark_theme) => {
                let comps = comps.borrow();
                let window = comps.window.as_ref().unwrap();

                if let Some(settings) = window.get_settings() {
                    settings.set_property_gtk_application_prefer_dark_theme(prefer_dark_theme);
                }
            }
        }
    }

    fn create_header_bar(&self, app: &gtk::Application) -> SubscriptionHandle {
        let header_bar = HeaderBar::new();
        let comps = self.comps.borrow();
        let window = comps.window.as_ref().unwrap();

        let projects = self.projects.clone();
        header_bar.pack_start(&comps.open_btn);
        comps
            .open_btn
            .connect_clicked(move |_| projects.borrow_mut().show());

        let new_tab_btn =
            Button::new_from_icon_name("tab-new-symbolic", gtk::IconSize::SmallToolbar.into());
        let shell_ref = Rc::clone(&self.shell);
        new_tab_btn.connect_clicked(move |_| shell_ref.borrow_mut().new_tab());
        new_tab_btn.set_can_focus(false);
        new_tab_btn.set_tooltip_text("Open a new tab");
        header_bar.pack_start(&new_tab_btn);

        header_bar.pack_end(&self.create_primary_menu_btn(app, &window));

        let paste_btn =
            Button::new_from_icon_name("edit-paste-symbolic", gtk::IconSize::SmallToolbar.into());
        let shell = self.shell.clone();
        paste_btn.connect_clicked(move |_| shell.borrow_mut().edit_paste());
        paste_btn.set_can_focus(false);
        paste_btn.set_tooltip_text("Paste from clipboard");
        header_bar.pack_end(&paste_btn);

        let save_btn = Button::new_with_label("Save All");
        let shell = self.shell.clone();
        save_btn.connect_clicked(move |_| shell.borrow_mut().edit_save_all());
        save_btn.set_can_focus(false);
        header_bar.pack_end(&save_btn);

        header_bar.set_show_close_button(true);

        window.set_titlebar(Some(&header_bar));

        let shell = self.shell.borrow();

        let update_subtitle = shell.state.borrow().subscribe(
            SubscriptionKey::from("DirChanged"),
            &["getcwd()"],
            move |args| {
                header_bar.set_subtitle(&*args[0]);
            },
        );

        update_subtitle
    }

    fn create_primary_menu_btn(
        &self,
        app: &gtk::Application,
        window: &gtk::ApplicationWindow,
    ) -> gtk::MenuButton {
        let plug_manager = self.plug_manager.clone();
        let btn = gtk::MenuButton::new();
        btn.set_can_focus(false);
        btn.set_image(&gtk::Image::new_from_icon_name(
            "open-menu-symbolic",
            gtk::IconSize::SmallToolbar.into(),
        ));

        // note actions created in application menu
        let menu = Menu::new();

        let section = Menu::new();
        section.append_item(&MenuItem::new("New Window", "app.new-window"));
        menu.append_section(None, &section);

        let section = Menu::new();
        section.append_item(&MenuItem::new("Sidebar", "app.show-sidebar"));
        menu.append_section(None, &section);

        let section = Menu::new();
        section.append_item(&MenuItem::new("Plugins", "app.Plugins"));
        section.append_item(&MenuItem::new("About", "app.HelpAbout"));
        menu.append_section(None, &section);

        menu.freeze();

        let plugs_action = SimpleAction::new("Plugins", None);
        plugs_action.connect_activate(
            clone!(window => move |_, _| plug_manager::Ui::new(&plug_manager).show(&window)),
        );

        let about_action = SimpleAction::new("HelpAbout", None);
        about_action.connect_activate(clone!(window => move |_, _| on_help_about(&window)));
        about_action.set_enabled(true);

        app.add_action(&about_action);
        app.add_action(&plugs_action);

        btn.set_menu_model(&menu);
        btn
    }
}

fn on_help_about(window: &gtk::ApplicationWindow) {
    let about = AboutDialog::new();
    about.set_transient_for(window);
    about.set_program_name("NeovimGtk");
    about.set_version(env!("CARGO_PKG_VERSION"));
    about.set_logo_icon_name("org.daa.NeovimGtk");
    about.set_authors(&[env!("CARGO_PKG_AUTHORS")]);
    about.set_comments(misc::about_comments().as_str());

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

fn gtk_window_size_allocate(
    app_window: &gtk::ApplicationWindow,
    comps: &mut Components,
    main: &Paned,
) {
    if !app_window.is_maximized() {
        let (current_width, current_height) = app_window.get_size();
        comps.window_state.current_width = current_width;
        comps.window_state.current_height = current_height;
    }
    if comps.window_state.show_sidebar {
        comps.window_state.sidebar_width = main.get_position();
    }
}

fn gtk_window_state_event(event: &gdk::EventWindowState, comps: &mut Components) {
    comps.window_state.is_maximized = event
        .get_new_window_state()
        .contains(gdk::WindowState::MAXIMIZED);
}

fn set_completeopts(shell: &RefCell<Shell>, args: Vec<String>) {
    let options = &args[0];

    shell.borrow().set_completeopts(options);
}

fn update_window_title(comps: &Arc<UiMutex<Components>>, args: Vec<String>) {
    let comps_ref = comps.clone();
    let comps = comps_ref.borrow();
    let window = comps.window.as_ref().unwrap();

    let file_path = &args[0];
    let dir = Path::new(&args[1]);
    let filename = if file_path.is_empty() {
        "[No Name]"
    } else if let Some(rel_path) = Path::new(&file_path)
        .strip_prefix(&dir)
        .ok()
        .and_then(|p| p.to_str())
    {
        rel_path
    } else {
        &file_path
    };

    window.set_title(filename);
}

#[derive(Serialize, Deserialize)]
struct WindowState {
    current_width: i32,
    current_height: i32,
    is_maximized: bool,
    show_sidebar: bool,
    sidebar_width: i32,
}

impl Default for WindowState {
    fn default() -> Self {
        WindowState {
            current_width: DEFAULT_WIDTH,
            current_height: DEFAULT_HEIGHT,
            is_maximized: false,
            show_sidebar: false,
            sidebar_width: DEFAULT_SIDEBAR_WIDTH,
        }
    }
}

impl SettingsLoader for WindowState {
    const SETTINGS_FILE: &'static str = "window.toml";

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

impl<T> UiMutex<T> {
    pub fn replace(&self, t: T) -> T {
        self.assert_ui_thread();
        self.data.replace(t)
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
