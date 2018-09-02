#![windows_subsystem = "windows"]

extern crate fnv;
extern crate cairo;
extern crate env_logger;
extern crate gdk;
extern crate gdk_sys;
extern crate gio;
#[cfg(unix)]
extern crate unix_daemonize;
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
extern crate regex;
extern crate rmpv;
extern crate unicode_segmentation;
extern crate unicode_width;

extern crate atty;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate toml;

mod sys;

mod color;
mod dirs;
mod mode;
mod nvim_config;
mod theme;
mod ui_model;
mod value;
#[macro_use]
mod ui;
mod cmd_line;
mod cursor;
mod error;
mod file_browser;
mod input;
mod misc;
mod nvim;
mod plug_manager;
mod popup_menu;
mod project;
mod render;
mod settings;
mod shell;
mod highlight;
mod grid;
mod shell_dlg;
mod subscriptions;
mod tabline;

use gio::prelude::*;
use std::cell::RefCell;
use std::env;
use std::io::Read;
use std::str::FromStr;
use std::time::Duration;
#[cfg(unix)]
use unix_daemonize::{daemonize_redirect, ChdirMode};

use ui::Ui;

use shell::ShellOptions;

const BIN_PATH_ARG: &str = "--nvim-bin-path";
const TIMEOUT_ARG: &str = "--timeout";
const DISABLE_WIN_STATE_RESTORE: &str = "--disable-win-restore";
const NO_FORK: &str = "--no-fork";

fn main() {
    env_logger::init();

    let input_data = RefCell::new(read_piped_input());

    let argv: Vec<String> = env::args()
        .take_while(|a| *a != "--")
        .filter(|a| !a.starts_with(BIN_PATH_ARG))
        .filter(|a| !a.starts_with(TIMEOUT_ARG))
        .filter(|a| !a.starts_with(DISABLE_WIN_STATE_RESTORE))
        .filter(|a| !a.starts_with(NO_FORK))
        .collect();

    #[cfg(unix)]
    {
        // fork to background by default
        let want_fork = env::args()
            .take_while(|a| *a != "--")
            .skip(1)
            .find(|a| a.starts_with(NO_FORK))
            .is_none();

        if want_fork {
            daemonize_redirect(
                Some("/tmp/nvim-gtk_stdout.log"),
                Some("/tmp/nvim-gtk_stderr.log"),
                ChdirMode::NoChdir,
            ).unwrap();
        }
    }

    let app_flags = gio::ApplicationFlags::HANDLES_OPEN | gio::ApplicationFlags::NON_UNIQUE;

    glib::set_program_name(Some("NeovimGtk"));

    let app = if cfg!(debug_assertions) {
        gtk::Application::new(Some("org.daa.NeovimGtkDebug"), app_flags)
    } else {
        gtk::Application::new(Some("org.daa.NeovimGtk"), app_flags)
    }.expect("Failed to initialize GTK application");

    app.connect_activate(move |app| activate(app, input_data.replace(None)));
    app.connect_open(open);

    let new_window_action = gio::SimpleAction::new("new-window", None);
    let app_ref = app.clone();
    new_window_action.connect_activate(move |_, _| activate(&app_ref, None));
    app.add_action(&new_window_action);

    gtk::Window::set_default_icon_name("org.daa.NeovimGtk");

    app.run(&argv);
}

fn collect_args_for_nvim() -> Vec<String> {
    std::env::args()
        .skip_while(|a| *a != "--")
        .skip(1)
        .collect()
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
        collect_args_for_nvim(),
        None,
    ));

    ui.init(app, !nvim_disable_win_state(std::env::args()));
}

fn activate(app: &gtk::Application, input_data: Option<String>) {
    let mut ui = Ui::new(ShellOptions::new(
        nvim_bin_path(std::env::args()),
        Vec::new(),
        nvim_timeout(std::env::args()),
        collect_args_for_nvim(),
        input_data,
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

fn read_piped_input() -> Option<String> {
    if atty::isnt(atty::Stream::Stdin) {
        let mut buf = String::new();
        match std::io::stdin().read_to_string(&mut buf) {
            Ok(size) if size > 0 => Some(buf),
            Ok(_) => None,
            Err(err) => {
                error!("Error read stdin {}", err);
                None
            }
        }
    } else {
        None
    }
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
                    .map(|s| s.to_string())
            )
        );
    }
}
