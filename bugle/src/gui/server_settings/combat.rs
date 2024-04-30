use std::rc::Rc;

use chrono::TimeDelta;
use fltk::button::CheckButton;
use fltk::prelude::*;
use fltk_float::grid::Grid;
use fltk_float::scroll::Scrollable;
use fltk_float::LayoutElement;

use crate::game::settings::server::{BaseCombatSettings, CombatSettings};
use crate::gui::wrapper_factory;

use super::{min_input_width, EditorBuilder, SliderInput, WeeklyHoursInput};

pub struct CombatTab {
    root: Scrollable,
    player_dmg_mult_prop: SliderInput,
    player_dmg_recv_mult_prop: SliderInput,
    npc_dmg_mult_prop: SliderInput,
    npc_dmg_recv_mult_prop: SliderInput,
    thrall_player_dmg_mult_prop: SliderInput,
    thrall_npc_dmg_mult_prop: SliderInput,
    npc_respawn_mult_prop: SliderInput,
    friendly_fire_dmg_mult_prop: SliderInput,
    raid_dmg_mult_prop: SliderInput,
    durability_mult_prop: SliderInput,
    thrall_wakeup_time_prop: SliderInput,
    gods_disabled_prop: CheckButton,
    gods_restricted_prop: CheckButton,
    gods_hours_prop: WeeklyHoursInput,
    aim_lock_enabled_prop: CheckButton,
}

impl CombatTab {
    pub fn new() -> Rc<Self> {
        let input_width = min_input_width(&["23:59", "9999"]);

        let root = Scrollable::builder().with_gap(10, 10);

        let mut grid = Grid::builder_with_factory(wrapper_factory())
            .with_row_spacing(5)
            .with_col_spacing(10);

        grid.col().add(); // label
        grid.col().with_stretch(1).add(); // checkbox
        grid.col().add(); // start label
        grid.col().with_min_size(input_width).add(); // start input
        grid.col().add(); // end label
        grid.col().with_min_size(input_width).add(); // end input

        let player_dmg_mult_prop = grid.range_prop("Player damage multiplier:", 0.1, 10.0, 1.0, 10);
        let player_dmg_recv_mult_prop =
            grid.range_prop("Player damage taken multiplier:", 0.1, 10.0, 1.0, 10);
        let npc_dmg_mult_prop = grid.range_prop("NPC damage multiplier:", 0.1, 10.0, 1.0, 10);
        let npc_dmg_recv_mult_prop =
            grid.range_prop("NPC damage taken multiplier:", 0.1, 10.0, 1.0, 10);
        let thrall_player_dmg_mult_prop =
            grid.range_prop("Thrall damage to players multiplier:", 0.1, 10.0, 1.0, 10);
        let thrall_npc_dmg_mult_prop =
            grid.range_prop("Thrall damage to NPCs multiplier:", 0.1, 10.0, 1.0, 10);
        let npc_respawn_mult_prop = grid.range_prop("NPC respawn multiplier:", 0.1, 10.0, 1.0, 10);
        let friendly_fire_dmg_mult_prop =
            grid.range_prop("Friendly fire damage multiplier:", 0.1, 10.0, 1.0, 10);
        let raid_dmg_mult_prop = grid.range_prop("Building damage multiplier:", 0.1, 10.0, 1.0, 10);
        let durability_mult_prop = grid.range_prop("Durability multiplier:", 0.1, 10.0, 1.0, 10);
        let thrall_wakeup_time_prop = grid.range_prop("Thrall wakeup time:", 900.0, 3600.0, 1.0, 1);
        let gods_disabled_prop = grid.bool_prop("Disable avatars");
        let gods_restricted_prop = grid.bool_prop("Time restrict avatar summoning");
        let gods_hours_prop = grid.weekly_hours_prop();
        let aim_lock_enabled_prop = grid.bool_prop("Enable target lock");

        let grid = grid.end();

        let root = root.add(grid);
        root.group().hide();

        Rc::new(Self {
            root,
            player_dmg_mult_prop,
            player_dmg_recv_mult_prop,
            npc_dmg_mult_prop,
            npc_dmg_recv_mult_prop,
            thrall_player_dmg_mult_prop,
            thrall_npc_dmg_mult_prop,
            npc_respawn_mult_prop,
            friendly_fire_dmg_mult_prop,
            raid_dmg_mult_prop,
            durability_mult_prop,
            thrall_wakeup_time_prop,
            gods_disabled_prop,
            gods_restricted_prop,
            gods_hours_prop,
            aim_lock_enabled_prop,
        })
    }

