use std::ops::{Deref, DerefMut};

use ini_persist::load::LoadProperty;
use ini_persist::save::SaveProperty;
use serde::{Deserialize, Serialize};

use crate::game::settings::Multiplier;

#[derive(Clone, Debug, Default, Deserialize, Serialize, LoadProperty, SaveProperty)]
pub struct BaseCraftingSettings {
    #[serde(rename = "S8")]
    #[ini(rename = "ItemConvertionMultiplier")]
    pub crafting_time_mult: Multiplier,

    #[serde(rename = "S4")]
    #[ini(rename = "ThrallConversionMultiplier")]
    pub thrall_crafting_time_mult: Multiplier,
}

#[derive(Debug, Clone, Default, LoadProperty, SaveProperty)]
pub struct CraftingSettings {
    #[ini(flatten)]
    pub base: BaseCraftingSettings,

    #[ini(rename = "FuelBurnTimeMultiplier")]
    pub fuel_burn_time_mult: Multiplier,

    #[ini(rename = "CraftingCostMultiplier")]
    pub crafting_cost_mult: Multiplier,
}

impl Deref for CraftingSettings {
    type Target = BaseCraftingSettings;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for CraftingSettings {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}
