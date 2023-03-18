use std::collections::HashMap;
use std::net::IpAddr;
use std::time::Duration;

use bitflags::bitflags;
use serde::Deserialize;
use serde_repr::Deserialize_repr;
use strum_macros::{AsRefStr, EnumIter, EnumString, FromRepr};

use crate::net::{is_valid_ip, is_valid_port};

use super::FavoriteServers;

#[derive(Clone, Debug, Deserialize)]
pub struct Server {
    #[serde(rename = "EXTERNAL_SERVER_UID")]
    pub id: String,

    #[serde(rename = "Name", default)]
    pub name: String,

    #[serde(rename = "MapName", default)]
    pub map: String,

    #[serde(rename = "private")]
    pub password_protected: bool,

    #[serde(rename = "CSF")]
    pub ownership: Ownership,

    #[serde(rename = "S05")]
    pub battleye_required: bool,

    #[serde(rename = "Sy")]
    pub region: Region,

    #[serde(rename = "maxplayers")]
    pub max_players: usize,

    #[serde(rename = "S0")]
    pub pvp_enabled: bool,

    #[serde(rename = "S30")]
    pub kind: Kind,

    #[serde(rename = "ip")]
    pub reported_ip: IpAddr,

    #[serde(rename = "kdsObservedServerAddress")]
    pub observed_ip: Option<IpAddr>,

    #[serde(skip, default = "crate::net::default_ip")]
    pub ip: IpAddr,

    #[serde(rename = "Port")]
    pub port: u32,

    #[serde(rename = "buildId")]
    pub build_id: u32,

    #[serde(rename = "Su")]
    pub community: Community,

    #[serde(rename = "S17")]
    pub mods: Option<String>,

    #[serde(rename = "S20")]
    pub max_ping: Option<u32>, // TODO: What's the default?

    #[serde(rename = "Sx")]
    pub max_clan_size: Option<u16>, // TODO: What's the default?

    #[serde(rename = "Sz")]
    pub xp_rate_mult: Multiplier,

    #[serde(flatten)]
    pub daylight: DaylightSettings,

    #[serde(flatten)]
    pub survival: SurvivalSettings,

    #[serde(flatten)]
    pub combat: CombatSettings,

    #[serde(flatten)]
    pub harvesting: HarvestingSettings,

    #[serde(flatten)]
    pub crafting: CraftingSettings,

    #[serde(flatten)]
    pub raid_hours: RaidHours,

    #[serde(skip)]
    pub connected_players: Option<usize>,

    #[serde(skip)]
    pub age: Option<Duration>,

    #[serde(skip)]
    pub ping: Option<Duration>,

    #[serde(skip)]
    pub waiting_for_pong: bool,

    #[serde(skip)]
    pub favorite: bool,

    #[serde(skip)]
    pub validity: Validity,
}

pub struct DeserializationContext<'dc> {
    pub build_id: u32,
    pub favorites: &'dc FavoriteServers,
}

impl Server {
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
        ctx: &DeserializationContext,
    ) -> Result<Self, D::Error> {
        let mut server = <Server as Deserialize>::deserialize(deserializer)?;

        server.waiting_for_pong = true;

        server.ip = if is_valid_ip(&server.reported_ip) || server.observed_ip.is_none() {
            server.reported_ip
        } else {
            server.observed_ip.unwrap()
        };

        if server.name.is_empty() {
            server.name = server.host();
        }

        server.favorite = ctx.favorites.contains(&server);

        if server.build_id != ctx.build_id {
            server.validity.insert(Validity::INVALID_BUILD);
        }
        if !is_valid_ip(server.ip()) {
            server.validity.insert(Validity::INVALID_ADDR);
        }
        if !is_valid_port(server.port) {
            server.validity.insert(Validity::INVALID_PORT);
        }

        Ok(server)
    }

    pub fn mode(&self) -> Mode {
        if self.pvp_enabled {
            match self.kind {
                Kind::Conflict => Mode::PVEC,
                Kind::Other => Mode::PVP,
            }
        } else {
            Mode::PVE
        }
    }

    pub fn ip(&self) -> &IpAddr {
        &self.ip
    }

    pub fn host(&self) -> String {
        format!("{}:{}", self.ip(), self.port)
    }

    pub fn is_valid(&self) -> bool {
        self.validity.is_valid()
    }

    pub fn is_official(&self) -> bool {
        self.ownership == Ownership::Official
    }

    pub fn is_modded(&self) -> bool {
        self.mods.is_some()
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    Deserialize_repr,
    AsRefStr,
    EnumIter,
    EnumString,
    FromRepr,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
)]
#[repr(u8)]
#[strum(ascii_case_insensitive)]
pub enum Region {
    EU,
    America,
    Asia,
    Oceania,
    LATAM,
    Japan,
}

#[derive(Clone, Copy, Debug, Deserialize_repr, PartialEq, Eq)]
#[repr(u8)]
pub enum Ownership {
    Private,
    Official,
}

#[derive(Clone, Copy, Debug, Deserialize_repr, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Community {
    Unspecified,
    Purist,
    Relaxed,
    Hardcore,
    RolePlaying,
    Experimental,
}