    pub fn root(&self) -> impl WidgetExt {
        self.root.group()
    }

    pub fn values(&self) -> CombatSettings {
        CombatSettings {
            base: BaseCombatSettings {
                durability_mult: self.durability_mult_prop.value().into(),
                thrall_wakeup_time: TimeDelta::try_seconds(
                    self.thrall_wakeup_time_prop.value() as i64
                )
                .unwrap(),
            },
            player_dmg_mult: self.player_dmg_mult_prop.value().into(),
            player_dmg_recv_mult: self.player_dmg_recv_mult_prop.value().into(),
            npc_dmg_mult: self.npc_dmg_mult_prop.value().into(),
            npc_dmg_recv_mult: self.npc_dmg_recv_mult_prop.value().into(),
            thrall_player_dmg_mult: self.thrall_player_dmg_mult_prop.value().into(),
            thrall_npc_dmg_mult: self.thrall_npc_dmg_mult_prop.value().into(),
            npc_respawn_mult: self.npc_respawn_mult_prop.value().into(),
            friendly_fire_dmg_mult: self.friendly_fire_dmg_mult_prop.value().into(),
            raid_dmg_mult: self.raid_dmg_mult_prop.value().into(),
            gods_disabled: self.gods_disabled_prop.is_checked(),
            gods_restricted: self.gods_restricted_prop.is_checked(),
            gods_hours: self.gods_hours_prop.value(),
            aim_lock_enabled: self.aim_lock_enabled_prop.is_checked(),
        }
    }

    pub fn set_values(&self, settings: &CombatSettings) {
        self.player_dmg_mult_prop
            .set_value(settings.player_dmg_mult);
        self.player_dmg_recv_mult_prop
            .set_value(settings.player_dmg_recv_mult);
        self.npc_dmg_mult_prop.set_value(settings.npc_dmg_mult);
        self.npc_dmg_recv_mult_prop
            .set_value(settings.npc_dmg_recv_mult);
        self.thrall_player_dmg_mult_prop
            .set_value(settings.thrall_player_dmg_mult);
        self.thrall_npc_dmg_mult_prop
            .set_value(settings.thrall_npc_dmg_mult);
        self.npc_respawn_mult_prop
            .set_value(settings.npc_respawn_mult);
        self.friendly_fire_dmg_mult_prop
            .set_value(settings.friendly_fire_dmg_mult);
        self.raid_dmg_mult_prop.set_value(settings.raid_dmg_mult);
        self.durability_mult_prop
            .set_value(settings.durability_mult);
        self.thrall_wakeup_time_prop
            .set_value(settings.thrall_wakeup_time.num_seconds() as f64);
        self.gods_disabled_prop.set_checked(settings.gods_disabled);
        self.gods_restricted_prop
            .set_checked(settings.gods_restricted);
        self.gods_hours_prop.set_value(&settings.gods_hours);
        self.aim_lock_enabled_prop
            .set_checked(settings.aim_lock_enabled);
    }
}

impl LayoutElement for CombatTab {
    fn min_size(&self) -> fltk_float::Size {
        self.root.min_size()
    }

    fn layout(&self, x: i32, y: i32, width: i32, height: i32) {
        self.root.layout(x, y, width, height)
    }
}
