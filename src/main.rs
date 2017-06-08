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
mod tabline;


use std::env;
use gtk::prelude::*;
use gio::{ApplicationExt, FileExt};

use ui::Ui;

const BIN_PATH_ARG: &'static str = "--nvim-bin-path";

fn main() {
    env_logger::init().expect("Can't initialize env_logger");

    let app_flags = gio::APPLICATION_HANDLES_OPEN;

    let app = if cfg!(debug_assertions) {
            gtk::Application::new(Some("org.daa.NeovimGtkDebug"),
                                  app_flags)
        } else {
            gtk::Application::new(Some("org.daa.NeovimGtk"), app_flags)
        }
        .expect("Failed to initialize GTK application");

    app.connect_activate(activate);
    {
        use gio::ApplicationExtManual;
        app.connect_open(open);
    }

    let args: Vec<String> = env::args().collect();
    let argv: Vec<&str> = args.iter()
        .filter(|a| !a.starts_with(BIN_PATH_ARG))
        .map(String::as_str)
        .collect();
    app.run(argv.len() as i32, &argv);
}

fn open(app: &gtk::Application, files: &[gio::File], _: &str) {
    for f in files {
        let mut ui = Ui::new();

        ui.init(app,
                nvim_bin_path(std::env::args()).as_ref(),
                f.get_path().as_ref().map(|p| p.to_str()).unwrap_or(None));
    }
}

fn activate(app: &gtk::Application) {
    let mut ui = Ui::new();

    ui.init(app,
            nvim_bin_path(std::env::args()).as_ref(),
            None);
}

fn nvim_bin_path<I>(args: I) -> Option<String>
    where I: Iterator<Item = String>
{
    args.skip_while(|a| !a.starts_with(BIN_PATH_ARG))
        .map(|p| p.split('=').nth(1).map(str::to_owned))
        .nth(0)
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
}
