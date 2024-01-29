use fltk::image::PngImage;
use fltk::prelude::ImageExt;

pub(super) struct Assets;

impl Assets {
    pub fn bugle_logo() -> impl ImageExt {
        PngImage::from_data(BUGLE_PNG).unwrap()
    }
}

const BUGLE_PNG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/bugle-logo.png"));
