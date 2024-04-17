use std::rc::Rc;

use chrono::TimeDelta;
use fltk::button::CheckButton;
use fltk::prelude::*;
use fltk_float::grid::Grid;
use fltk_float::scroll::Scrollable;
use fltk_float::LayoutElement;

use crate::game::settings::server::{BuildingSettings, CreativeMode};
use crate::gui::widgets::DropDownList;
use crate::gui::wrapper_factory;

use super::{min_input_width, EditorBuilder, SliderInput};

pub struct BuildingTab {
    root: Scrollable,
    creative_mode_prop: DropDownList,
    build_anywhere_prop: CheckButton,
    stability_loss_mult_prop: SliderInput,
    build_during_pvp_disabled_prop: CheckButton,
    abandonment_disabled_prop: CheckButton,
    decay_time_mult_prop: SliderInput,
    thrall_decay_disabled_prop: CheckButton,
    thrall_decay_time_prop: SliderInput,
}

impl BuildingTab {
    pub fn new(settings: BuildingSettings) -> Rc<Self> {
        let input_width = min_input_width(&["99.9"]);

        let root = Scrollable::builder().with_gap(10, 10);

        let mut grid = Grid::builder_with_factory(wrapper_factory())
            .with_row_spacing(5)
            .with_col_spacing(10);

        grid.col().add();
        grid.col().with_stretch(1).add();
        grid.col().with_min_size(input_width).add();

        let creative_mode_prop = grid.enum_prop(
            "Creative mode server:",
            &["Admins Only", "Everyone", "Force for Everyone"],
            settings.creative_mode as u8,
        );
        let build_anywhere_prop =
            grid.bool_prop("Allow building anywhere", settings.build_anywhere);
        let stability_loss_mult_prop = grid.range_prop(
            "Stability loss multiplier:",
            0.0,
            5.0,
            1.0,
            100,
            settings.stability_loss_mult,
        );
        let build_during_pvp_disabled_prop = grid.bool_prop(
            "Disable building during time-restricted PVP",
            settings.build_during_pvp_disabled,
        );
        let abandonment_disabled_prop = grid.bool_prop(
            "Disable building abandonment",
            settings.abandonment_disabled,
        );
        let decay_time_mult_prop = grid.range_prop(
            "Building decay time multiplier:",
            0.1,
            10.0,
            1.0,
            10,
            settings.decay_time_mult,
        );
        let thrall_decay_disabled_prop =
            grid.bool_prop("Disable thrall decay", settings.thrall_decay_disabled);
        let thrall_decay_time_prop = grid.range_prop(
            "Thrall decay time (days):",
            1.0,
            30.0,
            1.0,
            1,
            settings.thrall_decay_time.num_days() as f64,
        );

        let root = root.add(grid.end());
        root.group().hide();

        Rc::new(Self {
            root,
            creative_mode_prop,
            build_anywhere_prop,
            stability_loss_mult_prop,
            build_during_pvp_disabled_prop,
            abandonment_disabled_prop,
            decay_time_mult_prop,
            thrall_decay_disabled_prop,
            thrall_decay_time_prop,
        })
    }

    pub fn root(&self) -> impl WidgetExt {
        self.root.group()
    }

    pub fn values(&self) -> BuildingSettings {
        BuildingSettings {
            creative_mode: CreativeMode::from_repr(self.creative_mode_prop.value() as u8).unwrap(),
            build_anywhere: self.build_anywhere_prop.is_checked(),
            stability_loss_mult: self.stability_loss_mult_prop.value().into(),
            build_during_pvp_disabled: self.build_during_pvp_disabled_prop.is_checked(),
            abandonment_disabled: self.abandonment_disabled_prop.is_checked(),
            decay_time_mult: self.decay_time_mult_prop.value().into(),
            thrall_decay_disabled: self.thrall_decay_disabled_prop.is_checked(),
            thrall_decay_time: TimeDelta::try_days(self.thrall_decay_time_prop.value() as i64)
                .unwrap(),
        }
    }
}

impl LayoutElement for BuildingTab {
    fn min_size(&self) -> fltk_float::Size {
        self.root.min_size()
    }

    fn layout(&self, x: i32, y: i32, width: i32, height: i32) {
        self.root.layout(x, y, width, height)
    }
}
