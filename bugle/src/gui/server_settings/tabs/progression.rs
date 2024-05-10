use std::rc::Rc;

use fltk::prelude::*;
use fltk_float::grid::Grid;
use fltk_float::scroll::Scrollable;
use fltk_float::LayoutElement;

use crate::game::settings::server::{ProgressionSettings, PublicProgressionSettings};
use crate::gui::server_settings::{min_input_width, EditorBuilder, PrivateBuilder, SliderInput};
use crate::gui::wrapper_factory;

use super::SettingsTab;

pub struct ProgressionTab {
    root: Scrollable,
    xp_rate_mult_prop: SliderInput,
    private_props: Option<PrivateProperties>,
}

struct PrivateProperties {
    xp_time_mult_prop: SliderInput,
    xp_kill_mult_prop: SliderInput,
    xp_harvest_mult_prop: SliderInput,
    xp_craft_mult_prop: SliderInput,
}

impl ProgressionTab {
    pub fn new(include_private: bool) -> Rc<Self> {
        let input_width = min_input_width(&["99.9"]);

        let root = Scrollable::builder().with_gap(10, 10);

        let mut grid = Grid::builder_with_factory(wrapper_factory())
            .with_row_spacing(5)
            .with_col_spacing(10);

        grid.col().add();
        grid.col().with_stretch(1).add();
        grid.col().with_min_size(input_width).add();

        let mut grid = PrivateBuilder::new(grid, include_private);

        let xp_rate_mult_prop =
            grid.public
                .range_prop("Player XP rate multiplier:", 0.1, 10.0, 1.0, 10);
        let xp_time_mult_prop = grid.range_prop("Player XP time multiplier:", 0.0, 10.0, 1.0, 10);
        let xp_kill_mult_prop = grid.range_prop("Player XP kill multiplier:", 0.1, 10.0, 1.0, 10);
        let xp_harvest_mult_prop =
            grid.range_prop("Player XP harvest multiplier:", 0.1, 10.0, 1.0, 10);
        let xp_craft_mult_prop = grid.range_prop("Player XP craft multiplier:", 0.1, 10.0, 1.0, 10);

        let root = root.add(grid.into_inner().end());
        root.group().hide();

        let private_props = include_private.then(|| PrivateProperties {
            xp_time_mult_prop: xp_time_mult_prop.unwrap(),
            xp_kill_mult_prop: xp_kill_mult_prop.unwrap(),
            xp_harvest_mult_prop: xp_harvest_mult_prop.unwrap(),
            xp_craft_mult_prop: xp_craft_mult_prop.unwrap(),
        });

        Rc::new(Self {
            root,
            xp_rate_mult_prop,
            private_props,
        })
    }

    pub fn public_values(&self) -> PublicProgressionSettings {
        PublicProgressionSettings {
            xp_rate_mult: self.xp_rate_mult_prop.value().into(),
        }
    }

    pub fn values(&self) -> ProgressionSettings {
        self.private_props
            .as_ref()
            .unwrap()
            .values(self.public_values())
    }

    pub fn set_public_values(&self, settings: &PublicProgressionSettings) {
        self.xp_rate_mult_prop.set_value(settings.xp_rate_mult);
    }

    pub fn set_values(&self, settings: &ProgressionSettings) {
        self.set_public_values(settings);
        self.private_props.as_ref().unwrap().set_values(settings);
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

impl SettingsTab for ProgressionTab {
    fn root(&self) -> impl WidgetExt + 'static {
        self.root.group()
    }
}

impl PrivateProperties {
    fn values(&self, public: PublicProgressionSettings) -> ProgressionSettings {
        ProgressionSettings {
            public,
            xp_time_mult: self.xp_time_mult_prop.value().into(),
            xp_kill_mult: self.xp_kill_mult_prop.value().into(),
            xp_harvest_mult: self.xp_harvest_mult_prop.value().into(),
            xp_craft_mult: self.xp_craft_mult_prop.value().into(),
        }
    }

    fn set_values(&self, settings: &ProgressionSettings) {
        self.xp_time_mult_prop.set_value(settings.xp_time_mult);
        self.xp_kill_mult_prop.set_value(settings.xp_kill_mult);
        self.xp_harvest_mult_prop
            .set_value(settings.xp_harvest_mult);
        self.xp_craft_mult_prop.set_value(settings.xp_craft_mult);
    }
}
