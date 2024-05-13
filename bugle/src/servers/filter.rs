use std::str::FromStr;

use ini_persist::load::{LoadProperty, ParseProperty};
use ini_persist::save::{DisplayProperty, SaveProperty};
use strum_macros::{AsRefStr, EnumIter, EnumString, FromRepr};

use crate::game::settings::server::{Community, DropOnDeath};
use crate::game::settings::Multiplier;

use super::{Mode, Region, Server};

#[derive(Clone, Debug, Default, LoadProperty, SaveProperty)]
pub struct Filter {
    #[ini(rename = "Name", ignore_errors)]
    pub name: String,

    #[ini(rename = "Map", ignore_errors)]
    pub map: String,

    #[ini(rename = "Type", ignore_errors)]
    pub type_filter: TypeFilter,

    #[ini(rename = "Mode", ignore_errors)]
    pub mode: Option<Mode>,

    #[ini(rename = "Region", ignore_errors)]
    pub region: Option<Region>,

    #[ini(rename = "BattlEyeRequired", ignore_errors)]
    pub battleye_required: Option<bool>,

    #[ini(rename = "IncludeInvalid", ignore_errors)]
    pub include_invalid: bool,

    #[ini(rename = "IncludePasswordProtected", ignore_errors)]
    pub include_password_protected: bool,

    #[ini(rename = "Mods", ignore_errors)]
    pub mods: Option<bool>,

    #[ini(rename = "Community", ignore_errors)]
    pub community: Option<EnumFilter<Community>>,

    #[ini(rename = "MaxClanSize", ignore_errors)]
    pub max_clan_size: Option<RangeFilter<u16>>,

    #[ini(rename = "RaidEnabled", ignore_errors)]
    pub raid_enabled: Option<bool>,

    #[ini(rename = "RaidEnabled", ignore_errors)]
    pub raid_restricted: Option<bool>,

    #[ini(rename = "XPRateMult", ignore_errors)]
    pub xp_rate_mult: Option<RangeFilter<Multiplier>>,

    #[ini(rename = "DayCycleSpeedMult", ignore_errors)]
    pub day_cycle_speed_mult: Option<RangeFilter<Multiplier>>,

    #[ini(rename = "DawnDuskSpeedMult", ignore_errors)]
    pub dawn_dusk_speed_mult: Option<RangeFilter<Multiplier>>,

    #[ini(rename = "UseCatchUpTime", ignore_errors)]
    pub use_catch_up_time: Option<bool>,

    #[ini(rename = "StaminaCostMult", ignore_errors)]
    pub stamina_cost_mult: Option<RangeFilter<Multiplier>>,

    #[ini(rename = "IdleThirstMult", ignore_errors)]
    pub idle_thirst_mult: Option<RangeFilter<Multiplier>>,

    #[ini(rename = "ActiveThirstMult", ignore_errors)]
    pub active_thirst_mult: Option<RangeFilter<Multiplier>>,

    #[ini(rename = "IdleHungerMult", ignore_errors)]
    pub idle_hunger_mult: Option<RangeFilter<Multiplier>>,

    #[ini(rename = "ActiveHungerMult", ignore_errors)]
    pub active_hunger_mult: Option<RangeFilter<Multiplier>>,

    #[ini(rename = "DropItemsOnDeath", ignore_errors)]
    pub drop_items_on_death: Option<EnumFilter<DropOnDeath>>,

    #[ini(rename = "AnyoneCanLootCorpse", ignore_errors)]
    pub anyone_can_loot_corpse: Option<bool>,

    #[ini(rename = "OfflineCharsInWorld", ignore_errors)]
    pub offline_chars_in_world: Option<bool>,

    #[ini(rename = "DurabilityMult", ignore_errors)]
    pub durability_mult: Option<RangeFilter<Multiplier>>,

    #[ini(rename = "ThrallWakeupTimeSecs", ignore_errors)]
    pub thrall_wakeup_time_secs: Option<RangeFilter<i64>>,

    #[ini(rename = "HarvestAmountMult", ignore_errors)]
    pub harvest_amount_mult: Option<RangeFilter<Multiplier>>,

