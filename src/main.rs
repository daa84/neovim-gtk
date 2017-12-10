extern crate gtk;
extern crate gtk_sys;
extern crate gio;
extern crate gdk;
extern crate gdk_sys;
#[macro_use]
extern crate glib;
extern crate glib_sys as glib_ffi;
extern crate gobject_sys as gobject_ffi;
extern crate cairo;
extern crate pango;
extern crate pango_sys;
extern crate pangocairo;
extern crate neovim_lib;
extern crate phf;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate htmlescape;

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate toml;
extern crate serde_json;

mod sys;

mod nvim_config;
mod dirs;
mod theme;
mod color;
mod value;
mod mode;
mod ui_model;
#[macro_use]
mod ui;
mod plug_manager;
mod nvim;
mod render;
mod shell;
mod input;
mod settings;
mod cursor;
mod shell_dlg;
mod popup_menu;
mod project;
mod tabline;
mod error;


use std::env;
use std::time::Duration;
use std::str::FromStr;
use gio::{ApplicationExt, FileExt};

use ui::Ui;

use shell::ShellOptions;

const BIN_PATH_ARG: &str = "--nvim-bin-path";
const TIMEOUT_ARG: &str = "--timeout";

fn main() {
    env_logger::init().expect("Can't initialize env_logger");

    let app_flags = gio::APPLICATION_HANDLES_OPEN;

    let app = if cfg!(debug_assertions) {
        gtk::Application::new(Some("org.daa.NeovimGtkDebug"), app_flags)
    } else {
        gtk::Application::new(Some("org.daa.NeovimGtk"), app_flags)
    }.expect("Failed to initialize GTK application");

    app.connect_activate(activate);
    {
        use gio::ApplicationExtManual;
        app.connect_open(open);
    }

    gtk::Window::set_default_icon_name("org.daa.NeovimGtk");

    let args: Vec<String> = env::args().collect();
    let argv: Vec<&str> = args.iter()
        .filter(|a| !a.starts_with(BIN_PATH_ARG))
        .filter(|a| !a.starts_with(TIMEOUT_ARG))
        .map(String::as_str)
        .collect();
    app.run(&argv);
}

fn open(app: &gtk::Application, files: &[gio::File], _: &str) {
    for f in files {
        let mut ui = Ui::new(ShellOptions::new(
            nvim_bin_path(std::env::args()),
            f.get_path().and_then(|p| p.to_str().map(str::to_owned)),
            nvim_timeout(std::env::args()),
        ));

        ui.init(app);
    }
}

fn activate(app: &gtk::Application) {
    let mut ui = Ui::new(ShellOptions::new(
        nvim_bin_path(std::env::args()),
        None,
        nvim_timeout(std::env::args()),
    ));

    ui.init(app);
}

fn nvim_bin_path<I>(mut args: I) -> Option<String>
where
    I: Iterator<Item = String>,
{
    args.find(|a| a.starts_with(BIN_PATH_ARG)).and_then(|p| {
        p.split('=').nth(1).map(str::to_owned)
    })
}

fn nvim_timeout<I>(mut args: I) -> Option<Duration>
where
    I: Iterator<Item = String>,
{
    args.find(|a| a.starts_with(TIMEOUT_ARG))
        .and_then(|p| p.split('=').nth(1).map(str::to_owned))
        .and_then(|timeout| match u64::from_str(&timeout) {
            Ok(timeout) => Some(timeout),
            Err(err) => {
                error!("Can't convert timeout argument to integer: {}", err);
                None
            }
        })
        .map(|timeout| Duration::from_secs(timeout))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bin_path_arg() {
        assert_eq!(
            Some("/test_path".to_string()),
            nvim_bin_path(
                vec!["neovim-gtk", "--nvim-bin-path=/test_path"]
                    .iter()
                    .map(|s| s.to_string()),
            )
        );
    }


    #[test]
    fn test_timeout_arg() {
        assert_eq!(
            Some(Duration::from_secs(100)),
            nvim_timeout(vec!["neovim-gtk", "--timeout=100"].iter().map(
                |s| s.to_string(),
            ))
        );
    }
}
