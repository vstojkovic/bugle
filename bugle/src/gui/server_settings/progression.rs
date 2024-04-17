use std::rc::Rc;

use fltk::prelude::*;
use fltk_float::grid::Grid;
use fltk_float::scroll::Scrollable;
use fltk_float::LayoutElement;

use crate::game::settings::server::{BaseProgressionSettings, ProgressionSettings};
use crate::gui::wrapper_factory;

use super::{min_input_width, EditorBuilder, SliderInput};

pub struct ProgressionTab {
    root: Scrollable,
    xp_rate_mult_prop: SliderInput,
    xp_time_mult_prop: SliderInput,
    xp_kill_mult_prop: SliderInput,
    xp_harvest_mult_prop: SliderInput,
    xp_craft_mult_prop: SliderInput,
}

impl ProgressionTab {
    pub fn new(settings: ProgressionSettings) -> Rc<Self> {
        let input_width = min_input_width(&["99.9"]);

        let root = Scrollable::builder().with_gap(10, 10);

        let mut grid = Grid::builder_with_factory(wrapper_factory())
            .with_row_spacing(5)
            .with_col_spacing(10);

        grid.col().add();
        grid.col().with_stretch(1).add();
        grid.col().with_min_size(input_width).add();

        let xp_rate_mult_prop = grid.range_prop(
            "Player XP rate multiplier:",
            0.1,
            10.0,
            1.0,
            10,
            settings.xp_rate_mult,
        );
        let xp_time_mult_prop = grid.range_prop(
            "Player XP time multiplier:",
            0.0,
            10.0,
            1.0,
            10,
            settings.xp_time_mult,
        );
        let xp_kill_mult_prop = grid.range_prop(
            "Player XP kill multiplier:",
            0.1,
            10.0,
            1.0,
            10,
            settings.xp_kill_mult,
        );
        let xp_harvest_mult_prop = grid.range_prop(
            "Player XP harvest multiplier:",
            0.1,
            10.0,
            1.0,
            10,
            settings.xp_harvest_mult,
        );
        let xp_craft_mult_prop = grid.range_prop(
            "Player XP craft multiplier:",
            0.1,
            10.0,
            1.0,
            10,
            settings.xp_craft_mult,
        );

        let root = root.add(grid.end());
        root.group().hide();

        Rc::new(Self {
            root,
            xp_rate_mult_prop,
            xp_time_mult_prop,
            xp_kill_mult_prop,
            xp_harvest_mult_prop,
            xp_craft_mult_prop,
        })
    }

    pub fn root(&self) -> impl WidgetExt {
        self.root.group()
    }

    pub fn values(&self) -> ProgressionSettings {
        ProgressionSettings {
            base: BaseProgressionSettings {
                xp_rate_mult: self.xp_rate_mult_prop.value().into(),
            },
            xp_time_mult: self.xp_time_mult_prop.value().into(),
            xp_kill_mult: self.xp_kill_mult_prop.value().into(),
            xp_harvest_mult: self.xp_harvest_mult_prop.value().into(),
            xp_craft_mult: self.xp_craft_mult_prop.value().into(),
        }
    }
}

impl LayoutElement for ProgressionTab {
    fn min_size(&self) -> fltk_float::Size {
        self.root.min_size()
    }

    fn layout(&self, x: i32, y: i32, width: i32, height: i32) {
        self.root.layout(x, y, width, height)
    }
}
