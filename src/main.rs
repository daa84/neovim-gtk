#![windows_subsystem = "windows"]

extern crate cairo;
extern crate env_logger;
extern crate gdk;
extern crate gdk_sys;
extern crate gio;
#[macro_use]
extern crate glib;
extern crate glib_sys as glib_ffi;
extern crate gobject_sys as gobject_ffi;
extern crate gtk;
extern crate gtk_sys;
extern crate htmlescape;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
extern crate neovim_lib;
extern crate pango;
extern crate pango_cairo_sys;
extern crate pango_sys;
extern crate pangocairo;
extern crate percent_encoding;
extern crate phf;
extern crate rmpv;
extern crate regex;
extern crate unicode_width;

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate toml;

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
mod cmd_line;
mod shell_dlg;
mod popup_menu;
mod project;
mod tabline;
mod error;
mod file_browser;
mod subscriptions;
mod misc;

use std::env;
use std::time::Duration;
use std::str::FromStr;
use gio::prelude::*;

use ui::Ui;

use shell::ShellOptions;

const BIN_PATH_ARG: &str = "--nvim-bin-path";
const TIMEOUT_ARG: &str = "--timeout";
const DISABLE_WIN_STATE_RESTORE: &str = "--disable-win-restore";

fn main() {
    env_logger::init();

    let app_flags = gio::ApplicationFlags::HANDLES_OPEN | gio::ApplicationFlags::NON_UNIQUE;

    glib::set_program_name(Some("NeovimGtk"));

    let app = if cfg!(debug_assertions) {
        gtk::Application::new(Some("org.daa.NeovimGtkDebug"), app_flags)
    } else {
        gtk::Application::new(Some("org.daa.NeovimGtk"), app_flags)
    }.expect("Failed to initialize GTK application");

    app.connect_activate(activate);
    app.connect_open(open);

    let new_window_action = gio::SimpleAction::new("new-window", None);
    let app_ref = app.clone();
    new_window_action.connect_activate(move |_, _| activate(&app_ref));
    app.add_action(&new_window_action);

    gtk::Window::set_default_icon_name("org.daa.NeovimGtk");

    let args: Vec<String> = env::args().collect();
    let argv: Vec<String> = args.iter()
        .filter(|a| !a.starts_with(BIN_PATH_ARG))
        .filter(|a| !a.starts_with(TIMEOUT_ARG))
        .filter(|a| !a.starts_with(DISABLE_WIN_STATE_RESTORE))
        .cloned()
        .collect();
    app.run(&argv);
}

fn open(app: &gtk::Application, files: &[gio::File], _: &str) {
    let files_list: Vec<String> = files
        .into_iter()
        .filter_map(|f| f.get_path()?.to_str().map(str::to_owned))
        .collect();
    let mut ui = Ui::new(ShellOptions::new(
        nvim_bin_path(std::env::args()),
        files_list,
        nvim_timeout(std::env::args()),
    ));

    ui.init(app, !nvim_disable_win_state(std::env::args()));
}

fn activate(app: &gtk::Application) {
    let mut ui = Ui::new(ShellOptions::new(
        nvim_bin_path(std::env::args()),
        Vec::new(),
        nvim_timeout(std::env::args()),
    ));

    ui.init(app, !nvim_disable_win_state(std::env::args()));
}

fn nvim_bin_path<I>(mut args: I) -> Option<String>
where
    I: Iterator<Item = String>,
{
    args.find(|a| a.starts_with(BIN_PATH_ARG))
        .and_then(|p| p.split('=').nth(1).map(str::to_owned))
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

fn nvim_disable_win_state<I>(mut args: I) -> bool
where
    I: Iterator<Item = String>,
{
    args.find(|a| a.starts_with(DISABLE_WIN_STATE_RESTORE))
        .map(|_| true)
        .unwrap_or(false)
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
            nvim_timeout(
                vec!["neovim-gtk", "--timeout=100"]
                    .iter()
                    .map(|s| s.to_string(),)
            )
        );
    }
}
