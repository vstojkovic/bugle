use std::env;

fn main() {
    if let Ok(prerelease) = env::var("CARGO_PKG_VERSION_PRE") {
        if !prerelease.is_empty() {
            println!("cargo:rustc-cfg=default_log_debug");
        }
    }
}
