use std::rc::Rc;

use fltk::prelude::*;
use fltk_float::grid::Grid;
use fltk_float::scroll::Scrollable;
use fltk_float::LayoutElement;

use crate::game::settings::server::{HarvestingSettings, PublicHarvestingSettings};
use crate::gui::server_settings::{min_input_width, EditorBuilder, PrivateBuilder, SliderInput};
use crate::gui::wrapper_factory;

use super::SettingsTab;

pub struct HarvestingTab {
    root: Scrollable,
    item_spoil_rate_mult_prop: SliderInput,
    harvest_amount_mult_prop: SliderInput,
    rsrc_respawn_speed_mult_prop: SliderInput,
    private_props: Option<PrivateProperties>,
}

struct PrivateProperties {
    claim_radius_mult_prop: SliderInput,
}

impl HarvestingTab {
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

        let item_spoil_rate_mult_prop =
            grid.public
                .range_prop("Item spoil rate scale:", 0.1, 10.0, 1.0, 10);
        let harvest_amount_mult_prop =
            grid.public
                .range_prop("Harvest amount multiplier:", 0.1, 10.0, 1.0, 10);
        let rsrc_respawn_speed_mult_prop =
            grid.public
                .range_prop("Resource respawn speed multiplier:", 0.1, 10.0, 1.0, 10);
        let claim_radius_mult_prop =
            grid.range_prop("Land claim radius multiplier:", 0.25, 2.5, 1.0, 100);

        let root = root.add(grid.into_inner().end());
        root.group().hide();

        let private_props = include_private.then(|| PrivateProperties {
            claim_radius_mult_prop: claim_radius_mult_prop.unwrap(),
        });

        Rc::new(Self {
            root,
            item_spoil_rate_mult_prop,
            harvest_amount_mult_prop,
            rsrc_respawn_speed_mult_prop,
            private_props,
        })
    }

    pub fn public_values(&self) -> PublicHarvestingSettings {
        PublicHarvestingSettings {
            harvest_amount_mult: self.harvest_amount_mult_prop.value().into(),
            item_spoil_rate_mult: self.item_spoil_rate_mult_prop.value().into(),
            rsrc_respawn_speed_mult: self.rsrc_respawn_speed_mult_prop.value().into(),
        }
    }

    pub fn values(&self) -> HarvestingSettings {
        self.private_props
            .as_ref()
            .unwrap()
            .values(self.public_values())
    }

    pub fn set_public_values(&self, settings: &PublicHarvestingSettings) {
        self.item_spoil_rate_mult_prop
            .set_value(settings.item_spoil_rate_mult);
        self.harvest_amount_mult_prop
            .set_value(settings.harvest_amount_mult);
        self.rsrc_respawn_speed_mult_prop
            .set_value(settings.rsrc_respawn_speed_mult);
    }

    pub fn set_values(&self, settings: &HarvestingSettings) {
        self.set_public_values(settings);
        self.private_props.as_ref().unwrap().set_values(settings);
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

impl SettingsTab for HarvestingTab {
    fn root(&self) -> impl WidgetExt + 'static {
        self.root.group()
    }
}

impl PrivateProperties {
    fn values(&self, public: PublicHarvestingSettings) -> HarvestingSettings {
        HarvestingSettings {
            public,
            claim_radius_mult: self.claim_radius_mult_prop.value().into(),
        }
    }

    fn set_values(&self, settings: &HarvestingSettings) {
        self.claim_radius_mult_prop
            .set_value(settings.claim_radius_mult);
    }
}
