use std::ops::{Deref, DerefMut};

use ini_persist::load::LoadProperty;
use ini_persist::save::SaveProperty;
use serde::{Deserialize, Serialize};

use crate::game::settings::Multiplier;

#[derive(Clone, Debug, Default, Deserialize, Serialize, LoadProperty, SaveProperty)]
pub struct PublicProgressionSettings {
    #[serde(rename = "Sz")]
    #[ini(rename = "PlayerXPRateMultiplier")]
    pub xp_rate_mult: Multiplier,
}

#[derive(Debug, Clone, Default, LoadProperty, SaveProperty)]
pub struct ProgressionSettings {
    #[ini(flatten)]
    pub public: PublicProgressionSettings,

    #[ini(rename = "PlayerXPTimeMultiplier")]
    pub xp_time_mult: Multiplier,

    #[ini(rename = "PlayerXPKillMultiplier")]
    pub xp_kill_mult: Multiplier,

    #[ini(rename = "PlayerXPHarvestMultiplier")]
    pub xp_harvest_mult: Multiplier,

    #[ini(rename = "PlayerXPCraftMultiplier")]
    pub xp_craft_mult: Multiplier,
}

impl Deref for ProgressionSettings {
    type Target = PublicProgressionSettings;
    fn deref(&self) -> &Self::Target {
        &self.public
    }
}

impl DerefMut for ProgressionSettings {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.public
    }
}
