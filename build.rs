use std::env;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Result};
use resvg::tiny_skia::Pixmap;
use resvg::usvg::fontdb::Database;
use resvg::usvg::{PostProcessingSteps, Tree, TreeParsing, TreePostProc};

fn svg_to_png(svg_path: PathBuf) -> Result<()> {
    let mut png_path = match std::env::var_os("OUT_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => bail!("missing OUT_DIR environment variable"),
    };
    png_path.push("assets");
    std::fs::create_dir_all(&png_path)?;
    png_path.push(svg_path.file_name().unwrap());
    png_path.set_extension("png");

    let data = std::fs::read(svg_path)?;
    let mut tree = Tree::from_data(&data, &Default::default())?;
    tree.postprocess(PostProcessingSteps::default(), &Database::new());

    let size = tree.size.to_int_size();
    let mut image = Pixmap::new(size.width(), size.height())
        .ok_or_else(|| anyhow!("Failed to allocate pixmap"))?;
    resvg::render(&tree, Default::default(), &mut image.as_mut());

    image.save_png(png_path)?;

    Ok(())
}

fn main() -> Result<()> {
    if let Ok(prerelease) = env::var("CARGO_PKG_VERSION_PRE") {
        if !prerelease.is_empty() {
            println!("cargo:rustc-cfg=default_log_debug");
        }
    }

    svg_to_png(Path::new("assets").join("bugle-logo.svg"))?;

    Ok(())
}
