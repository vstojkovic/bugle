use chrono::TimeDelta;
use ini_persist::load::LoadProperty;
use ini_persist::save::{default_remove, SaveProperty};

use crate::game::settings::{display_minutes, parse_minutes, Multiplier};

#[derive(Debug, Clone, LoadProperty, SaveProperty)]
pub struct FollowerSettings {
    #[ini(rename = "AnimalPenCraftingTimeMultiplier")]
    pub pen_crafting_time_mult: Multiplier,

    #[ini(rename = "FeedBoxRangeMultiplier")]
    pub feeder_rang_mult: Multiplier,

    #[ini(rename = "UseMinionPopulationLimit")]
    pub cap_enabled: bool,

    #[ini(rename = "MinionPopulationBaseValue")]
    pub cap_base: u8,

    #[ini(rename = "MinionPopulationPerPlayer")]
    pub cap_per_player: u8,

    #[ini(rename = "MinionOverpopulationCleanup", parse_with = parse_minutes, remove_with = default_remove, display_with = display_minutes)]
    pub cleanup_interval: TimeDelta,
}

impl Default for FollowerSettings {
    fn default() -> Self {
        Self {
            pen_crafting_time_mult: Multiplier(1.0),
            feeder_rang_mult: Multiplier(1.0),
            cap_enabled: false,
            cap_base: 50,
            cap_per_player: 5,
            cleanup_interval: TimeDelta::try_minutes(60).unwrap(),
        }
    }
}
