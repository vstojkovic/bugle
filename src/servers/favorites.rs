use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::str::FromStr;

use lazy_static::lazy_static;
use nom::branch::alt;
use nom::bytes::complete::{escaped, escaped_transform, is_not, take_while_m_n};
use nom::character::complete::{anychar, char, multispace0};
use nom::combinator::{all_consuming, map, recognize};
use nom::multi::{fold_many0, many1_count};
use nom::sequence::{delimited, preceded};
use nom::{AsChar, IResult};
use regex::Regex;

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

    pub fn parse(input: &str) -> Result<FavoriteServer, ()> {
        extract_value(parse_favorite_impl(input)).map_err(|_| ())
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

fn parse_favorite_impl(input: &str) -> IResult<&str, FavoriteServer, ()> {
    let (input, map) = all_consuming(delimited(char('('), parse_map, char(')')))(input)?;
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

fn parse_map(input: &str) -> IResult<&str, HashMap<&str, &str>, ()> {
    let (input, (key, value)) = parse_entry(input)?;
    fold_many0(
        preceded(char(','), parse_entry),
        move || {
            let mut map = HashMap::new();
            map.insert(key, value);
            map
        },
        |mut map, (key, value)| {
            map.entry(key).or_insert(value);
            map
        },
    )(input)
}

fn parse_entry(input: &str) -> IResult<&str, (&str, &str), ()> {
    let (input, key) = parse_key(input)?;
    let (input, _) = char('=')(input)?;
    let (input, value) = parse_value(input)?;
    Ok((input, (key, value)))
}

fn parse_key(input: &str) -> IResult<&str, &str, ()> {
    map(preceded(multispace0, is_not("=")), str::trim_end)(input)
}

fn parse_value(input: &str) -> IResult<&str, &str, ()> {
    map(
        preceded(
            multispace0,
            recognize(many1_count(alt((
                is_not("\",)"),
                delimited(char('"'), escaped(is_not("\\\""), '\\', anychar), char('"')),
            )))),
        ),
        str::trim_end,
    )(input)
}

fn parse_quoted(input: &str) -> IResult<&str, String, ()> {
    delimited(
        char('"'),
        escaped_transform(is_not("\\\""), '\\', anychar),
        char('"'),
    )(input)
}

fn parse_hex(input: &str, len: usize) -> IResult<&str, &str, ()> {
    all_consuming(take_while_m_n(len, len, AsChar::is_hex_digit))(input)
}

fn extract_value<V>(result: IResult<&str, V, ()>) -> Result<V, nom::Err<()>> {
    result.map(|(_, value)| value)
}
