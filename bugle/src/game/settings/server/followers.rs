use chrono::TimeDelta;
use ini_persist::load::LoadProperty;

use crate::game::settings::{parse_minutes, Multiplier};

#[derive(Debug, Clone, LoadProperty)]
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

    #[ini(rename = "MinionOverpopulationCleanup", parse_with = parse_minutes)]
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
