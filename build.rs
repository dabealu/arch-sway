fn main() {
    // CARGO_PKG_VERSION set this way, will override the version taken from Cargo.toml
    if let Ok(val) = std::env::var("ARCHSWAY_RELEASE_VERSION") {
        println!("cargo:rustc-env=CARGO_PKG_VERSION={}", val);
    }
    // println!("cargo:rerun-if-env-changed=ARCHSWAY_RELEASE_VERSION");
}
