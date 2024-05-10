use std::ops::{Deref, DerefMut};

use ini_persist::load::LoadProperty;
use ini_persist::save::SaveProperty;
use serde::{Deserialize, Serialize};

use crate::game::settings::Multiplier;

#[derive(Clone, Debug, Default, Deserialize, Serialize, LoadProperty, SaveProperty)]
pub struct PublicHarvestingSettings {
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

#[derive(Debug, Clone, Default, LoadProperty, SaveProperty)]
pub struct HarvestingSettings {
    #[ini(flatten)]
    pub public: PublicHarvestingSettings,

    #[ini(rename = "LandClaimRadiusMultiplier")]
    pub claim_radius_mult: Multiplier,
}

impl Deref for HarvestingSettings {
    type Target = PublicHarvestingSettings;
    fn deref(&self) -> &Self::Target {
        &self.public
    }
}

impl DerefMut for HarvestingSettings {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.public
    }
}
