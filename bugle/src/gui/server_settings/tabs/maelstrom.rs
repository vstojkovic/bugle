use std::rc::Rc;

use chrono::TimeDelta;
use fltk::button::CheckButton;
use fltk::prelude::*;
use fltk_float::grid::Grid;
use fltk_float::scroll::Scrollable;
use fltk_float::LayoutElement;
use num::ToPrimitive;

use crate::game::settings::server::MaelstromSettings;
use crate::gui::server_settings::{min_input_width, EditorBuilder, SliderInput, WeeklyHoursInput};
use crate::gui::wrapper_factory;

use super::SettingsTab;

pub struct MaelstromTab {
    root: Scrollable,
    storm_enabled_prop: CheckButton,
    storm_hours_prop: WeeklyHoursInput,
    storm_min_online_players_prop: SliderInput,
    storm_endurance_drain_mult_prop: SliderInput,
    build_in_storm_enabled_prop: CheckButton,
    storm_map_blocker_prop: CheckButton,
    monsters_enabled_prop: CheckButton,
    monster_idle_lifespan_prop: SliderInput,
    monster_spawn_rate_mult_prop: SliderInput,
    env_monster_respawn_rate_mult_prop: SliderInput,
    max_env_monsters_prop: SliderInput,
    max_ambush_monsters_prop: SliderInput,
    siege_monsters_enabled_prop: CheckButton,
    siege_monster_map_markers_prop: CheckButton,
    siege_monster_dmg_mult_prop: SliderInput,
    min_siege_build_size_prop: SliderInput,
    siege_monster_respawn_rate_mult_prop: SliderInput,
    max_siege_monsters_prop: SliderInput,
    siege_build_size_mult_prop: SliderInput,
    storm_cooldown_prop: SliderInput,
    storm_accumulation_prop: SliderInput,
    storm_duration_prop: SliderInput,
    storm_dissipation_prop: SliderInput,
    storm_build_dmg_enabled_prop: CheckButton,
    min_storm_build_size_prop: SliderInput,
    storm_build_dmg_rate_mult_prop: SliderInput,
    storm_build_dmg_mult_prop: SliderInput,
    vault_refresh_time_prop: SliderInput,
    vault_refresh_deviation_prop: SliderInput,
    surge_cost_mult_prop: SliderInput,
    surge_despawn_time_prop: SliderInput,
    shrine_defense_duration_mult_prop: SliderInput,
}

