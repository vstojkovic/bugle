use std::rc::Rc;

use fltk::prelude::*;
use fltk_float::grid::Grid;
use fltk_float::scroll::Scrollable;
use fltk_float::LayoutElement;

use crate::game::settings::server::{BaseCraftingSettings, CraftingSettings};
use crate::gui::wrapper_factory;

use super::{min_input_width, EditorBuilder, SliderInput};

pub struct CraftingTab {
    root: Scrollable,
    crafting_time_mult_prop: SliderInput,
    thrall_crafting_time_mult_prop: SliderInput,
    fuel_burn_time_mult_prop: SliderInput,
    crafting_cost_mult_prop: SliderInput,
}

impl CraftingTab {
    pub fn new(settings: CraftingSettings) -> Rc<Self> {
        let input_width = min_input_width(&["99.99"]);

        let root = Scrollable::builder().with_gap(10, 10);

        let mut grid = Grid::builder_with_factory(wrapper_factory())
            .with_row_spacing(5)
            .with_col_spacing(10);

        grid.col().add();
        grid.col().with_stretch(1).add();
        grid.col().with_min_size(input_width).add();

        let crafting_time_mult_prop = grid.range_prop(
            "Crafting time multiplier:",
            0.1,
            10.0,
            1.0,
            100,
            settings.crafting_time_mult,
        );
        let thrall_crafting_time_mult_prop = grid.range_prop(
            "Thrall crafting time multiplier:",
            0.1,
            10.0,
            1.0,
            100,
            settings.thrall_crafting_time_mult,
        );
        let fuel_burn_time_mult_prop = grid.range_prop(
            "Fuel burn time multiplier:",
            0.1,
            10.0,
            1.0,
            100,
            settings.fuel_burn_time_mult,
        );
        let crafting_cost_mult_prop = grid.range_prop(
            "Crafting cost multiplier:",
            0.1,
            10.0,
            1.0,
            100,
            settings.crafting_cost_mult,
        );

        let root = root.add(grid.end());
        root.group().hide();

        Rc::new(Self {
            root,
            crafting_time_mult_prop,
            thrall_crafting_time_mult_prop,
            fuel_burn_time_mult_prop,
            crafting_cost_mult_prop,
        })
    }

    pub fn root(&self) -> impl WidgetExt {
        self.root.group()
    }

    pub fn values(&self) -> CraftingSettings {
        CraftingSettings {
            base: BaseCraftingSettings {
                crafting_time_mult: self.crafting_time_mult_prop.value().into(),
                thrall_crafting_time_mult: self.thrall_crafting_time_mult_prop.value().into(),
            },
            fuel_burn_time_mult: self.fuel_burn_time_mult_prop.value().into(),
            crafting_cost_mult: self.crafting_cost_mult_prop.value().into(),
        }
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
