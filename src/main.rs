extern crate gtk;
extern crate gtk_sys;
extern crate gio;
extern crate gdk;
extern crate gdk_sys;
extern crate glib;
extern crate cairo;
extern crate pango;
extern crate pangocairo;
extern crate neovim_lib;
extern crate phf;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate htmlescape;

#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate toml;

mod ui_model;
#[macro_use]
mod ui;
mod nvim;
mod shell;
mod input;
mod settings;
mod cursor;
mod shell_dlg;
mod popup_menu;
mod project;

use std::env;
use gio::ApplicationExt;

use ui::Ui;

const BIN_PATH_ARG: &'static str = "--nvim-bin-path";

fn main() {
    env_logger::init().expect("Can't initialize env_logger");

    let app = if cfg!(debug_assertions) {
            gtk::Application::new(Some("org.daa.NeovimGtkDebug"),
                                  gio::ApplicationFlags::empty())
        } else {
            gtk::Application::new(Some("org.daa.NeovimGtk"), gio::ApplicationFlags::empty())
        }
        .expect("Failed to initialize GTK application");

    app.connect_activate(activate);

    let args: Vec<String> = env::args().collect();
    let mut argv: Vec<&str> = args.iter()
        .filter(|a| !a.starts_with(BIN_PATH_ARG))
        .map(String::as_str)
        .collect();
    if open_arg().is_some() {
        argv.pop();
    }
    app.run(argv.len() as i32, &argv);
}

fn activate(app: &gtk::Application) {
    let mut ui = Ui::new();

    ui.init(app,
            nvim_bin_path(std::env::args()).as_ref(),
            open_arg().as_ref());
}

fn nvim_bin_path<I>(args: I) -> Option<String>
    where I: Iterator<Item = String>
{
    args.skip_while(|a| !a.starts_with(BIN_PATH_ARG))
        .map(|p| p.split('=').nth(1).map(str::to_owned))
        .nth(0)
        .unwrap_or(None)
}

fn open_arg() -> Option<String> {
    open_arg_impl(std::env::args())
}

fn open_arg_impl<I>(args: I) -> Option<String>
    where I: Iterator<Item = String>
{
    args.skip(1)
        .last()
        .map(|a| if !a.starts_with("-") {
                 Some(a.to_owned())
             } else {
                 None
             })
        .unwrap_or(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bin_path_arg() {
        assert_eq!(Some("/test_path".to_string()),
                   nvim_bin_path(vec!["neovim-gtk", "--nvim-bin-path=/test_path"]
                                     .iter()
                                     .map(|s| s.to_string())));
    }

    #[test]
    fn test_open_arg() {
        assert_eq!(Some("some_file.txt".to_string()),
                   open_arg_impl(vec!["neovim-gtk",
                                      "--nvim-bin-path=/test_path",
                                      "some_file.txt"]
                                         .iter()
                                         .map(|s| s.to_string())));
    }

    #[test]
    fn test_empty_open_arg() {
        assert_eq!(None,
                   open_arg_impl(vec!["neovim-gtk"].iter().map(|s| s.to_string())));
    }
}
