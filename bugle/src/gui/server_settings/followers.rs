use std::rc::Rc;

use chrono::TimeDelta;
use fltk::button::CheckButton;
use fltk::prelude::*;
use fltk_float::grid::Grid;
use fltk_float::scroll::Scrollable;
use fltk_float::LayoutElement;
use num::ToPrimitive;

use crate::game::settings::server::FollowerSettings;
use crate::gui::wrapper_factory;

use super::{min_input_width, EditorBuilder, SliderInput};

pub struct FollowersTab {
    root: Scrollable,
    pen_crafting_time_mult_prop: SliderInput,
    feeder_rang_mult_prop: SliderInput,
    cap_enabled_prop: CheckButton,
    cap_base_prop: SliderInput,
    cap_per_player_prop: SliderInput,
    cleanup_interval_prop: SliderInput,
}

impl FollowersTab {
    pub fn new() -> Rc<Self> {
        let input_width = min_input_width(&["99.999"]);

        let root = Scrollable::builder().with_gap(10, 10);

        let mut grid = Grid::builder_with_factory(wrapper_factory())
            .with_row_spacing(5)
            .with_col_spacing(10);

        grid.col().add();
        grid.col().with_stretch(1).add();
        grid.col().with_min_size(input_width).add();

        let pen_crafting_time_mult_prop = grid.range_prop(
            "Animal pen crafting time multiplier:",
            0.001,
            10.0,
            1.0,
            1000,
        );
        let feeder_rang_mult_prop =
            grid.range_prop("Food container range multiplier:", 0.1, 4.0, 1.0, 10);
        let cap_enabled_prop = grid.bool_prop("Use follower population limit");
        let cap_base_prop = grid.range_prop("Follower population base value:", 0.0, 150.0, 1.0, 1);
        let cap_per_player_prop =
            grid.range_prop("Follower population per player:", 0.0, 150.0, 1.0, 1);
        let cleanup_interval_prop =
            grid.range_prop("Overpopulation cleanup interval:", 5.0, 720.0, 1.0, 1);

        let root = root.add(grid.end());
        root.group().hide();

        Rc::new(Self {
            root,
            pen_crafting_time_mult_prop,
            feeder_rang_mult_prop,
            cap_enabled_prop,
            cap_base_prop,
            cap_per_player_prop,
            cleanup_interval_prop,
        })
    }

    pub fn root(&self) -> impl WidgetExt {
        self.root.group()
    }

    pub fn values(&self) -> FollowerSettings {
        FollowerSettings {
            pen_crafting_time_mult: self.pen_crafting_time_mult_prop.value().into(),
            feeder_rang_mult: self.feeder_rang_mult_prop.value().into(),
            cap_enabled: self.cap_enabled_prop.is_checked(),
            cap_base: self.cap_base_prop.value().to_u8().unwrap(),
            cap_per_player: self.cap_per_player_prop.value().to_u8().unwrap(),
            cleanup_interval: TimeDelta::try_minutes(self.cleanup_interval_prop.value() as i64)
                .unwrap(),
        }
    }

    pub fn set_values(&self, settings: &FollowerSettings) {
        self.pen_crafting_time_mult_prop
            .set_value(settings.pen_crafting_time_mult);
        self.feeder_rang_mult_prop
            .set_value(settings.feeder_rang_mult);
        self.cap_enabled_prop.set_checked(settings.cap_enabled);
        self.cap_base_prop.set_value(settings.cap_base);
        self.cap_per_player_prop.set_value(settings.cap_per_player);
        self.cleanup_interval_prop
            .set_value(settings.cleanup_interval.num_minutes() as f64);
    }
}

impl LayoutElement for FollowersTab {
    fn min_size(&self) -> fltk_float::Size {
        self.root.min_size()
    }

    fn layout(&self, x: i32, y: i32, width: i32, height: i32) {
        self.root.layout(x, y, width, height)
    }
}