#[derive(Clone, Copy, Debug, Deserialize_repr, PartialEq, Eq)]
#[repr(u8)]
pub enum Kind {
    Conflict = 1,
    #[serde(other)]
    Other,
}

#[derive(
    Clone, Copy, Debug, AsRefStr, EnumIter, EnumString, FromRepr, PartialEq, Eq, PartialOrd, Ord,
)]
#[strum(ascii_case_insensitive)]
pub enum Mode {
    PVE,
    PVEC,
    PVP,
}

bitflags! {
    #[derive(Default)]
    pub struct Validity: u8 {
        const VALID = 0;
        const INVALID_BUILD = 1;
        const INVALID_ADDR = 2;
        const INVALID_PORT = 4;
    }
}

impl Validity {
    pub fn is_valid(self) -> bool {
        self == Self::VALID
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(transparent)]
pub struct Multiplier(f64);

impl Default for Multiplier {
    fn default() -> Self {
        Self(1.0)
    }
}

impl Multiplier {
    pub fn to_string(&self) -> String {
        format!("{:.2}", self.0)
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct DaylightSettings {
    #[serde(rename = "Sb", default)]
    pub day_cycle_speed_mult: Multiplier,

    #[serde(rename = "Sg", default)]
    pub dawn_dusk_speed_mult: Multiplier,

    #[serde(rename = "Sd", default = "default_true")]
    pub use_catch_up_time: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SurvivalSettings {
    #[serde(rename = "Sj", default)]
    pub stamina_cost_mult: Multiplier,

    #[serde(rename = "S21")]
    pub idle_thirst_mult: Multiplier,

    #[serde(rename = "S22")]
    pub active_thirst_mult: Multiplier,

    #[serde(rename = "S23")]
    pub idle_hunger_mult: Multiplier,

    #[serde(rename = "S24")]
    pub active_hunger_mult: Multiplier,

    #[serde(rename = "S7")]
    pub drop_items_on_death: bool,

    #[serde(rename = "Sa")]
    pub anyone_can_loot_corpse: bool,

    #[serde(rename = "S5", default = "default_true")]
    pub offline_chars_in_world: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CombatSettings {
    #[serde(rename = "S6", default)]
    pub durability_mult: Multiplier,

    #[serde(rename = "So")]
    thrall_wakeup_time_secs: Option<f64>,
}

impl CombatSettings {
    pub fn thrall_wakeup_time_secs(&self) -> f64 {
        self.thrall_wakeup_time_secs.unwrap_or(1800.0)
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct HarvestingSettings {
    #[serde(rename = "Sl")]
    pub harvest_amount_mult: Multiplier,

    #[serde(rename = "Sk", default)]
    pub item_spoil_rate_mult: Multiplier,

    #[serde(rename = "Sm", default)]
    pub rsrc_respawn_speed_mult: Multiplier,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CraftingSettings {
    #[serde(rename = "S8")]
    pub crafting_time_mult: Multiplier,

    #[serde(rename = "S4")]
    pub thrall_crafting_time_mult: Multiplier,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Weekday {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}

#[derive(Clone, Copy, Debug)]
pub struct RaidTime(u16);

impl RaidTime {
    pub fn hours(&self) -> u8 {
        (self.0 / 100) as _
    }

    pub fn minutes(&self) -> u8 {
        (self.0 % 100) as _
    }

    pub fn to_string(&self) -> String {
        format!("{:02}:{:02}", self.hours(), self.minutes())
    }
}

impl From<u16> for RaidTime {
    fn from(time: u16) -> Self {
        Self(time)
    }
}

#[derive(Clone, Debug)]
pub struct RaidHours {
    hours: HashMap<Weekday, (RaidTime, RaidTime)>,
}

impl RaidHours {
    pub fn get(&self, day: Weekday) -> Option<&(RaidTime, RaidTime)> {
        self.hours.get(&day)
    }
}

impl<'de> Deserialize<'de> for RaidHours {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::{MapAccess, Visitor};

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
            type Value = RaidHours;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("the Ultimate Answer")
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

                Ok(RaidHours {
                    hours: defined_days
                        .into_iter()
                        .filter_map(|day| {
                            hours
                                .get(&day)
                                .map(|values| (day, (values[0].into(), values[1].into())))
                        })
                        .collect(),
                })
            }
        }

        deserializer.deserialize_map(MapVisitor)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, AsRefStr, EnumIter, EnumString, FromRepr)]
#[strum(ascii_case_insensitive)]
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

#[derive(Clone, Copy, Debug, AsRefStr, EnumIter, EnumString, Hash, PartialEq, Eq)]
#[strum(ascii_case_insensitive)]
pub enum SortKey {
    Name,
    Map,
    Mode,
    Region,
    Players,
    Age,
    Ping,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SortCriteria {
    pub key: SortKey,
    pub ascending: bool,
}

impl Default for SortCriteria {
    fn default() -> Self {
        Self {
            key: SortKey::Name,
            ascending: true,
        }
    }
}

impl SortCriteria {
    pub fn reversed(&self) -> Self {
        Self {
            key: self.key,
            ascending: !self.ascending,
        }
    }
}

#[inline(always)]
const fn default_true() -> bool {
    true
}
