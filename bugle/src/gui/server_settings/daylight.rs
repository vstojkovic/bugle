use std::rc::Rc;

use fltk::button::CheckButton;
use fltk::prelude::*;
use fltk_float::grid::Grid;
use fltk_float::scroll::Scrollable;
use fltk_float::LayoutElement;

use crate::game::settings::server::{BaseDaylightSettings, DaylightSettings};
use crate::gui::wrapper_factory;

use super::{min_input_width, EditorBuilder, SliderInput};

pub struct DaylightTab {
    root: Scrollable,
    day_cycle_speed_mult_prop: SliderInput,
    day_time_speed_mult_prop: SliderInput,
    night_time_speed_mult_prop: SliderInput,
    dawn_dusk_speed_mult_prop: SliderInput,
    use_catch_up_time_prop: CheckButton,
    catch_up_time_prop: SliderInput,
}

impl DaylightTab {
    pub fn new(settings: DaylightSettings) -> Rc<Self> {
        let input_width = min_input_width(&["99.9"]);

        let root = Scrollable::builder().with_gap(10, 10);

        let mut grid = Grid::builder_with_factory(wrapper_factory())
            .with_row_spacing(5)
            .with_col_spacing(10);

        grid.col().add();
        grid.col().with_stretch(1).add();
        grid.col().with_min_size(input_width).add();

        let day_cycle_speed_mult_prop = grid.range_prop(
            "Day cycle speed:",
            0.1,
            10.0,
            1.0,
            10,
            settings.day_cycle_speed_mult,
        );
        let day_time_speed_mult_prop = grid.range_prop(
            "Day time speed:",
            0.1,
            10.0,
            1.0,
            10,
            settings.day_time_speed_mult,
        );
        let night_time_speed_mult_prop = grid.range_prop(
            "Night time speed:",
            0.1,
            10.0,
            1.0,
            10,
            settings.night_time_speed_mult,
        );
        let dawn_dusk_speed_mult_prop = grid.range_prop(
            "Dawn/dusk time speed:",
            0.1,
            10.0,
            1.0,
            10,
            settings.dawn_dusk_speed_mult,
        );
        let use_catch_up_time_prop =
            grid.bool_prop("Use catch up time", settings.use_catch_up_time);
        let catch_up_time_prop =
            grid.range_prop("Catch up time:", 1.0, 24.0, 1.0, 1, settings.catch_up_time);

        let root = root.add(grid.end());
        root.group().hide();

        Rc::new(Self {
            root,
            day_cycle_speed_mult_prop,
            day_time_speed_mult_prop,
            night_time_speed_mult_prop,
            dawn_dusk_speed_mult_prop,
            use_catch_up_time_prop,
            catch_up_time_prop,
        })
    }

    pub fn root(&self) -> impl WidgetExt {
        self.root.group()
    }

    pub fn values(&self) -> DaylightSettings {
        DaylightSettings {
            base: BaseDaylightSettings {
                day_cycle_speed_mult: self.day_cycle_speed_mult_prop.value().into(),
                dawn_dusk_speed_mult: self.dawn_dusk_speed_mult_prop.value().into(),
                use_catch_up_time: self.use_catch_up_time_prop.is_checked(),
            },
            day_time_speed_mult: self.day_time_speed_mult_prop.value().into(),
            night_time_speed_mult: self.night_time_speed_mult_prop.value().into(),
            catch_up_time: self.catch_up_time_prop.value(),
        }
    }
}

impl LayoutElement for DaylightTab {
    fn min_size(&self) -> fltk_float::Size {
        self.root.min_size()
    }

    fn layout(&self, x: i32, y: i32, width: i32, height: i32) {
        self.root.layout(x, y, width, height)
    }
}
