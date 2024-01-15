use std::fs::File;
use std::io::Write;

use anyhow::Result;
use fltk::enums::Font;
use fltk::image::PngImage;
use fltk::prelude::ImageExt;
use tempfile::tempdir;

pub(super) struct Assets;

impl Assets {
    pub fn crom_font() -> Result<Font> {
        let dir = tempdir()?;
        let path = dir.path().join("Crom_v1.ttf");

        let mut file = File::create(&path)?;
        file.write_all(CROM_TTF)?;
        drop(file);

        let font = Font::load_font(path)?;
        Font::set_font(Font::Zapfdingbats, &font);
        Ok(Font::Zapfdingbats)
    }

    pub fn mod_provenance_icons() -> impl ImageExt {
        PngImage::from_data(MOD_PROVENANCE_ICONS).unwrap()
    }
}

const CROM_TTF: &[u8] = include_bytes!("assets/Crom_v1.ttf");
const MOD_PROVENANCE_ICONS: &[u8] = include_bytes!("assets/mod-provenance-icons.png");
