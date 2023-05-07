use std::borrow::Cow;
use std::collections::HashSet;
use std::net::IpAddr;
use std::str::FromStr;

use anyhow::Result;
use lazy_static::lazy_static;
use nom::IResult;
use regex::Regex;

use crate::parser_utils::{extract_value, parse_hex, parse_map, parse_quoted, ParserError};

use super::Server;

pub struct FavoriteServer {
    pub name: Option<String>,
    pub ip: Option<IpAddr>,
    pub port: Option<u32>,
    pub id: Option<String>,
}

impl FavoriteServer {
    pub fn from_server(server: &Server) -> Self {
        Self {
            name: Some(server.name.clone()),
            ip: Some(*server.ip()),
            port: Some(server.port),
            id: Some(server.id.clone()),
        }
    }

    pub fn parse(input: &str) -> Result<Self> {
        Ok(extract_value(parse_favorite_impl(input))
            .map_err(|err| ParserError::from_err(input, err))?)
    }

    pub fn to_string(&self) -> String {
        use std::fmt::Write;

        let mut result = "(".to_string();

        if let Some(name) = &self.name {
            write!(&mut result, "{}=\"{}\",", KEY_NAME, escape_string(name)).unwrap();
        }

        if let Some(ip) = &self.ip {
            write!(&mut result, "{}=\"{}\",", KEY_IP, ip).unwrap();
        }

        if let Some(port) = &self.port {
            write!(&mut result, "{}={},", KEY_PORT, port).unwrap();
        }

        if let Some(id) = &self.id {
            write!(&mut result, "{}={},", KEY_ID, id).unwrap();
        }

        result.pop();
        result.push(')');

        result
    }
}

pub struct FavoriteServers {
    by_addr: HashSet<(IpAddr, u32)>,
    by_id: HashSet<String>,
}

impl FavoriteServers {
    pub fn new() -> Self {
        Self {
            by_addr: HashSet::new(),
            by_id: HashSet::new(),
        }
    }

    pub fn insert(&mut self, favorite: FavoriteServer) -> bool {
        if let (Some(ip), Some(port)) = (favorite.ip, favorite.port) {
            self.by_addr.insert((ip, port))
        } else if let Some(id) = favorite.id {
            self.by_id.insert(id)
        } else {
            false
        }
    }

    pub fn contains(&self, server: &Server) -> bool {
        self.by_addr.contains(&(*server.ip(), server.port)) || self.by_id.contains(&server.id)
    }
}

lazy_static! {
    static ref RE_ESCAPABLE: Regex = Regex::new(r#"['"\\]"#).unwrap();
}

const KEY_NAME: &str = "ServerName";
const KEY_IP: &str = "IPAddress";
const KEY_PORT: &str = "Port";
const KEY_ID: &str = "UID";

fn escape_string(s: &str) -> Cow<str> {
    RE_ESCAPABLE.replace_all(s, "\\$0")
}

fn parse_favorite_impl(input: &str) -> IResult<&str, FavoriteServer> {
    let (input, map) = parse_map(input)?;
    let name = map
        .get(KEY_NAME)
        .and_then(|value| extract_value(parse_quoted(value)).ok());
    let ip = map
        .get(KEY_IP)
        .and_then(|value| extract_value(parse_quoted(value)).ok())
        .and_then(|s| IpAddr::from_str(&s).ok());
    let port = map
        .get(KEY_PORT)
        .and_then(|value| u32::from_str_radix(value, 10).ok());
    let id = map
        .get(KEY_ID)
        .and_then(|value| extract_value(parse_hex(value, 32)).ok())
        .map(str::to_string);
    let favorite = FavoriteServer { name, ip, port, id };
    Ok((input, favorite))
}
