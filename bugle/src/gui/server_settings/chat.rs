use std::rc::Rc;

use fltk::button::CheckButton;
use fltk::prelude::*;
use fltk_float::grid::Grid;
use fltk_float::scroll::Scrollable;
use fltk_float::LayoutElement;
use num::ToPrimitive;

use crate::game::settings::server::ChatSettings;
use crate::gui::wrapper_factory;

use super::{min_input_width, EditorBuilder, SliderInput};

pub struct ChatTab {
    root: Scrollable,
    local_radius_prop: SliderInput,
    max_msg_len_prop: SliderInput,
    global_enabled_prop: CheckButton,
}

impl ChatTab {
    pub fn new(settings: ChatSettings) -> Rc<Self> {
        let input_width = min_input_width(&["99999.9"]);

        let root = Scrollable::builder().with_gap(10, 10);

        let mut grid = Grid::builder_with_factory(wrapper_factory())
            .with_row_spacing(5)
            .with_col_spacing(10);

        grid.col().add();
        grid.col().with_stretch(1).add();
        grid.col().with_min_size(input_width).add();

        let local_radius_prop = grid.range_prop(
            "Chat local radius:",
            0.0,
            20000.0,
            1.0,
            10,
            settings.local_radius,
        );
        let max_msg_len_prop = grid.range_prop(
            "Max message length:",
            0.0,
            1024.0,
            1.0,
            1,
            settings.max_msg_len as f64,
        );
        let global_enabled_prop = grid.bool_prop("Chat has global", settings.global_enabled);

        let root = root.add(grid.end());
        root.group().hide();

        Rc::new(Self {
            root,
            local_radius_prop,
            max_msg_len_prop,
            global_enabled_prop,
        })
    }

    pub fn root(&self) -> impl WidgetExt {
        self.root.group()
    }

    pub fn values(&self) -> ChatSettings {
        ChatSettings {
            local_radius: self.local_radius_prop.value(),
            max_msg_len: self.max_msg_len_prop.value().to_u16().unwrap(),
            global_enabled: self.global_enabled_prop.is_checked(),
        }
    }
}

impl LayoutElement for ChatTab {
    fn min_size(&self) -> fltk_float::Size {
        self.root.min_size()
    }

    fn layout(&self, x: i32, y: i32, width: i32, height: i32) {
        self.root.layout(x, y, width, height)
    }
}