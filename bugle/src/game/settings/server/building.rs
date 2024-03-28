use chrono::TimeDelta;
use ini_persist::LoadProperty;
use strum_macros::EnumIter;

use crate::game::settings::{parse_seconds, Multiplier};

#[derive(Debug, Clone, LoadProperty)]
pub struct BuildingSettings {
    #[ini(rename = "CreativeModeServer")]
    pub creative_mode: CreativeMode,

    #[ini(rename = "AllowBuildingAnywhere")]
    pub build_anywhere: bool,

    #[ini(rename = "StabilityLossMultiplier")]
    pub stability_loss_mult: Multiplier,

    #[ini(rename = "DisableBuildingDuringTimeRestrictedPVP")]
    pub build_during_pvp_disabled: bool,

    #[ini(rename = "DisableBuildingAbandonment")]
    pub abandonment_disabled: bool,

    #[ini(rename = "BuildingDecayTimeMultiplier")]
    pub decay_time_mult: Multiplier,

    #[ini(rename = "DisableThrallDecay")]
    pub thrall_decay_disabled: bool,

    #[ini(rename = "ThrallDecayTime", parse_with = parse_seconds)]
    pub thrall_decay_time: TimeDelta,
}

impl Default for BuildingSettings {
    fn default() -> Self {
        Self {
            creative_mode: CreativeMode::Admins,
            build_anywhere: false,
            stability_loss_mult: Multiplier(1.0),
            build_during_pvp_disabled: false,
            abandonment_disabled: true,
            decay_time_mult: Multiplier(1.0),
            thrall_decay_disabled: false,
            thrall_decay_time: TimeDelta::try_days(15).unwrap(),
        }
    }
}

#[derive(Debug, Clone, Copy, EnumIter, LoadProperty)]
#[ini(repr)]
pub enum CreativeMode {
    Admins,
    Everybody,
    Forced,
}
