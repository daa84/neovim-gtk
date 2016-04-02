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
        .entry("Enter", "\"CR\"")
        .entry("Return", "\"CR\"")
        .build(&mut file)
        .unwrap();
    write!(&mut file, ";\n").unwrap();
}
