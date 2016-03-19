
fn main() {
    if cfg!(target = "windows") {
        println!("cargo:rustc-link-search=native=C:\\msys64\\mingw64\\lib");
    }
}
