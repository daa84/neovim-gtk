use std::rc::{Rc, Weak};
use std::cell::RefCell;

#[cfg(unix)]
use nvim::RepaintMode;

use shell::Shell;
#[cfg(unix)]
use gio;
#[cfg(unix)]
use gio::SettingsExt;

#[derive(PartialEq)]
pub enum FontSource {
    Rpc,
    #[cfg(unix)]
    Gnome,
    Default,
}

struct State {
    font_source: FontSource,

    #[cfg(unix)]
    gnome_interface_settings: gio::Settings,
}

impl State {
    #[cfg(unix)]
    pub fn new() -> State {
        State {
            font_source: FontSource::Default,
            gnome_interface_settings: gio::Settings::new("org.gnome.desktop.interface"),
        }
    }

    #[cfg(target_os = "windows")]
    pub fn new() -> State {
        State { font_source: FontSource::Default }
    }

    #[cfg(unix)]
    fn update_font(&mut self, shell: &mut Shell) {
        // rpc is priority for font
        if self.font_source == FontSource::Rpc {
            return;
        }

        if let Some(ref font_name) =
            self.gnome_interface_settings
                .get_string("monospace-font-name") {
            shell.set_font_desc(font_name);
            self.font_source = FontSource::Gnome;
        }
    }
}

pub struct Settings {
    shell: Option<Weak<RefCell<Shell>>>,
    state: Rc<RefCell<State>>,
}

impl Settings {
    pub fn new() -> Settings {
        Settings {
            shell: None,
            state: Rc::new(RefCell::new(State::new())),
        }
    }

    pub fn set_shell(&mut self, shell: Weak<RefCell<Shell>>) {
        self.shell = Some(shell);
    }

    #[cfg(unix)]
    pub fn init(&mut self) {
        let shell = Weak::upgrade(self.shell.as_ref().unwrap()).unwrap();
        let state = self.state.clone();
        self.state.borrow_mut().update_font(&mut *shell.borrow_mut());
        self.state
            .borrow()
            .gnome_interface_settings
            .connect_changed(move |_, _| monospace_font_changed(&mut *shell.borrow_mut(), &mut *state.borrow_mut()));
    }

    #[cfg(target_os = "windows")]
    pub fn init(&mut self) {}

    pub fn set_font_source(&mut self, src: FontSource) {
        self.state.borrow_mut().font_source = src;
    }
}

#[cfg(unix)]
fn monospace_font_changed(mut shell: &mut Shell, state: &mut State) {
    // rpc is priority for font
    if state.font_source != FontSource::Rpc {
        state.update_font(&mut shell);
        shell.redraw(&RepaintMode::All);
    }
}
