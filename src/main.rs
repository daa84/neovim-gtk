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

mod nvim;
mod ui_model;
#[macro_use]
mod ui;
mod shell;
mod input;
mod settings;
mod cursor;

use std::thread;
use std::env;
use gio::ApplicationExt;

use shell::Shell;

const BIN_PATH_ARG: &'static str = "--nvim-bin-path";

fn main() {
    let app = gtk::Application::new(Some("org.daa.NeovimGtk"), gio::ApplicationFlags::empty())
        .expect("Failed to initialize GTK application");

    app.connect_activate(activate);

    let args: Vec<String> = env::args().collect();
    let mut argv: Vec<&str> =
        args.iter().filter(|a| !a.starts_with(BIN_PATH_ARG)).map(String::as_str).collect();
    if open_arg().is_some() {
        argv.pop();
    }
    app.run(argv.len() as i32, &argv);
}

fn activate(app: &gtk::Application) {
    ui::UI.with(|ui_cell| {
        let mut ui = ui_cell.borrow_mut();
        if !ui.initialized {
            ui.init(app);

            let path = nvim_bin_path(std::env::args());
            nvim::initialize(&mut ui.shell, path.as_ref())
                .expect("Can't start nvim instance");

            guard_dispatch_thread(&mut ui.shell);
        }

        nvim::open_file(ui.shell.nvim(), open_arg().as_ref());
    });
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

fn guard_dispatch_thread(shell: &mut Shell) {
    let guard = shell.nvim().session.take_dispatch_guard();
    thread::spawn(move || {
        guard.join().expect("Can't join dispatch thread");
        glib::idle_add(move || {
            ui::UI.with(|ui_cell| { ui_cell.borrow().close_window(); });
            glib::Continue(false)
        });
    });
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
