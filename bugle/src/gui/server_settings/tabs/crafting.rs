use std::rc::Rc;

use fltk::prelude::*;
use fltk_float::grid::Grid;
use fltk_float::scroll::Scrollable;
use fltk_float::LayoutElement;

use crate::game::settings::server::{CraftingSettings, PublicCraftingSettings};
use crate::gui::server_settings::{EditorBuilder, PrivateBuilder, SliderInput};
use crate::gui::{min_input_width, wrapper_factory};

use super::SettingsTab;

pub struct CraftingTab {
    root: Scrollable,
    crafting_time_mult_prop: SliderInput,
    thrall_crafting_time_mult_prop: SliderInput,
    private_props: Option<PrivateProperties>,
}

struct PrivateProperties {
    fuel_burn_time_mult_prop: SliderInput,
    crafting_cost_mult_prop: SliderInput,
}

impl CraftingTab {
    pub fn new(include_private: bool) -> Rc<Self> {
        let input_width = min_input_width(&["99.99"]);

        let root = Scrollable::builder().with_gap(10, 10);

        let mut grid = Grid::builder_with_factory(wrapper_factory())
            .with_row_spacing(5)
            .with_col_spacing(10);

        grid.col().add();
        grid.col().with_stretch(1).add();
        grid.col().with_min_size(input_width).add();

        let mut grid = PrivateBuilder::new(grid, include_private);

        let crafting_time_mult_prop =
            grid.public
                .range_prop("Crafting time multiplier:", 0.1, 10.0, 1.0, 100);
        let thrall_crafting_time_mult_prop =
            grid.public
                .range_prop("Thrall crafting time multiplier:", 0.1, 10.0, 1.0, 100);
        let fuel_burn_time_mult_prop =
            grid.range_prop("Fuel burn time multiplier:", 0.1, 10.0, 1.0, 100);
        let crafting_cost_mult_prop =
            grid.range_prop("Crafting cost multiplier:", 0.1, 10.0, 1.0, 100);

        let root = root.add(grid.into_inner().end());
        root.group().hide();

        let private_props = include_private.then(|| PrivateProperties {
            fuel_burn_time_mult_prop: fuel_burn_time_mult_prop.unwrap(),
            crafting_cost_mult_prop: crafting_cost_mult_prop.unwrap(),
        });

        Rc::new(Self {
            root,
            crafting_time_mult_prop,
            thrall_crafting_time_mult_prop,
            private_props,
        })
    }

    pub fn public_values(&self) -> PublicCraftingSettings {
        PublicCraftingSettings {
            crafting_time_mult: self.crafting_time_mult_prop.value().into(),
            thrall_crafting_time_mult: self.thrall_crafting_time_mult_prop.value().into(),
        }
    }

    pub fn values(&self) -> CraftingSettings {
        self.private_props
            .as_ref()
            .unwrap()
            .values(self.public_values())
    }

    pub fn set_public_values(&self, settings: &CraftingSettings) {
        self.crafting_time_mult_prop
            .set_value(settings.crafting_time_mult);
        self.thrall_crafting_time_mult_prop
            .set_value(settings.thrall_crafting_time_mult);
    }

    pub fn set_values(&self, settings: &CraftingSettings) {
        self.set_public_values(settings);
        self.private_props.as_ref().unwrap().set_values(settings);
    }
}

impl LayoutElement for CraftingTab {
    fn min_size(&self) -> fltk_float::Size {
        self.root.min_size()
    }

    fn layout(&self, x: i32, y: i32, width: i32, height: i32) {
        self.root.layout(x, y, width, height)
    }
}

impl SettingsTab for CraftingTab {
    fn root(&self) -> impl WidgetExt + 'static {
        self.root.group()
    }
}

impl PrivateProperties {
    pub fn values(&self, public: PublicCraftingSettings) -> CraftingSettings {
        CraftingSettings {
            public,
            fuel_burn_time_mult: self.fuel_burn_time_mult_prop.value().into(),
            crafting_cost_mult: self.crafting_cost_mult_prop.value().into(),
        }
    }

    fn set_values(&self, settings: &CraftingSettings) {
        self.fuel_burn_time_mult_prop
            .set_value(settings.fuel_burn_time_mult);
        self.crafting_cost_mult_prop
            .set_value(settings.crafting_cost_mult);
    }
}
