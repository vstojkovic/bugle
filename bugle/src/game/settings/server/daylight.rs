use std::ops::{Deref, DerefMut};

use ini_persist::load::LoadProperty;
use ini_persist::save::SaveProperty;
use serde::{Deserialize, Serialize};

use crate::game::settings::Multiplier;

#[derive(Clone, Debug, Deserialize, Serialize, LoadProperty, SaveProperty)]
pub struct BaseDaylightSettings {
    #[serde(rename = "Sb", default)]
    #[ini(rename = "DayCycleSpeedScale")]
    pub day_cycle_speed_mult: Multiplier,

    #[serde(rename = "Sg", default)]
    #[ini(rename = "DawnDuskSpeedScale")]
    pub dawn_dusk_speed_mult: Multiplier,

    #[serde(rename = "Sd", default = "default_catch_up_time")]
    #[ini(rename = "UseClientCatchUpTime")]
    pub use_catch_up_time: bool,
}

impl Default for BaseDaylightSettings {
    fn default() -> Self {
        Self {
            day_cycle_speed_mult: Default::default(),
            dawn_dusk_speed_mult: Default::default(),
            use_catch_up_time: default_catch_up_time(),
        }
    }
}

#[derive(Debug, Clone, LoadProperty, SaveProperty)]
pub struct DaylightSettings {
    #[ini(flatten)]
    pub base: BaseDaylightSettings,

    #[ini(rename = "DayTimeSpeedScale")]
    pub day_time_speed_mult: Multiplier,

    #[ini(rename = "NightTimeSpeedScale")]
    pub night_time_speed_mult: Multiplier,

    #[ini(rename = "ClientCatchUpTime")]
    pub catch_up_time: f64, // TODO: Figure out if fractional values are supported
}

impl Deref for DaylightSettings {
    type Target = BaseDaylightSettings;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for DaylightSettings {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl Default for DaylightSettings {
    fn default() -> Self {
        Self {
            base: Default::default(),
            day_time_speed_mult: Multiplier(1.0),
            night_time_speed_mult: Multiplier(1.0),
            catch_up_time: 10.0,
        }
    }
}

#[inline(always)]
const fn default_catch_up_time() -> bool {
    true
}
