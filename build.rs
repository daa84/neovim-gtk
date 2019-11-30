extern crate build_version;
extern crate phf_codegen;

#[cfg(windows)]
extern crate winres;

use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

fn main() {
    build_version::write_version_file().expect("Failed to write version.rs file");

    if cfg!(target_os = "windows") {
        println!("cargo:rustc-link-search=native=C:\\msys64\\mingw64\\lib");

        set_win_icon();
    }

    let path = Path::new(&env::var("OUT_DIR").unwrap()).join("key_map_table.rs");
    let mut file = BufWriter::new(File::create(&path).unwrap());

    writeln!(
        &mut file,
        "static KEYVAL_MAP: phf::Map<&'static str, &'static str> = \n{};\n",
        phf_codegen::Map::new()
            .entry("F1", "\"F1\"")
            .entry("F2", "\"F2\"")
            .entry("F3", "\"F3\"")
            .entry("F4", "\"F4\"")
            .entry("F5", "\"F5\"")
            .entry("F6", "\"F6\"")
            .entry("F7", "\"F7\"")
            .entry("F8", "\"F8\"")
            .entry("F9", "\"F9\"")
            .entry("F10", "\"F10\"")
            .entry("F11", "\"F11\"")
            .entry("F12", "\"F12\"")
            .entry("Left", "\"Left\"")
            .entry("Right", "\"Right\"")
            .entry("Up", "\"Up\"")
            .entry("Down", "\"Down\"")
            .entry("Home", "\"Home\"")
            .entry("End", "\"End\"")
            .entry("BackSpace", "\"BS\"")
            .entry("Return", "\"CR\"")
            .entry("Escape", "\"Esc\"")
            .entry("Delete", "\"Del\"")
            .entry("Insert", "\"Insert\"")
            .entry("Page_Up", "\"PageUp\"")
            .entry("Page_Down", "\"PageDown\"")
            .entry("Enter", "\"CR\"")
            .entry("Tab", "\"Tab\"")
            .entry("ISO_Left_Tab", "\"Tab\"")
            .build()
    )
    .unwrap();
}

#[cfg(windows)]
fn set_win_icon() {
    let mut res = winres::WindowsResource::new();
    res.set_icon("resources/neovim.ico");
    if let Err(err) = res.compile() {
        eprintln!("Error set icon: {}", err);
    }
}

#[cfg(unix)]
fn set_win_icon() {}
