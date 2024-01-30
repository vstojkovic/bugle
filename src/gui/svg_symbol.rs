use std::collections::HashMap;

use fltk::enums::Color;
use fltk::image::SvgImage;
use fltk::prelude::ImageExt;
use lazy_static::lazy_static;
use regex::Regex;

use super::color_rgb;

pub struct SvgSymbol {
    svg: String,
    use_caller_color: bool,
    images: HashMap<ImageKey, SvgImage>,
}

type ImageKey = (Option<u32>, i32, i32);

impl SvgSymbol {
    pub fn new(svg: String) -> Self {
        let use_caller_color = PLACEHOLDER_REGEX.is_match(&svg);
        Self {
            svg,
            use_caller_color,
            images: HashMap::new(),
        }
    }

    pub fn draw(&mut self, color: Color) {
        let x = fltk::draw::transform_x(-1.0, -1.0) as i32;
        let y = fltk::draw::transform_y(-1.0, -1.0) as i32;
        let w = (fltk::draw::transform_x(1.0, 1.0) as i32) - x + 1;
        let h = (fltk::draw::transform_y(1.0, 1.0) as i32) - y + 1;
        self.image_for(color, w, h).draw(x, y, w, h);
    }

    fn image_for(&mut self, color: Color, width: i32, height: i32) -> &mut SvgImage {
        let rgb = color_rgb(color);
        let key = (
            if self.use_caller_color { Some(rgb) } else { None },
            width,
            height,
        );
        self.images.entry(key).or_insert_with(|| {
            let data = if self.use_caller_color {
                PLACEHOLDER_REGEX.replace_all(&self.svg, format!("#{:06x}", rgb))
            } else {
                (&self.svg).into()
            };
            let mut image = SvgImage::from_data(data.as_ref()).unwrap();
            image.scale(width, height, false, true);
            image
        })
    }
}

lazy_static! {
    static ref PLACEHOLDER_REGEX: Regex = Regex::new("currentColor").unwrap();
}

macro_rules! draw_svg_symbol {
    ($svg:expr) => {
        |color| {
            use std::cell::RefCell;
            use $crate::gui::svg_symbol::SvgSymbol;

            thread_local! {
                static SYM: RefCell<SvgSymbol> = RefCell::new(SvgSymbol::new($svg.to_string()));
            }

            SYM.with_borrow_mut(|sym| sym.draw(color));
        }
    };
}
pub(crate) use draw_svg_symbol;
