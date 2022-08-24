use std::net::IpAddr;
use std::time::Duration;

use bitflags::bitflags;
use serde::Deserialize;
use serde_repr::Deserialize_repr;
use strum_macros::{EnumIter, FromRepr};

#[derive(
    Clone, Copy, Debug, Deserialize_repr, EnumIter, FromRepr, PartialEq, Eq, PartialOrd, Ord,
)]
#[repr(u8)]
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

#[derive(Clone, Copy, Debug, Deserialize_repr, PartialEq, Eq)]
#[repr(u8)]
pub enum Kind {
    Conflict = 1,
    #[serde(other)]
    Other,
}

#[derive(Clone, Copy, Debug, EnumIter, FromRepr, PartialEq, Eq, PartialOrd, Ord)]
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

    #[serde(rename = "S17")]
    pub mods: Option<String>,

    #[serde(skip)]
    pub connected_players: Option<usize>,

    #[serde(skip)]
    pub age: Option<Duration>,

    #[serde(skip)]
    pub ping: Option<Duration>,

    #[serde(skip)]
    pub favorite: bool,

    #[serde(skip)]
    pub validity: Validity,
}

impl Server {
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
}
