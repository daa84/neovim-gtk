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
        .entry("slash", "\"/\"")
        .entry("backslash", "\"\\\\\"")
        .entry("dead_circumflex", "\"^\"")
        .entry("at", "\"@\"")
        .entry("numbersign", "\"#\"")
        .entry("dollar", "\"$\"")
        .entry("percent", "\"%\"")
        .entry("ampersand", "\"&\"")
        .entry("asterisk", "\"*\"")
        .entry("parenleft", "\"(\"")
        .entry("parenright", "\")\"")
        .entry("underscore", "\"_\"")
        .entry("plus", "\"+\"")
        .entry("minus", "\"-\"")
        .entry("bracketleft", "\"[\"")
        .entry("bracketright", "\"]\"")
        .entry("braceleft", "\"{\"")
        .entry("braceright", "\"}\"")
        .entry("dead_diaeresis", "\"\\\"\"")
        .entry("dead_acute", "\"'\"")
        .entry("less", "\"<\"")
        .entry("greater", "\">\"")
        .entry("comma", "\",\"")
        .entry("colon", "\":\"")
        .entry("period", "\".\"")
        .entry("BackSpace", "\"BS\"")
        .entry("space", "\"space\"")
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
