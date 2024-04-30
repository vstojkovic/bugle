use std::cmp::Ordering;
use std::net::{IpAddr, SocketAddr};
use std::ops::{Deref, DerefMut};
use std::time::Duration;

use bitflags::bitflags;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use strum_macros::{AsRefStr, EnumIter, EnumString, FromRepr};
use uuid::Uuid;

use crate::game::settings::server::{
    BaseCombatSettings, BaseCraftingSettings, BaseDaylightSettings, BaseGeneralSettings,
    BaseHarvestingSettings, BaseProgressionSettings, BaseSurvivalSettings, CombatModeModifier,
};
use crate::net::{is_valid_ip, is_valid_port};

use super::FavoriteServers;

#[derive(Clone, Debug)]
pub struct Server {
    data: ServerData,
    pub ip: IpAddr,
    pub connected_players: Option<usize>,
    pub age: Option<Duration>,
    pub ping: Option<Duration>,
    pub waiting_for_pong: bool,
    pub favorite: bool,
    pub saved_id: Option<Uuid>,
    pub validity: Validity,
    pub merged: bool,
    pub tombstone: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ServerData {
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

    #[serde(rename = "Sy")]
    pub region: Region,

    #[serde(rename = "maxplayers")]
    pub max_players: usize,

    #[serde(rename = "ip")]
    pub reported_ip: IpAddr,

    #[serde(rename = "kdsObservedServerAddress")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_ip: Option<IpAddr>,

    #[serde(rename = "Port")]
    pub port: u32,

    #[serde(rename = "buildId")]
    pub build_id: u32,

    #[serde(rename = "S17")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mods: Option<String>,

    #[serde(flatten)]
    pub general: BaseGeneralSettings,

    #[serde(flatten)]
    pub progression: BaseProgressionSettings,

    #[serde(flatten)]
    pub daylight: BaseDaylightSettings,

    #[serde(flatten)]
    pub survival: BaseSurvivalSettings,

    #[serde(flatten)]
    pub combat: BaseCombatSettings,

    #[serde(flatten)]
    pub harvesting: BaseHarvestingSettings,

    #[serde(flatten)]
    pub crafting: BaseCraftingSettings,
}

impl Deref for Server {
    type Target = ServerData;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for Server {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl Server {
    pub fn new(data: ServerData) -> Self {
        let ip = if is_valid_ip(&data.reported_ip) || data.observed_ip.is_none() {
            data.reported_ip
        } else {
            data.observed_ip.unwrap()
        };

        let mut server = Server {
            data,
            ip,
            connected_players: None,
            age: None,
            ping: None,
            waiting_for_pong: false,
            favorite: false,
            saved_id: None,
            validity: Validity::VALID,
            merged: false,
            tombstone: false,
        };

        if server.name.is_empty() {
            server.name = server.host();
        }

        if !is_valid_ip(&server.ip) {
            server.validity.insert(Validity::INVALID_ADDR);
        }
        if !is_valid_port(server.port) {
            server.validity.insert(Validity::INVALID_PORT);
        }

        server
    }

    pub fn validate_build(&mut self, build_id: u32) {
        if self.build_id != build_id {
            self.validity.insert(Validity::INVALID_BUILD);
        }
    }

    pub fn check_favorites(&mut self, favorites: &FavoriteServers) {
        self.favorite = favorites.contains(&self);
    }

    pub fn prepare_for_ping(&mut self) {
        self.waiting_for_pong = self.is_valid();
    }

    pub fn host(&self) -> String {
        format!("{}:{}", self.ip, self.port)
    }

    pub fn game_addr(&self) -> Option<SocketAddr> {
        if self.is_valid() {
            Some(SocketAddr::new(self.ip, self.port as _))
        } else {
            None
        }
    }

    pub fn ping_addr(&self) -> Option<SocketAddr> {
        if self.is_valid() {
            Some(SocketAddr::new(self.ip, (self.port + 1) as _))
        } else {
            None
        }
    }

    pub fn is_valid(&self) -> bool {
        self.validity.is_valid()
    }

    pub fn is_saved(&self) -> bool {
        self.saved_id.is_some()
    }

    pub fn preference(&self, rhs: &Self) -> Ordering {
        match rhs.is_saved().cmp(&self.is_saved()) {
            Ordering::Equal => rhs.favorite.cmp(&self.favorite),
            ord @ _ => ord,
        }
    }

    pub fn similarity(&self, rhs: &Self) -> Similarity {
        let mut score = 0;
        if self.id == rhs.id {
            score += 6;
        }
        if self.name == rhs.name {
            score += 5;
        }
        if (self.ip == rhs.ip) && (self.port == rhs.port) {
            score += 3;
        }
        if self.map == rhs.map {
            score += 2;
        }
        Similarity(score)
    }

    pub fn merge_from(&mut self, source: &mut Self) {
        let saved_id = self.saved_id;
        self.clone_from(source);
        self.saved_id = saved_id;
        self.merged = true;
        source.tombstone = true;
    }
}

impl<'de> Deserialize<'de> for Server {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let data = ServerData::deserialize(deserializer)?;
        Ok(Server::new(data))
    }
}

impl Serialize for Server {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.data.serialize(serializer)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Similarity(isize);

impl Similarity {
    pub fn satisfies(&self, confidence: Confidence) -> bool {
        self.0 >= (confidence as isize)
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(isize)]
pub enum Confidence {
    #[allow(unused)]
    Low = 6,

    High = 10,

    #[allow(unused)]
    Full = 16,
}

impl ServerData {
    pub fn mode(&self) -> Mode {
        if self.general.pvp_enabled {
            match self.general.mode_modifier {
                CombatModeModifier::Conflict => Mode::PVEC,
                CombatModeModifier::Other(_) => Mode::PVP,
            }
        } else {
            Mode::PVE
        }
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
    Serialize_repr,
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

#[derive(Clone, Copy, Debug, Deserialize_repr, Serialize_repr, PartialEq, Eq)]
#[repr(u8)]
pub enum Ownership {
    Private,
    Official,
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
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, AsRefStr, EnumIter, EnumString, FromRepr)]
#[strum(ascii_case_insensitive)]
#[repr(u8)]
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

#[derive(Clone, Debug, Default)]
pub struct Filter {
    pub name: String,
    pub map: String,
    pub type_filter: TypeFilter,
    pub mode: Option<Mode>,
    pub region: Option<Region>,
    pub battleye_required: Option<bool>,
    pub include_invalid: bool,
    pub exclude_password_protected: bool,
    pub mods: Option<bool>,
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
