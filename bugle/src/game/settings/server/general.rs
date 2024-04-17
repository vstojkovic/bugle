use std::ops::{Deref, DerefMut};

use anyhow::Result;
use chrono::TimeDelta;
use ini_persist::load::{LoadProperty, ParseProperty};
use ini_persist::save::{default_remove, DisplayProperty, SaveProperty};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use strum_macros::{EnumIter, FromRepr};

use crate::game::settings::{display_seconds, parse_seconds, DailyHours, Nudity};

#[derive(Clone, Debug, Deserialize, Serialize, LoadProperty, SaveProperty)]
pub struct BaseGeneralSettings {
    #[serde(rename = "S05")]
    #[ini(rename = "IsBattlEyeEnabled")]
    pub battleye_required: bool,

    #[serde(rename = "S0")]
    #[ini(rename = "PVPEnabled")]
    pub pvp_enabled: bool,

    #[serde(rename = "S30")]
    #[ini(rename = "CombatModeModifier")]
    pub mode_modifier: CombatModeModifier,

    #[serde(rename = "Su")]
    #[ini(rename = "ServerCommunity")]
    pub community: Community,

    #[serde(rename = "S20")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ini(rename = "MaxAllowedPing")]
    pub max_ping: Option<u32>,

    #[serde(rename = "Sx", default = "default_clan_size")]
    #[ini(rename = "clanMaxSize")]
    pub max_clan_size: u16,

    #[serde(rename = "S2", default)]
    #[ini(rename = "CanDamagePlayerOwnedStructures")]
    pub raid_enabled: bool,

    #[serde(rename = "S25")]
    #[ini(rename = "RestrictPVPBuildingDamageTime")]
    pub raid_restricted: bool,

    #[serde(flatten, with = "raid_hours_serde")]
    #[ini(rename = "PVPBuildingDamage")]
    pub raid_hours: DailyHours,
}

impl Default for BaseGeneralSettings {
    fn default() -> Self {
        Self {
            battleye_required: false,
            pvp_enabled: false,
            mode_modifier: Default::default(),
            community: Default::default(),
            max_ping: None,
            max_clan_size: default_clan_size(),
            raid_enabled: false,
            raid_restricted: false,
            raid_hours: Default::default(),
        }
    }
}

#[derive(Debug, Clone, LoadProperty, SaveProperty)]
pub struct GeneralSettings {
    #[ini(flatten)]
    pub base: BaseGeneralSettings,

    #[ini(rename = "ServerMessageOfTheDay")]
    pub motd: String,

    #[ini(rename = "ServerPassword")]
    pub server_password: String,

    #[ini(rename = "AdminPassword")]
    pub admin_password: String,

    #[ini(rename = "RestrictPVPTime")]
    pub pvp_restricted: bool,

    #[ini(rename = "PVP")]
    pub pvp_hours: DailyHours,

    #[ini(rename = "DynamicBuildingDamage")]
    pub dbd_enabled: bool,

    #[ini(rename = "DynamicBuildingDamagePeriod", parse_with = parse_seconds, remove_with = default_remove, display_with = display_seconds)]
    pub dbd_period: TimeDelta,

    #[ini(rename = "NoOwnership")]
    pub no_ownership: bool,

    #[ini(rename = "ContainersIgnoreOwnership")]
    pub containers_ignore_ownership: bool,

    #[ini(rename = "EnableSandStorm")]
    pub sandstorm_enabled: bool,

    #[ini(rename = "EnableClanMarkers")]
    pub clan_markers_enabled: bool,

    #[ini(rename = "CoopTetheringLimit")]
    pub tether_distance: f64,

    #[ini(rename = "MaxNudity")]
    pub max_nudity: Nudity,

    #[ini(rename = "serverVoiceChat", parse_with = true_if_non_zero, display_with = one_if_true)]
    pub voice_chat_enabled: bool,

    #[ini(rename = "EnableWhitelist")]
    pub enforce_whitelist: bool,

    #[ini(rename = "DisableLandclaimNotifications")]
    pub claim_popup_disabled: bool,

    #[ini(rename = "EventLogCauserPrivacy")]
    pub log_privacy: EventLogPrivacy,

    #[ini(rename = "AllowFamilySharedAccount")]
    pub family_share_allowed: bool,

    #[ini(rename = "HealthbarVisibilityDistance")]
    pub healthbar_distance: f64,

    #[ini(rename = "ShowOnlinePlayers")]
    pub online_info_visibility: OnlinePlayerInfoVisibility,
}

