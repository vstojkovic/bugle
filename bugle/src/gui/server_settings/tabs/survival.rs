use std::rc::Rc;

use fltk::button::CheckButton;
use fltk::prelude::*;
use fltk_float::grid::Grid;
use fltk_float::scroll::Scrollable;
use fltk_float::LayoutElement;

use crate::game::settings::server::{DropOnDeath, PublicSurvivalSettings, SurvivalSettings};
use crate::gui::server_settings::{EditorBuilder, PrivateBuilder, SliderInput};
use crate::gui::widgets::DropDownList;
use crate::gui::{min_input_width, wrapper_factory};

use super::SettingsTab;

pub struct SurvivalTab {
    root: Scrollable,
    offline_chars_in_world: bool,
    stamina_cost_mult_prop: SliderInput,
    active_thirst_mult_prop: SliderInput,
    active_hunger_mult_prop: SliderInput,
    idle_thirst_mult_prop: SliderInput,
    idle_hunger_mult_prop: SliderInput,
    drop_items_on_death_prop: DropDownList,
    anyone_can_loot_corpse_prop: CheckButton,
    private_props: Option<PrivateProperties>,
}

struct PrivateProperties {
    corruption_removal_mult_prop: SliderInput,
    corruption_gain_mult_prop: SliderInput,
    sorcery_corruption_gain_mult_prop: SliderInput,
}

impl SurvivalTab {
    pub fn new(settings: &PublicSurvivalSettings, include_private: bool) -> Rc<Self> {
        let input_width = min_input_width(&["99.9"]);

        let root = Scrollable::builder().with_gap(10, 10);

        let mut grid = Grid::builder_with_factory(wrapper_factory())
            .with_row_spacing(5)
            .with_col_spacing(10);

        grid.col().add();
        grid.col().with_stretch(1).add();
        grid.col().with_min_size(input_width).add();

        let mut grid = PrivateBuilder::new(grid, include_private);

        let stamina_cost_mult_prop =
            grid.public
                .range_prop("Stamina cost multiplier:", 0.1, 10.0, 1.0, 10);
        let active_thirst_mult_prop =
            grid.public
                .range_prop("Player active thirst multiplier:", 0.1, 10.0, 1.0, 10);
        let active_hunger_mult_prop =
            grid.public
                .range_prop("Player active hunger multiplier:", 0.1, 10.0, 1.0, 10);
        let idle_thirst_mult_prop =
            grid.public
                .range_prop("Player idle thirst multiplier:", 0.1, 10.0, 1.0, 10);
        let idle_hunger_mult_prop =
            grid.public
                .range_prop("Player idle hunger multiplier:", 0.1, 10.0, 1.0, 10);
        let drop_items_on_death_prop = grid.public.enum_prop(
            "Drop equipment on death:",
            &["Everything", "Backpack", "Nothing"],
        );
        let anyone_can_loot_corpse_prop = grid.public.bool_prop("Everybody can loot corpse");
        let corruption_removal_mult_prop =
            grid.range_prop("Thrall corruption removal multiplier:", 0.1, 10.0, 1.0, 10);
        let corruption_gain_mult_prop =
            grid.range_prop("Player corruption gain multiplier:", 0.1, 10.0, 1.0, 10);
        let sorcery_corruption_gain_mult_prop = grid.range_prop(
            "Player sorcerous corruption gain multiplier:",
            0.1,
            10.0,
            1.0,
            10,
        );

        let root = root.add(grid.into_inner().end());
        root.group().hide();

        let private_props = include_private.then(|| PrivateProperties {
            corruption_removal_mult_prop: corruption_removal_mult_prop.unwrap(),
            corruption_gain_mult_prop: corruption_gain_mult_prop.unwrap(),
            sorcery_corruption_gain_mult_prop: sorcery_corruption_gain_mult_prop.unwrap(),
        });

        Rc::new(Self {
            root,
            offline_chars_in_world: settings.offline_chars_in_world,
            stamina_cost_mult_prop,
            active_thirst_mult_prop,
            active_hunger_mult_prop,
            idle_thirst_mult_prop,
            idle_hunger_mult_prop,
            drop_items_on_death_prop,
            anyone_can_loot_corpse_prop,
            private_props,
        })
    }

    pub fn public_values(&self) -> PublicSurvivalSettings {
        PublicSurvivalSettings {
            stamina_cost_mult: self.stamina_cost_mult_prop.value().into(),
            idle_thirst_mult: self.idle_thirst_mult_prop.value().into(),
            active_thirst_mult: self.active_thirst_mult_prop.value().into(),
            idle_hunger_mult: self.idle_hunger_mult_prop.value().into(),
            active_hunger_mult: self.active_hunger_mult_prop.value().into(),
            drop_items_on_death:
                DropOnDeath::from_repr(self.drop_items_on_death_prop.value() as u8).unwrap(),
            anyone_can_loot_corpse: self.anyone_can_loot_corpse_prop.is_checked(),
            offline_chars_in_world: self.offline_chars_in_world,
        }
    }

    pub fn values(&self) -> SurvivalSettings {
        self.private_props
            .as_ref()
            .unwrap()
            .values(self.public_values())
    }

    pub fn set_public_values(&self, settings: &PublicSurvivalSettings) {
        self.stamina_cost_mult_prop
            .set_value(settings.stamina_cost_mult);
        self.active_thirst_mult_prop
            .set_value(settings.active_thirst_mult);
        self.active_hunger_mult_prop
            .set_value(settings.active_hunger_mult);
        self.idle_thirst_mult_prop
            .set_value(settings.idle_thirst_mult);
        self.idle_hunger_mult_prop
            .set_value(settings.idle_hunger_mult);
        self.drop_items_on_death_prop
            .set_value(settings.drop_items_on_death as u8);
        self.anyone_can_loot_corpse_prop
            .set_checked(settings.anyone_can_loot_corpse);
    }

    pub fn set_values(&self, settings: &SurvivalSettings) {
        self.set_public_values(settings);
        self.private_props.as_ref().unwrap().set_values(settings);
    }
}

impl LayoutElement for SurvivalTab {
    fn min_size(&self) -> fltk_float::Size {
        self.root.min_size()
    }

    fn layout(&self, x: i32, y: i32, width: i32, height: i32) {
        self.root.layout(x, y, width, height)
    }
}

impl SettingsTab for SurvivalTab {
    fn root(&self) -> impl WidgetExt + 'static {
        self.root.group()
    }
}

impl PrivateProperties {
    fn values(&self, public: PublicSurvivalSettings) -> SurvivalSettings {
        SurvivalSettings {
            public,
            corruption_removal_mult: self.corruption_removal_mult_prop.value().into(),
            corruption_gain_mult: self.corruption_gain_mult_prop.value().into(),
            sorcery_corruption_gain_mult: self.sorcery_corruption_gain_mult_prop.value().into(),
        }
    }

    fn set_values(&self, settings: &SurvivalSettings) {
        self.corruption_removal_mult_prop
            .set_value(settings.corruption_removal_mult);
        self.corruption_gain_mult_prop
            .set_value(settings.corruption_gain_mult);
        self.sorcery_corruption_gain_mult_prop
            .set_value(settings.sorcery_corruption_gain_mult);
    }
}
