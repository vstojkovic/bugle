use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use anyhow::{bail, Result};
use walkdir::WalkDir;
use zip::ZipWriter;

fn compress_l10n() -> Result<()> {
    let mut out_path = match std::env::var_os("OUT_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => bail!("missing OUT_DIR environment variable"),
    };
    out_path.push("l10n.zip");

    let out_file = File::create(out_path)?;
    let mut zip = ZipWriter::new(out_file);
    for entry in WalkDir::new("l10n") {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        let ftl_source = std::fs::read(entry.path())?;
        zip.start_file(
            path.strip_prefix("l10n").unwrap().to_string_lossy(),
            Default::default(),
        )?;
        zip.write_all(&ftl_source[..])?;
    }

    Ok(())
}

fn main() -> Result<()> {
    if let Ok(prerelease) = env::var("CARGO_PKG_VERSION_PRE") {
        if !prerelease.is_empty() {
            println!("cargo:rustc-cfg=default_log_debug");
        }
    }
    compress_l10n()?;
    println!("cargo:rerun-if-changed=l10n");
    Ok(())
}
