use std::cell::{RefCell, Ref, RefMut};
use std::thread;
use std::rc::Rc;
use std::sync::Arc;

use gtk;
use gtk_sys;
use gtk::prelude::*;
use gtk::{ApplicationWindow, HeaderBar, ToolButton, Image, AboutDialog};
use gio::{Menu, MenuItem, SimpleAction};
use glib;

use settings::Settings;
use shell::Shell;
use shell_dlg;
use project::Projects;

pub struct Ui {
    initialized: bool,
    comps: Arc<UiMutex<Components>>,
    settings: Rc<RefCell<Settings>>,
    shell: Rc<RefCell<Shell>>,
    projects: Rc<RefCell<Projects>>,
}

pub struct Components {
    window: Option<ApplicationWindow>,
    header_bar: HeaderBar,
    open_btn: ToolButton,
}

impl Components {
    fn new() -> Components {
        let save_image = Image::new_from_icon_name("document-open",
                                                   gtk_sys::GTK_ICON_SIZE_SMALL_TOOLBAR as i32);

        Components {
            open_btn: ToolButton::new(Some(&save_image), "Open"),
            window: None,
            header_bar: HeaderBar::new(),
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
    pub fn new() -> Ui {
        let comps = Arc::new(UiMutex::new(Components::new()));
        let settings = Rc::new(RefCell::new(Settings::new()));
        let shell = Rc::new(RefCell::new(Shell::new(settings.clone(), &comps)));
        settings.borrow_mut().set_shell(Rc::downgrade(&shell));

        let projects = Projects::new(&comps.borrow().open_btn, shell.clone());

        Ui {
            initialized: false,
            comps,
            shell,
            settings,
            projects,
        }
    }

    pub fn init(&mut self,
                app: &gtk::Application,
                nvim_bin_path: Option<&String>,
                open_path: Option<&String>) {
        if self.initialized {
            return;
        }
        self.initialized = true;

        self.create_main_menu(app);

        let mut settings = self.settings.borrow_mut();
        settings.init();

        let mut comps = self.comps.borrow_mut();

        comps.header_bar.set_show_close_button(true);


        let projects = self.projects.clone();
        comps.header_bar.pack_start(&comps.open_btn);
        comps.open_btn.connect_clicked(move |_| projects.borrow_mut().show());

        let save_image = Image::new_from_icon_name("document-save",
                                                   gtk_sys::GTK_ICON_SIZE_SMALL_TOOLBAR as i32);
        let save_btn = ToolButton::new(Some(&save_image), "Save");

        let shell = self.shell.clone();
        save_btn.connect_clicked(move |_| shell.borrow_mut().edit_save_all());
        comps.header_bar.pack_start(&save_btn);

        let paste_image = Image::new_from_icon_name("edit-paste",
                                                    gtk_sys::GTK_ICON_SIZE_SMALL_TOOLBAR as i32);
        let paste_btn = ToolButton::new(Some(&paste_image), "Paste");
        let shell = self.shell.clone();
        paste_btn.connect_clicked(move |_| shell.borrow_mut().edit_paste());
        comps.header_bar.pack_start(&paste_btn);

        self.shell.borrow_mut().init();

        comps.window = Some(ApplicationWindow::new(app));
        let window = comps.window.as_ref().unwrap();

        window.set_titlebar(Some(&comps.header_bar));

        let mut shell = self.shell.borrow_mut();
        window.add(&*shell.drawing_area());

        window.show_all();
        window.set_title("Neovim-gtk");

        let comps_ref = self.comps.clone();
        let shell_ref = self.shell.clone();
        window.connect_delete_event(move |_, _| gtk_delete(&*comps_ref, &*shell_ref));

        shell.add_configure_event();
        shell.init_nvim(nvim_bin_path);

        if open_path.is_some() {
            shell.open_file(open_path.unwrap());
        }

        self.guard_dispatch_thread(&mut shell);
    }

    fn guard_dispatch_thread(&self, shell: &mut Shell) {
        let state = shell.state.borrow();
        let guard = state.nvim().session.take_dispatch_guard();

        let comps = self.comps.clone();
        thread::spawn(move || {
                          guard.join().expect("Can't join dispatch thread");
                          glib::idle_add(move || {
                                             comps.borrow().close_window();
                                             glib::Continue(false)
                                         });
                      });
    }

    fn create_main_menu(&self, app: &gtk::Application) {
        let menu = Menu::new();
        let help = Menu::new();

        let about = MenuItem::new("About", None);
        about.set_detailed_action("app.HelpAbout");
        help.append_item(&about);


        menu.append_item(&MenuItem::new_submenu("Help", &help));

        app.set_menubar(Some(&menu));


        let about_action = SimpleAction::new("HelpAbout", None);
        let comps = self.comps.clone();
        about_action.connect_activate(move |_, _| on_help_about(&*comps.borrow()));
        about_action.set_enabled(true);
        app.add_action(&about_action);
    }
}

fn on_help_about(comps: &Components) {
    let about = AboutDialog::new();
    about.set_transient_for(comps.window.as_ref());
    about.set_program_name("NeovimGtk");
    about.set_version(env!("CARGO_PKG_VERSION"));
    about.set_logo(None);
    about.set_authors(&[env!("CARGO_PKG_AUTHORS")]);

    about.connect_response(|about, _| about.destroy());
    about.show();
}

fn gtk_delete(comps: &UiMutex<Components>, shell: &RefCell<Shell>) -> Inhibit {
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
    thread: String,
    data: RefCell<T>,
}

unsafe impl<T: ?Sized> Send for UiMutex<T> {}
unsafe impl<T: ?Sized> Sync for UiMutex<T> {}

impl<T> UiMutex<T> {
    pub fn new(t: T) -> UiMutex<T> {
        UiMutex {
            thread: thread::current()
                .name()
                .expect("Can create UI  only from main thread, current thiread has no name")
                .to_owned(),
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
        match thread::current().name() {
            Some(name) if name == self.thread => (),
            Some(name) => {
                panic!("Can create UI  only from main thread, {}", name);
            }
            None => panic!("Can create UI  only from main thread, current thread has no name"),
        }
    }
}