    #[ini(rename = "ItemSpoilRateMult", ignore_errors)]
    pub item_spoil_rate_mult: Option<RangeFilter<Multiplier>>,

    #[ini(rename = "RsrcRespawnSpeedMult", ignore_errors)]
    pub rsrc_respawn_speed_mult: Option<RangeFilter<Multiplier>>,

    #[ini(rename = "CraftingTimeMult", ignore_errors)]
    pub crafting_time_mult: Option<RangeFilter<Multiplier>>,

    #[ini(rename = "ThrallCraftingTimeMult", ignore_errors)]
    pub thrall_crafting_time_mult: Option<RangeFilter<Multiplier>>,
}

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    AsRefStr,
    EnumIter,
    EnumString,
    FromRepr,
    LoadProperty,
    SaveProperty,
)]
#[strum(ascii_case_insensitive)]
#[repr(u8)]
#[ini(ignore_case)]
pub enum TypeFilter {
    All,
    Official,
    Private,
    Favorite,
}

impl Default for TypeFilter {
    fn default() -> Self {
        Self::All
    }
}

impl TypeFilter {
    pub fn matches(&self, server: &Server) -> bool {
        match self {
            Self::All => true,
            Self::Official => server.is_official(),
            Self::Private => !server.is_official(),
            Self::Favorite => server.favorite,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RangeFilter<T: ParseProperty + DisplayProperty + Copy + PartialOrd> {
    pub min: Option<T>,
    pub max: Option<T>,
    pub negate: bool,
}

impl<T: ParseProperty + DisplayProperty + Copy + PartialOrd> RangeFilter<T> {
    pub fn matches(&self, value: T) -> bool {
        let in_range = self.min.map(|min| value >= min).unwrap_or(true)
            && self.max.map(|max| value <= max).unwrap_or(true);
        self.negate != in_range
    }
}

impl<T: ParseProperty + DisplayProperty + Copy + PartialOrd> ParseProperty for RangeFilter<T> {
    fn parse(text: &str) -> ini_persist::Result<Self> {
        let (negate, text) = if text.starts_with('!') { (true, &text[1..]) } else { (false, text) };
        let Some((min, max)) = text.split_once(',') else {
            return Err(ini_persist::Error::invalid_type("invalid format"));
        };
        let min = (!min.is_empty()).then(|| T::parse(min)).transpose()?;
        let max = (!max.is_empty()).then(|| T::parse(max)).transpose()?;
        if min.is_none() && max.is_none() {
            return Err(ini_persist::Error::invalid_value(
                "must have at least one bound",
            ));
        }
        Ok(Self { min, max, negate })
    }
}

impl<T: ParseProperty + DisplayProperty + Copy + PartialOrd> DisplayProperty for RangeFilter<T> {
    fn display(&self) -> String {
        let negate = if self.negate { "!" } else { "" };
        let min = self.min.as_ref().map(T::display).unwrap_or_default();
        let max = self.max.as_ref().map(T::display).unwrap_or_default();
        format!("{}{},{}", negate, min, max)
    }
}

#[derive(Debug, Clone)]
pub struct EnumFilter<T: FromStr + Into<&'static str> + Copy + Eq> {
    pub value: T,
    pub negate: bool,
}

impl<T: FromStr + Into<&'static str> + Copy + Eq> EnumFilter<T> {
    pub fn matches(&self, value: T) -> bool {
        self.negate != (self.value == value)
    }
}

impl<T: FromStr + Into<&'static str> + Copy + Eq> ParseProperty for EnumFilter<T> {
    fn parse(text: &str) -> ini_persist::Result<Self> {
        let (negate, text) = if text.starts_with('!') { (true, &text[1..]) } else { (false, text) };
        let value = T::from_str(text).map_err(|_| ini_persist::Error::custom("invalid format"))?;
        Ok(Self { value, negate })
    }
}

impl<T: FromStr + Into<&'static str> + Copy + Eq> DisplayProperty for EnumFilter<T> {
    fn display(&self) -> String {
        let negate = if self.negate { "!" } else { "" };
        format!("{}{}", negate, self.value.into())
    }
}
