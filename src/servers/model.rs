use std::net::IpAddr;

use serde::Deserialize;
use serde_repr::Deserialize_repr;

#[derive(Clone, Copy, Debug, Deserialize_repr, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Region {
    America = 1,
    Asia = 2,
    EU = 0,
    Japan = 5,
    LATAM = 4,
    Oceania = 3,
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
    Conflict,
    #[serde(other)]
    Other,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Mode {
    PVE,
    PVEC,
    PVP,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Server {
    #[serde(rename = "EXTERNAL_SERVER_UID")]
    pub id: String,

    #[serde(rename = "Name", default)]
    pub name: String,

    #[serde(rename = "MapName")]
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

    #[serde()]
    pub ip: IpAddr,

    #[serde(rename = "Port")]
    pub port: u32,

    #[serde(rename = "buildId")]
    pub build_id: u64,
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

    pub fn host(&self) -> String {
        format!("{}:{}", self.ip, self.port)
    }
}
