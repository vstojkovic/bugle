use std::ops::{Deref, DerefMut};

use ini_persist::LoadProperty;
use serde::{Deserialize, Serialize};

use crate::game::settings::Multiplier;

#[derive(Clone, Debug, Default, Deserialize, Serialize, LoadProperty)]
pub struct BaseHarvestingSettings {
    #[serde(rename = "Sl")]
    #[ini(rename = "HarvestAmountMultiplier")]
    pub harvest_amount_mult: Multiplier,

    #[serde(rename = "Sk", default)]
    #[ini(rename = "ItemSpoilRateScale")]
    pub item_spoil_rate_mult: Multiplier,

    #[serde(rename = "Sm", default)]
    #[ini(rename = "ResourceRespawnSpeedMultiplier")]
    pub rsrc_respawn_speed_mult: Multiplier,
}

#[derive(Debug, Clone, Default, LoadProperty)]
pub struct HarvestingSettings {
    #[ini(flatten)]
    pub base: BaseHarvestingSettings,

    #[ini(rename = "LandClaimRadiusMultiplier")]
    pub claim_radius_mult: Multiplier,
}

impl Deref for HarvestingSettings {
    type Target = BaseHarvestingSettings;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for HarvestingSettings {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}
