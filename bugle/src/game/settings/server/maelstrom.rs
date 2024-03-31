use chrono::TimeDelta;
use ini_persist::load::LoadProperty;
use ini_persist::save::{default_remove, SaveProperty};

use crate::game::settings::{
    display_minutes, display_seconds, parse_minutes, parse_seconds, HourMinute, Hours, Multiplier,
    WeeklyHours,
};

#[derive(Debug, Clone, LoadProperty, SaveProperty)]
pub struct MaelstromSettings {
    #[ini(rename = "StormEnabled")]
    pub storm_enabled: bool,

    #[ini(rename = "StormTime")]
    pub storm_hours: WeeklyHours,

    #[ini(rename = "StormMinimumOnlinePlayers")]
    pub storm_min_online_players: u8,

    #[ini(rename = "StormEnduranceDrainMultiplier")]
    pub storm_endurance_drain_mult: Multiplier,

    #[ini(rename = "StormBuildingAllowed")]
    pub build_in_storm_enabled: bool,

    #[ini(rename = "StormMapBlocker")]
    pub storm_map_blocker: bool,

    #[ini(rename = "ElderThingsEnabled")]
    pub monsters_enabled: bool,

    #[ini(rename = "ElderThingsIdleLifespan", parse_with = parse_seconds, remove_with = default_remove, display_with = display_seconds)]
    pub monster_idle_lifespan: TimeDelta, // UNIT: seconds?

    #[ini(rename = "ElderThingSpawnRate")]
    pub monster_spawn_rate_mult: Multiplier,

    #[ini(rename = "AmbientElderThingRespawnRate")]
    pub env_monster_respawn_rate_mult: Multiplier,

    #[ini(rename = "MaxAmbientElderThings")]
    pub max_env_monsters: u16,

    #[ini(rename = "MaxAmbushElderThings")]
    pub max_ambush_monsters: u16,

    #[ini(rename = "SiegeElderThingsEnabled")]
    pub siege_monsters_enabled: bool,

    #[ini(rename = "SiegeElderThingMapMarkers")]
    pub siege_monster_map_markers: bool,

    #[ini(rename = "ElderThingSiegeDamageMultiplier")]
    pub siege_monster_dmg_mult: Multiplier,

    #[ini(rename = "MinimumBuildingSizeToBeSieged")]
    pub min_siege_build_size: u16,

    #[ini(rename = "SiegeElderThingRespawnRate")]
    pub siege_monster_respawn_rate_mult: Multiplier,

    #[ini(rename = "MaxSiegeElderThings")]
    pub max_siege_monsters: u8,

    #[ini(rename = "ElderThingSiegeBuildingSizeMultiplier")]
    pub siege_build_size_mult: Multiplier,

    #[ini(rename = "StormCooldown", parse_with = parse_minutes, remove_with = default_remove, display_with = display_minutes)]
    pub storm_cooldown: TimeDelta,

    #[ini(rename = "StormAccumulationTime", parse_with = parse_minutes, remove_with = default_remove, display_with = display_minutes)]
    pub storm_accumulation: TimeDelta,

    #[ini(rename = "StormDuration", parse_with = parse_minutes, remove_with = default_remove, display_with = display_minutes)]
    pub storm_duration: TimeDelta,

    #[ini(rename = "StormDissipationTime", parse_with = parse_minutes, remove_with = default_remove, display_with = display_minutes)]
    pub storm_dissipation: TimeDelta,

    #[ini(rename = "StormBuildingDamageEnabled")]
    pub storm_build_dmg_enabled: bool,

    #[ini(rename = "MinimumStormDamageBuildingPieces")]
    pub min_storm_build_size: u16,

    #[ini(rename = "StormBuildingDamageRateMultiplier")]
    pub storm_build_dmg_rate_mult: Multiplier,

    #[ini(rename = "StormBuildingDamageMultiplier")]
    pub storm_build_dmg_mult: Multiplier,

    #[ini(rename = "VaultRefreshTime", parse_with = parse_minutes, remove_with = default_remove, display_with = display_minutes)]
    pub vault_refresh_time: TimeDelta,

    #[ini(rename = "VaultRefreshDeviation", parse_with = parse_minutes, remove_with = default_remove, display_with = display_minutes)]
    pub vault_refresh_deviation: TimeDelta,

    #[ini(rename = "SurgeSacrificeRequirementMultiplier")]
    pub surge_cost_mult: Multiplier,

    #[ini(rename = "SurgeDespawnTimer", parse_with = parse_minutes, remove_with = default_remove, display_with = display_minutes)]
    pub surge_despawn_time: TimeDelta,

    #[ini(rename = "AltarModuleActiveTimeMultiplier")]
    pub shrine_defense_duration_mult: Multiplier,
}

impl Default for MaelstromSettings {
    fn default() -> Self {
        Self {
            storm_enabled: true,
            storm_hours: WeeklyHours {
                weekday_hours: Hours {
                    start: HourMinute(00_00),
                    end: HourMinute(23_59),
                },
                weekend_hours: Hours {
                    start: HourMinute(00_00),
                    end: HourMinute(23_59),
                },
            },
            storm_min_online_players: 0,
            storm_endurance_drain_mult: Multiplier(0.0),
            build_in_storm_enabled: true,
            storm_map_blocker: true,
            monsters_enabled: true,
            monster_idle_lifespan: TimeDelta::try_seconds(30).unwrap(),
            monster_spawn_rate_mult: Multiplier(1.0),
            env_monster_respawn_rate_mult: Multiplier(1.0),
            max_env_monsters: 700,
            max_ambush_monsters: 200,
            siege_monsters_enabled: false,
            siege_monster_map_markers: false,
            siege_monster_dmg_mult: Multiplier(1.0),
            min_siege_build_size: 41,
            siege_monster_respawn_rate_mult: Multiplier(1.0),
            max_siege_monsters: 5,
            siege_build_size_mult: Multiplier(1.0),
            storm_cooldown: TimeDelta::try_minutes(105).unwrap(),
            storm_accumulation: TimeDelta::try_minutes(1).unwrap(),
            storm_duration: TimeDelta::try_minutes(15).unwrap(),
            storm_dissipation: TimeDelta::try_minutes(1).unwrap(),
            storm_build_dmg_enabled: false,
            min_storm_build_size: 0,
            storm_build_dmg_rate_mult: Multiplier(1.0),
            storm_build_dmg_mult: Multiplier(1.0),
            vault_refresh_time: TimeDelta::try_minutes(10).unwrap(),
            vault_refresh_deviation: TimeDelta::try_minutes(2).unwrap(),
            surge_cost_mult: Multiplier(1.0),
            surge_despawn_time: TimeDelta::try_minutes(90).unwrap(),
            shrine_defense_duration_mult: Multiplier(1.0),
        }
    }
}
