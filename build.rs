fn main() {
    if cfg!(target_os = "windows") {
        println!("cargo:rustc-link-search=native=C:\\msys64\\mingw64\\lib");
    }
}
