use std::cell::{RefCell, Ref, RefMut};
use std::{env, thread};
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

use gtk;
use gtk::prelude::*;
use gtk::{ApplicationWindow, HeaderBar, Button, AboutDialog, SettingsExt, Paned, Orientation};
use gio;
use gio::prelude::*;
use gio::{Menu, MenuExt, MenuItem, SimpleAction};

use settings::Settings;
use shell::{self, Shell, ShellOptions};
use shell_dlg;
use project::Projects;
use plug_manager;
use file_browser::FileBrowserWidget;
use subscriptions::SubscriptionHandle;

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
    file_browser: Arc<UiMutex<FileBrowserWidget>>,
}

pub struct Components {
    window: Option<ApplicationWindow>,
    open_btn: Button,
}

impl Components {
    fn new() -> Components {
        let open_btn = Button::new();
        let open_btn_box = gtk::Box::new(gtk::Orientation::Horizontal, 3);
        open_btn_box.pack_start(&gtk::Label::new("Open"), false, false, 3);
        open_btn_box.pack_start(
            &gtk::Image::new_from_icon_name("pan-down-symbolic", gtk::IconSize::Menu.into()),
            false, false, 3
        );
        open_btn.add(&open_btn_box);
        open_btn.set_can_focus(false);
        Components {
            open_btn,
            window: None,
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

    pub fn init(&mut self, app: &gtk::Application) {
        if self.initialized {
            return;
        }
        self.initialized = true;

        let mut settings = self.settings.borrow_mut();
        settings.init();

        self.shell.borrow_mut().init();

        self.comps.borrow_mut().window = Some(ApplicationWindow::new(app));

        let comps = self.comps.borrow();
        let window = comps.window.as_ref().unwrap();

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
            self.create_main_menu(app);
        }

        let update_subtitle = if use_header_bar {
            Some(self.create_header_bar())
        } else {
            None
        };

        let show_sidebar_action =
            gio::SimpleAction::new_stateful("show-sidebar", None, &true.to_variant());
        let file_browser_ref = self.file_browser.clone();
        show_sidebar_action.connect_activate(move |action, _| {
            if let Some(state) = action.get_state() {
                let is_active = !state.get::<bool>().unwrap();
                action.change_state(&(is_active).to_variant());
                let file_browser = file_browser_ref.borrow();
                file_browser.set_visible(is_active);
            }
        });
        app.add_action(&show_sidebar_action);

        window.set_default_size(1200, 800);

        let main = Paned::new(Orientation::Horizontal);
        let shell = self.shell.borrow();
        let file_browser = self.file_browser.borrow();
        main.pack1(&**file_browser, false, false);
        main.pack2(&**shell, true, false);

        window.add(&main);

        window.show_all();

        let comps_ref = self.comps.clone();
        let update_title = shell.state.borrow()
            .subscribe("BufEnter,DirChanged", &["expand('%:p')", "getcwd()"], move |args| {
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
            });

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
            plug_manager_ref.borrow_mut().init_nvim_client(state.nvim_clone());
            file_browser_ref.borrow_mut().init(&state);
            state.set_autocmds();
            state.run_now(&update_title);
            if let Some(ref update_subtitle) = update_subtitle {
                state.run_now(&update_subtitle);
            }
        }));
    }

    fn create_header_bar(&self) -> SubscriptionHandle {
        let header_bar = HeaderBar::new();
        let shell = self.shell.borrow();
        let comps = self.comps.borrow();
        let window = comps.window.as_ref().unwrap();

        let projects = self.projects.clone();
        header_bar.pack_start(&comps.open_btn);
        comps.open_btn.connect_clicked(
            move |_| projects.borrow_mut().show(),
        );

        let new_tab_btn = Button::new_from_icon_name(
            "tab-new-symbolic",
            gtk::IconSize::SmallToolbar.into(),
        );
        let shell_ref = Rc::clone(&self.shell);
        new_tab_btn.connect_clicked(move |_| shell_ref.borrow_mut().new_tab());
        new_tab_btn.set_can_focus(false);
        header_bar.pack_start(&new_tab_btn);

        let builder = gtk::Builder::new_from_string(include_str!("../resources/menu.ui"));
        let menu: gio::MenuModel = builder.get_object("hamburger-menu").unwrap();

        let paste_action = gio::SimpleAction::new("paste", None);
        let shell_ref = self.shell.clone();
        paste_action.connect_activate(move |_, _| shell_ref.borrow_mut().edit_paste());
        window.add_action(&paste_action);

        let save_all_action = gio::SimpleAction::new("save-all", None);
        let shell_ref = self.shell.clone();
        save_all_action.connect_activate(move |_, _| shell_ref.borrow_mut().edit_save_all());
        window.add_action(&save_all_action);

        let menu_btn = gtk::MenuButton::new();
        menu_btn.set_image(&gtk::Image::new_from_icon_name(
            "open-menu-symbolic",
            gtk::IconSize::SmallToolbar.into(),
        ));
        menu_btn.set_menu_model(&menu);
        menu_btn.set_can_focus(false);
        header_bar.pack_end(&menu_btn);

        header_bar.set_show_close_button(true);

        window.set_titlebar(Some(&header_bar));

        let update_subtitle = shell.state.borrow()
            .subscribe("DirChanged", &["getcwd()"], move |args| {
                header_bar.set_subtitle(&*args[0]);
            });

        update_subtitle
    }

    fn create_main_menu(&self, app: &gtk::Application) {
        let comps = self.comps.clone();
        let plug_manager = self.plug_manager.clone();

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
        app.set_app_menu(Some(&menu));

        let plugs_action = SimpleAction::new("Plugins", None);
        plugs_action.connect_activate(
            clone!(comps => move |_, _| plug_manager::Ui::new(&plug_manager).show(
                    comps
                    .borrow()
                    .window
                    .as_ref()
                    .unwrap(),
                    )),
        );

        let about_action = SimpleAction::new("HelpAbout", None);
        about_action.connect_activate(move |_, _| on_help_about(&*comps.borrow()));
        about_action.set_enabled(true);

        app.add_action(&about_action);
        app.add_action(&plugs_action);
    }
}

fn on_help_about(comps: &Components) {
    let about = AboutDialog::new();
    about.set_transient_for(comps.window.as_ref());
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
