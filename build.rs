extern crate phf_codegen;

use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

fn main() {
    if cfg!(target_os = "windows") {
        println!("cargo:rustc-link-search=native=C:\\msys64\\mingw64\\lib");
    }

    let path = Path::new(&env::var("OUT_DIR").unwrap()).join("key_map_table.rs");
    let mut file = BufWriter::new(File::create(&path).unwrap());

    write!(&mut file, "static KEYVAL_MAP: phf::Map<&'static str, &'static str> = ").unwrap();
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
        .entry("BackSpace", "\"BS\"")
        .entry("Return", "\"CR\"")
        .entry("Escape", "\"Esc\"")
        .entry("Delete", "\"Del\"")
        .entry("Page_Up", "\"PageUp\"")
        .entry("Page_Down", "\"PageDown\"")
        .entry("Enter", "\"CR\"")
        .entry("Tab", "\"Tab\"")
        .entry("ISO_Left_Tab", "\"Tab\"")
        .build(&mut file)
        .unwrap();
    write!(&mut file, ";\n").unwrap();
}