impl MaelstromTab {
    pub fn new() -> Rc<Self> {
        let input_width = min_input_width(&["23:59", "9999.99"]);

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

        let storm_enabled_prop = grid.bool_prop("Enable Maelstrom");
        let storm_hours_prop = grid.weekly_hours_prop();
        let storm_min_online_players_prop =
            grid.range_prop("Maelstrom minimum online players:", 0.0, 40.0, 1.0, 1);
        let storm_endurance_drain_mult_prop =
            grid.range_prop("Maelstrom endurance drain multiplier:", 0.1, 1.0, 1.0, 10);
        let build_in_storm_enabled_prop = grid.bool_prop("Allow building in Maelstrom");
        let storm_map_blocker_prop = grid.bool_prop("Show Maelstrom on map");
        let monsters_enabled_prop = grid.bool_prop("Elder Things enabled");
        let monster_idle_lifespan_prop =
            grid.range_prop("Elder Thing idle lifespan:", 30.0, 1800.0, 1.0, 1);
        let monster_spawn_rate_mult_prop =
            grid.range_prop("Elder Thing spawn rate:", 0.1, 9.0, 1.0, 10);
        let env_monster_respawn_rate_mult_prop =
            grid.range_prop("Ambient Elder Thing respawn rate:", 0.1, 10.0, 1.0, 10);
        let max_env_monsters_prop =
            grid.range_prop("Max ambient Elder Things:", 1.0, 700.0, 1.0, 1);
        let max_ambush_monsters_prop =
            grid.range_prop("Max ambush Elder Things:", 1.0, 300.0, 1.0, 1);
        let siege_monsters_enabled_prop = grid.bool_prop("Siege Elder Things enabled");
        let siege_monster_map_markers_prop = grid.bool_prop("Siege Elder Thing map markers");
        let siege_monster_dmg_mult_prop = grid.range_prop(
            "Elder Thing siege damage multiplier:",
            1.0,
            2500.0,
            1.0,
            100,
        );
        let min_siege_build_size_prop =
            grid.range_prop("Minimum building size to be sieged:", 0.0, 1000.0, 1.0, 1);
        let siege_monster_respawn_rate_mult_prop =
            grid.range_prop("Siege Elder Thing respawn rate:", 0.01, 10.0, 1.0, 100);
        let max_siege_monsters_prop = grid.range_prop("Max siege Elder Things:", 1.0, 25.0, 1.0, 1);
        let siege_build_size_mult_prop = grid.range_prop(
            "Elder Thing siege building size multiplier:",
            0.1,
            10.0,
            1.0,
            10,
        );
        let storm_cooldown_prop = grid.range_prop("Maelstrom cooldown:", 1.0, 1440.0, 1.0, 1);
        let storm_accumulation_prop =
            grid.range_prop("Maelstrom accumulation time:", 1.0, 1440.0, 1.0, 1);
        let storm_duration_prop = grid.range_prop("Maelstrom duration:", 1.0, 1440.0, 1.0, 1);
        let storm_dissipation_prop =
            grid.range_prop("Maelstrom dissipation time:", 1.0, 1440.0, 1.0, 1);
        let storm_build_dmg_enabled_prop = grid.bool_prop("Maelstrom building damage enabled");
        let min_storm_build_size_prop = grid.range_prop(
            "Minimum building pieces for Maelstrom damage:",
            0.0,
            1000.0,
            1.0,
            1,
        );
        let storm_build_dmg_rate_mult_prop = grid.range_prop(
            "Maelstrom building damage rate multiplier:",
            0.1,
            20.0,
            1.0,
            10,
        );
        let storm_build_dmg_mult_prop =
            grid.range_prop("Maelstrom building damage multiplier:", 0.1, 20.0, 1.0, 10);
        let vault_refresh_time_prop = grid.range_prop("Vault refresh time:", 2.0, 10800.0, 1.0, 1);
        let vault_refresh_deviation_prop =
            grid.range_prop("Vault refresh deviation:", 0.0, 3600.0, 1.0, 1);
        let surge_cost_mult_prop = grid.range_prop("Surge cost multiplier:", 0.1, 5.0, 1.0, 10);
        let surge_despawn_time_prop = grid.range_prop("Surge despawn timer:", 5.0, 300.0, 1.0, 1);
        let shrine_defense_duration_mult_prop = grid.range_prop(
            "Leyshrine defense active time multiplier:",
            0.1,
            10.0,
            1.0,
            10,
        );

        let grid = grid.end();

        let root = root.add(grid);
        root.group().hide();

        Rc::new(Self {
            root,
            storm_enabled_prop,
            storm_hours_prop,
            storm_min_online_players_prop,
            storm_endurance_drain_mult_prop,
            build_in_storm_enabled_prop,
            storm_map_blocker_prop,
            monsters_enabled_prop,
            monster_idle_lifespan_prop,
            monster_spawn_rate_mult_prop,
            env_monster_respawn_rate_mult_prop,
            max_env_monsters_prop,
            max_ambush_monsters_prop,
            siege_monsters_enabled_prop,
            siege_monster_map_markers_prop,
            siege_monster_dmg_mult_prop,
            min_siege_build_size_prop,
            siege_monster_respawn_rate_mult_prop,
            max_siege_monsters_prop,
            siege_build_size_mult_prop,
            storm_cooldown_prop,
            storm_accumulation_prop,
            storm_duration_prop,
            storm_dissipation_prop,
            storm_build_dmg_enabled_prop,
            min_storm_build_size_prop,
            storm_build_dmg_rate_mult_prop,
            storm_build_dmg_mult_prop,
            vault_refresh_time_prop,
            vault_refresh_deviation_prop,
            surge_cost_mult_prop,
            surge_despawn_time_prop,
            shrine_defense_duration_mult_prop,
        })
    }

    pub fn values(&self) -> MaelstromSettings {
        MaelstromSettings {
            storm_enabled: self.storm_enabled_prop.is_checked(),
            storm_hours: self.storm_hours_prop.value(),
            storm_min_online_players: self.storm_min_online_players_prop.value().to_u8().unwrap(),
            storm_endurance_drain_mult: self.storm_endurance_drain_mult_prop.value().into(),
            build_in_storm_enabled: self.build_in_storm_enabled_prop.is_checked(),
            storm_map_blocker: self.storm_map_blocker_prop.is_checked(),
            monsters_enabled: self.monsters_enabled_prop.is_checked(),
            monster_idle_lifespan: TimeDelta::try_seconds(
                self.monster_idle_lifespan_prop.value() as i64
            )
            .unwrap(),
            monster_spawn_rate_mult: self.monster_spawn_rate_mult_prop.value().into(),
            env_monster_respawn_rate_mult: self.env_monster_respawn_rate_mult_prop.value().into(),
            max_env_monsters: self.max_env_monsters_prop.value().to_u16().unwrap(),
            max_ambush_monsters: self.max_ambush_monsters_prop.value().to_u16().unwrap(),
            siege_monsters_enabled: self.siege_monsters_enabled_prop.is_checked(),
            siege_monster_map_markers: self.siege_monster_map_markers_prop.is_checked(),
            siege_monster_dmg_mult: self.siege_monster_dmg_mult_prop.value().into(),
            min_siege_build_size: self.min_siege_build_size_prop.value().to_u16().unwrap(),
            siege_monster_respawn_rate_mult: self
                .siege_monster_respawn_rate_mult_prop
                .value()
                .into(),
            max_siege_monsters: self.max_siege_monsters_prop.value().to_u8().unwrap(),
            siege_build_size_mult: self.siege_build_size_mult_prop.value().into(),
            storm_cooldown: TimeDelta::try_minutes(self.storm_cooldown_prop.value() as i64)
                .unwrap(),
            storm_accumulation: TimeDelta::try_minutes(self.storm_accumulation_prop.value() as i64)
                .unwrap(),
            storm_duration: TimeDelta::try_minutes(self.storm_duration_prop.value() as i64)
                .unwrap(),
            storm_dissipation: TimeDelta::try_minutes(self.storm_dissipation_prop.value() as i64)
                .unwrap(),
            storm_build_dmg_enabled: self.storm_build_dmg_enabled_prop.is_checked(),
            min_storm_build_size: self.min_storm_build_size_prop.value().to_u16().unwrap(),
            storm_build_dmg_rate_mult: self.storm_build_dmg_rate_mult_prop.value().into(),
            storm_build_dmg_mult: self.storm_build_dmg_mult_prop.value().into(),
            vault_refresh_time: TimeDelta::try_minutes(self.vault_refresh_time_prop.value() as i64)
                .unwrap(),
            vault_refresh_deviation: TimeDelta::try_minutes(
                self.vault_refresh_deviation_prop.value() as i64,
            )
            .unwrap(),
            surge_cost_mult: self.surge_cost_mult_prop.value().into(),
            surge_despawn_time: TimeDelta::try_minutes(self.surge_despawn_time_prop.value() as i64)
                .unwrap(),
            shrine_defense_duration_mult: self.shrine_defense_duration_mult_prop.value().into(),
        }
    }

