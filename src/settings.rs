use std::rc::{Rc, Weak};
use std::cell::RefCell;

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
            self.gnome_interface_settings.get_string(
                "monospace-font-name",
            )
        {
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
        self.state.borrow_mut().update_font(
            &mut *shell.borrow_mut(),
        );
        self.state
            .borrow()
            .gnome_interface_settings
            .connect_changed(move |_, _| {
                monospace_font_changed(&mut *shell.borrow_mut(), &mut *state.borrow_mut())
            });
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
    }
}

use std::path::Path;
use std::fs::File;
use std::io::prelude::*;

use toml;
use serde;

use dirs;

pub trait SettingsLoader: Sized + serde::Serialize {
    const SETTINGS_FILE: &'static str;

    fn empty() -> Self;

    fn from_str(s: &str) -> Result<Self, String>;

    fn load() -> Self {
        match load_err() {
            Ok(settings) => settings,
            Err(e) => {
                error!("{}", e);
                Self::empty()
            }
        }
    }

    fn is_file_exists() -> bool {
        if let Ok(mut toml_path) = dirs::get_app_config_dir() {
            toml_path.push(Self::SETTINGS_FILE);
            toml_path.is_file()
        } else {
            false
        }
    }

    fn save(&self) {
        match save_err(self) {
            Ok(()) => (),
            Err(e) => error!("{}", e),
        }
    }
}

fn load_from_file<T: SettingsLoader>(path: &Path) -> Result<T, String> {
    if path.exists() {
        let mut file = File::open(path).map_err(|e| format!("{}", e))?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).map_err(
            |e| format!("{}", e),
        )?;
        T::from_str(&contents)
    } else {
        Ok(T::empty())
    }
}

fn load_err<T: SettingsLoader>() -> Result<T, String> {
    let mut toml_path = dirs::get_app_config_dir_create()?;
    toml_path.push(T::SETTINGS_FILE);
    load_from_file(&toml_path)
}


fn save_err<T: SettingsLoader>(sl: &T) -> Result<(), String> {
    let mut toml_path = dirs::get_app_config_dir_create()?;
    toml_path.push(T::SETTINGS_FILE);
    let mut file = File::create(toml_path).map_err(|e| format!("{}", e))?;

    let contents = toml::to_vec::<T>(sl).map_err(|e| format!("{}", e))?;

    file.write_all(&contents).map_err(|e| format!("{}", e))?;

    Ok(())
}
