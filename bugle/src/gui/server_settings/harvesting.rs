use std::rc::Rc;

use fltk::prelude::*;
use fltk_float::grid::Grid;
use fltk_float::scroll::Scrollable;
use fltk_float::LayoutElement;

use crate::game::settings::server::{BaseHarvestingSettings, HarvestingSettings};
use crate::gui::wrapper_factory;

use super::{min_input_width, EditorBuilder, SliderInput};

pub struct HarvestingTab {
    root: Scrollable,
    item_spoil_rate_mult_prop: SliderInput,
    harvest_amount_mult_prop: SliderInput,
    rsrc_respawn_speed_mult_prop: SliderInput,
    claim_radius_mult_prop: SliderInput,
}

impl HarvestingTab {
    pub fn new(settings: HarvestingSettings) -> Rc<Self> {
        let input_width = min_input_width(&["99.9"]);

        let root = Scrollable::builder().with_gap(10, 10);

        let mut grid = Grid::builder_with_factory(wrapper_factory())
            .with_row_spacing(5)
            .with_col_spacing(10);

        grid.col().add();
        grid.col().with_stretch(1).add();
        grid.col().with_min_size(input_width).add();

        let item_spoil_rate_mult_prop = grid.range_prop(
            "Item spoil rate scale:",
            0.1,
            10.0,
            1.0,
            10,
            settings.item_spoil_rate_mult,
        );
        let harvest_amount_mult_prop = grid.range_prop(
            "Harvest amount multiplier:",
            0.1,
            10.0,
            1.0,
            10,
            settings.harvest_amount_mult,
        );
        let rsrc_respawn_speed_mult_prop = grid.range_prop(
            "Resource respawn speed multiplier:",
            0.1,
            10.0,
            1.0,
            10,
            settings.rsrc_respawn_speed_mult,
        );
        let claim_radius_mult_prop = grid.range_prop(
            "Land claim radius multiplier:",
            0.25,
            2.5,
            1.0,
            100,
            settings.claim_radius_mult,
        );

        let root = root.add(grid.end());
        root.group().hide();

        Rc::new(Self {
            root,
            item_spoil_rate_mult_prop,
            harvest_amount_mult_prop,
            rsrc_respawn_speed_mult_prop,
            claim_radius_mult_prop,
        })
    }

    pub fn root(&self) -> impl WidgetExt {
        self.root.group()
    }

    pub fn values(&self) -> HarvestingSettings {
        HarvestingSettings {
            base: BaseHarvestingSettings {
                harvest_amount_mult: self.harvest_amount_mult_prop.value().into(),
                item_spoil_rate_mult: self.item_spoil_rate_mult_prop.value().into(),
                rsrc_respawn_speed_mult: self.rsrc_respawn_speed_mult_prop.value().into(),
            },
            claim_radius_mult: self.claim_radius_mult_prop.value().into(),
        }
    }
}

impl LayoutElement for HarvestingTab {
    fn min_size(&self) -> fltk_float::Size {
        self.root.min_size()
    }

    fn layout(&self, x: i32, y: i32, width: i32, height: i32) {
        self.root.layout(x, y, width, height)
    }
}