    pub fn set_values(&self, settings: &MaelstromSettings) {
        self.storm_enabled_prop.set_checked(settings.storm_enabled);
        self.storm_hours_prop.set_value(&settings.storm_hours);
        self.storm_min_online_players_prop
            .set_value(settings.storm_min_online_players);
        self.storm_endurance_drain_mult_prop
            .set_value(settings.storm_endurance_drain_mult);
        self.build_in_storm_enabled_prop
            .set_checked(settings.build_in_storm_enabled);
        self.storm_map_blocker_prop
            .set_checked(settings.storm_map_blocker);
        self.monsters_enabled_prop
            .set_checked(settings.monsters_enabled);
        self.monster_idle_lifespan_prop
            .set_value(settings.monster_idle_lifespan.num_seconds() as f64);
        self.monster_spawn_rate_mult_prop
            .set_value(settings.monster_spawn_rate_mult);
        self.env_monster_respawn_rate_mult_prop
            .set_value(settings.env_monster_respawn_rate_mult);
        self.max_env_monsters_prop
            .set_value(settings.max_env_monsters);
        self.max_ambush_monsters_prop
            .set_value(settings.max_ambush_monsters);
        self.siege_monsters_enabled_prop
            .set_checked(settings.siege_monsters_enabled);
        self.siege_monster_map_markers_prop
            .set_checked(settings.siege_monster_map_markers);
        self.siege_monster_dmg_mult_prop
            .set_value(settings.siege_monster_dmg_mult);
        self.min_siege_build_size_prop
            .set_value(settings.min_siege_build_size);
        self.siege_monster_respawn_rate_mult_prop
            .set_value(settings.siege_monster_respawn_rate_mult);
        self.max_siege_monsters_prop
            .set_value(settings.max_siege_monsters);
        self.siege_build_size_mult_prop
            .set_value(settings.siege_build_size_mult);
        self.storm_cooldown_prop
            .set_value(settings.storm_cooldown.num_minutes() as f64);
        self.storm_accumulation_prop
            .set_value(settings.storm_accumulation.num_minutes() as f64);
        self.storm_duration_prop
            .set_value(settings.storm_duration.num_minutes() as f64);
        self.storm_dissipation_prop
            .set_value(settings.storm_dissipation.num_minutes() as f64);
        self.storm_build_dmg_enabled_prop
            .set_checked(settings.storm_build_dmg_enabled);
        self.min_storm_build_size_prop
            .set_value(settings.min_storm_build_size);
        self.storm_build_dmg_rate_mult_prop
            .set_value(settings.storm_build_dmg_rate_mult);
        self.storm_build_dmg_mult_prop
            .set_value(settings.storm_build_dmg_mult);
        self.vault_refresh_time_prop
            .set_value(settings.vault_refresh_time.num_minutes() as f64);
        self.vault_refresh_deviation_prop
            .set_value(settings.vault_refresh_deviation.num_minutes() as f64);
        self.surge_cost_mult_prop
            .set_value(settings.surge_cost_mult);
        self.surge_despawn_time_prop
            .set_value(settings.surge_despawn_time.num_minutes() as f64);
        self.shrine_defense_duration_mult_prop
            .set_value(settings.shrine_defense_duration_mult);
    }
}

impl LayoutElement for MaelstromTab {
    fn min_size(&self) -> fltk_float::Size {
        self.root.min_size()
    }

    fn layout(&self, x: i32, y: i32, width: i32, height: i32) {
        self.root.layout(x, y, width, height)
    }
}

impl SettingsTab for MaelstromTab {
    fn root(&self) -> impl WidgetExt + 'static {
        self.root.group()
    }
}
