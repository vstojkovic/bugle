use std::rc::Rc;

use fltk::button::CheckButton;
use fltk::prelude::*;
use fltk_float::grid::Grid;
use fltk_float::scroll::Scrollable;
use fltk_float::LayoutElement;

use crate::game::settings::server::{DaylightSettings, PublicDaylightSettings};
use crate::gui::server_settings::{EditorBuilder, PrivateBuilder, SliderInput};
use crate::gui::{min_input_width, wrapper_factory};

use super::SettingsTab;

pub struct DaylightTab {
    root: Scrollable,
    day_cycle_speed_mult_prop: SliderInput,
    dawn_dusk_speed_mult_prop: SliderInput,
    use_catch_up_time_prop: CheckButton,
    private_props: Option<PrivateProperties>,
}

struct PrivateProperties {
    day_time_speed_mult_prop: SliderInput,
    night_time_speed_mult_prop: SliderInput,
    catch_up_time_prop: SliderInput,
}

impl DaylightTab {
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

        let day_cycle_speed_mult_prop =
            grid.public
                .range_prop("Day cycle speed:", 0.1, 10.0, 1.0, 10);
        let day_time_speed_mult_prop = grid.range_prop("Day time speed:", 0.1, 10.0, 1.0, 10);
        let night_time_speed_mult_prop = grid.range_prop("Night time speed:", 0.1, 10.0, 1.0, 10);
        let dawn_dusk_speed_mult_prop =
            grid.public
                .range_prop("Dawn/dusk time speed:", 0.1, 10.0, 1.0, 10);
        let use_catch_up_time_prop = grid.public.bool_prop("Use catch up time");
        let catch_up_time_prop = grid.range_prop("Catch up time:", 1.0, 24.0, 1.0, 1);

        let root = root.add(grid.into_inner().end());
        root.group().hide();

        let private_props = include_private.then(|| PrivateProperties {
            day_time_speed_mult_prop: day_time_speed_mult_prop.unwrap(),
            night_time_speed_mult_prop: night_time_speed_mult_prop.unwrap(),
            catch_up_time_prop: catch_up_time_prop.unwrap(),
        });

        Rc::new(Self {
            root,
            day_cycle_speed_mult_prop,
            dawn_dusk_speed_mult_prop,
            use_catch_up_time_prop,
            private_props,
        })
    }

    pub fn public_values(&self) -> PublicDaylightSettings {
        PublicDaylightSettings {
            day_cycle_speed_mult: self.day_cycle_speed_mult_prop.value().into(),
            dawn_dusk_speed_mult: self.dawn_dusk_speed_mult_prop.value().into(),
            use_catch_up_time: self.use_catch_up_time_prop.is_checked(),
        }
    }

    pub fn values(&self) -> DaylightSettings {
        self.private_props
            .as_ref()
            .unwrap()
            .values(self.public_values())
    }

    pub fn set_public_values(&self, settings: &PublicDaylightSettings) {
        self.day_cycle_speed_mult_prop
            .set_value(settings.day_cycle_speed_mult);
        self.dawn_dusk_speed_mult_prop
            .set_value(settings.dawn_dusk_speed_mult);
        self.use_catch_up_time_prop
            .set_checked(settings.use_catch_up_time);
    }

    pub fn set_values(&self, settings: &DaylightSettings) {
        self.set_public_values(settings);
        self.private_props.as_ref().unwrap().set_values(settings);
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

impl SettingsTab for DaylightTab {
    fn root(&self) -> impl WidgetExt + 'static {
        self.root.group()
    }
}

impl PrivateProperties {
    fn values(&self, public: PublicDaylightSettings) -> DaylightSettings {
        DaylightSettings {
            public,
            day_time_speed_mult: self.day_time_speed_mult_prop.value().into(),
            night_time_speed_mult: self.night_time_speed_mult_prop.value().into(),
            catch_up_time: self.catch_up_time_prop.value(),
        }
    }

    fn set_values(&self, settings: &DaylightSettings) {
        self.day_time_speed_mult_prop
            .set_value(settings.day_time_speed_mult);
        self.night_time_speed_mult_prop
            .set_value(settings.night_time_speed_mult);
        self.catch_up_time_prop.set_value(settings.catch_up_time);
    }
}