impl Deref for GeneralSettings {
    type Target = BaseGeneralSettings;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for GeneralSettings {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            base: Default::default(),
            motd: String::new(),
            server_password: String::new(),
            admin_password: String::new(),
            pvp_restricted: false,
            pvp_hours: DailyHours::new(),
            dbd_enabled: false,
            dbd_period: TimeDelta::try_seconds(1800).unwrap(),
            no_ownership: false,
            containers_ignore_ownership: true,
            sandstorm_enabled: true,
            clan_markers_enabled: true,
            tether_distance: 52000.0,
            max_nudity: Nudity::None,
            voice_chat_enabled: true,
            enforce_whitelist: false,
            claim_popup_disabled: true,
            log_privacy: EventLogPrivacy::Admins,
            family_share_allowed: true,
            healthbar_distance: 15000.0,
            online_info_visibility: OnlinePlayerInfoVisibility::ShowAll,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatModeModifier {
    Conflict,
    Other(u8),
}

impl Default for CombatModeModifier {
    fn default() -> Self {
        Self::Other(0)
    }
}

impl CombatModeModifier {
    pub fn from_repr(value: u8) -> Self {
        match value {
            1 => Self::Conflict,
            _ => Self::Other(value),
        }
    }

    pub fn to_repr(self) -> u8 {
        match self {
            Self::Conflict => 1,
            Self::Other(repr) => repr,
        }
    }
}

impl Serialize for CombatModeModifier {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to_repr().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for CombatModeModifier {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let repr = u8::deserialize(deserializer)?;
        Ok(CombatModeModifier::from_repr(repr))
    }
}

impl ParseProperty for CombatModeModifier {
    fn parse(text: &str) -> ini_persist::Result<Self> {
        Ok(Self::from_repr(u8::parse(text)?))
    }
}

impl DisplayProperty for CombatModeModifier {
    fn display(&self) -> String {
        self.to_repr().to_string()
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    Deserialize_repr,
    Serialize_repr,
    LoadProperty,
    SaveProperty,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
)]
#[repr(u8)]
#[ini(repr)]
pub enum Community {
    Unspecified,
    Purist,
    Relaxed,
    Hardcore,
    RolePlaying,
    Experimental,
}

impl Default for Community {
    fn default() -> Self {
        Self::Unspecified
    }
}

#[derive(Debug, Clone, Copy, EnumIter, FromRepr, LoadProperty, SaveProperty)]
#[repr(u8)]
#[ini(repr)]
pub enum EventLogPrivacy {
    Everybody,
    Admins,
    Nobody,
}

#[derive(Debug, Clone, Copy, EnumIter, FromRepr, LoadProperty, SaveProperty)]
#[repr(u8)]
#[ini(repr)]
pub enum OnlinePlayerInfoVisibility {
    ShowAll,
    ShowClan,
    ShowNobody,
}

#[inline(always)]
const fn default_clan_size() -> u16 {
    30
}

fn true_if_non_zero(value: &str) -> ini_persist::Result<bool> {
    Ok(u8::parse(value)? != 0)
}

fn one_if_true(value: &bool) -> String {
    match value {
        false => "0".to_string(),
        true => "1".to_string(),
    }
}

mod raid_hours_serde {
    use std::collections::HashMap;

    use chrono::Weekday;
    use serde::de::{MapAccess, Visitor};
    use serde::ser::SerializeMap;

    use crate::game::settings::{HourMinute, Hours};

    use super::DailyHours;

    pub fn serialize<S: serde::Serializer>(
        hours: &DailyHours,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(hours.len() * 3))?;
        for (
            day,
            Hours {
                start: HourMinute(start),
                end: HourMinute(end),
            },
        ) in hours.iter()
        {
            let offset = *day as isize;
            map.serialize_entry(&format!("S{}", 92 + offset), start)?;
            map.serialize_entry(&format!("S{}", 99 + offset), end)?;
            map.serialize_entry(&format!("S{}", 106 + offset), &true)?;
        }
        map.end()
    }

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<DailyHours, D::Error> {
        fn hours_entry_index(key: &str) -> Option<(Weekday, usize)> {
            match key {
                "S92" => Some((Weekday::Mon, 0)),
                "S93" => Some((Weekday::Tue, 0)),
                "S94" => Some((Weekday::Wed, 0)),
                "S95" => Some((Weekday::Thu, 0)),
                "S96" => Some((Weekday::Fri, 0)),
                "S97" => Some((Weekday::Sat, 0)),
                "S98" => Some((Weekday::Sun, 0)),
                "S99" => Some((Weekday::Mon, 1)),
                "S100" => Some((Weekday::Tue, 1)),
                "S101" => Some((Weekday::Wed, 1)),
                "S102" => Some((Weekday::Thu, 1)),
                "S103" => Some((Weekday::Fri, 1)),
                "S104" => Some((Weekday::Sat, 1)),
                "S105" => Some((Weekday::Sun, 1)),
                _ => None,
            }
        }

        fn hours_inclusion_key(key: &str) -> Option<Weekday> {
            match key {
                "S106" => Some(Weekday::Mon),
                "S107" => Some(Weekday::Tue),
                "S108" => Some(Weekday::Wed),
                "S109" => Some(Weekday::Thu),
                "S110" => Some(Weekday::Fri),
                "S111" => Some(Weekday::Sat),
                "S112" => Some(Weekday::Sun),
                _ => None,
            }
        }

        struct MapVisitor;

        impl<'de> Visitor<'de> for MapVisitor {
            type Value = DailyHours;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("map")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut hours = HashMap::new();
                let mut defined_days = Vec::new();

                while let Some(key) = map.next_key::<&str>()? {
                    if let Some((day, idx)) = hours_entry_index(key) {
                        hours.entry(day).or_insert([0, 0])[idx] = map.next_value()?;
                    } else if let Some(day) = hours_inclusion_key(key) {
                        if map.next_value::<bool>()? {
                            defined_days.push(day);
                        }
                    }
                }

                Ok(defined_days
                    .into_iter()
                    .filter_map(|day| {
                        hours.get(&day).map(|values| {
                            (
                                day,
                                Hours {
                                    start: values[0].into(),
                                    end: values[1].into(),
                                },
                            )
                        })
                    })
                    .collect())
            }
        }

        deserializer.deserialize_map(MapVisitor)
    }
}
