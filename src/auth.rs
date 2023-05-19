use std::borrow::Borrow;
use std::collections::HashMap;
use std::hash::Hash;

use anyhow::Result;
use lazy_static::lazy_static;
use network_interface::NetworkInterface;
use nom::IResult;
use regex::Regex;

pub mod playfab;

use crate::parser_utils::{extract_value, parse_map, parse_quoted, ParserError};
use crate::workers::TaskState;

pub type Capability = Result<()>;

#[derive(Debug)]
pub struct AuthState {
    pub platform_user: Result<PlatformUser>,
    pub fls_account: TaskState<Result<Account>>,
    pub online_capability: TaskState<Capability>,
    pub sp_capability: TaskState<Capability>,
}

#[derive(Debug, Clone)]
pub struct Account {
    pub master_id: String,
    #[allow(dead_code)]
    pub title_id: String,
    pub display_name: String,
    pub platform_id: String,
}

#[derive(Debug, Clone)]
pub struct PlatformUser {
    pub id: String,
    pub display_name: String,
}

#[derive(Debug, Clone)]
pub struct CachedUser {
    pub account: Account,
    #[allow(dead_code)]
    pub user_token: String,
}

impl CachedUser {
    pub fn new(account: Account) -> Self {
        Self {
            account,
            user_token: calculate_user_token(),
        }
    }

    pub fn parse(input: &str) -> Result<Self> {
        Ok(extract_value(parse_cached_user_impl(input))
            .map_err(|err| ParserError::from_err(input, err))?)
    }

    pub fn to_string(&self) -> String {
        // TODO: Escape values
        format!(
            "({}=\"{}\",{}=\"{}\",{}=\"{}\",{}=\"{}\",{}=\"{}\")",
            KEY_MASTER_ACCOUNT_ID,
            &self.account.master_id,
            KEY_TITLE_PLAYER_ID,
            &self.account.title_id,
            KEY_PLATFORM_ID,
            &self.account.platform_id,
            KEY_TITLE_DISPLAY_NAME,
            &self.account.display_name,
            KEY_USER_TOKEN,
            &self.user_token,
        )
    }
}

pub struct CachedUsers {
    by_platform_id: HashMap<String, CachedUser>,
}

impl CachedUsers {
    pub fn new() -> Self {
        Self {
            by_platform_id: HashMap::new(),
        }
    }

    pub fn insert(&mut self, user: CachedUser) {
        self.by_platform_id
            .insert(user.account.platform_id.clone(), user);
    }

    pub fn by_platform_id<Q>(&self, platform_id: &Q) -> Option<&CachedUser>
    where
        String: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.by_platform_id.get(platform_id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &CachedUser> {
        self.by_platform_id.values()
    }
}

const KEY_MASTER_ACCOUNT_ID: &str = "MasterAccountId";
const KEY_TITLE_PLAYER_ID: &str = "TitlePlayerId";
const KEY_TITLE_DISPLAY_NAME: &str = "TitleDisplayName";
const KEY_PLATFORM_ID: &str = "PlatformId";
const KEY_USER_TOKEN: &str = "UserToken";

lazy_static! {
    static ref RE_NON_HEX: Regex = Regex::new(r"[^0-9a-zA-Z]").unwrap();
}

fn parse_cached_user_impl(input: &str) -> IResult<&str, CachedUser> {
    let (input, map) = parse_map(input)?;
    let master_account_id = map
        .get(KEY_MASTER_ACCOUNT_ID)
        .and_then(|value| extract_value(parse_quoted(value)).ok())
        .unwrap_or_default();
    let title_player_id = map
        .get(KEY_TITLE_PLAYER_ID)
        .and_then(|value| extract_value(parse_quoted(value)).ok())
        .unwrap_or_default();
    let title_display_name = map
        .get(KEY_TITLE_DISPLAY_NAME)
        .and_then(|value| extract_value(parse_quoted(value)).ok())
        .unwrap_or_default();
    let platform_id = map
        .get(KEY_PLATFORM_ID)
        .and_then(|value| extract_value(parse_quoted(value)).ok())
        .unwrap_or_default();
    let user_token = map
        .get(KEY_USER_TOKEN)
        .and_then(|value| extract_value(parse_quoted(value)).ok())
        .unwrap_or_default();
    let user = CachedUser {
        account: Account {
            master_id: master_account_id,
            title_id: title_player_id,
            display_name: title_display_name,
            platform_id,
        },
        user_token,
    };
    Ok((input, user))
}

fn calculate_user_token() -> String {
    use network_interface::NetworkInterfaceConfig;

    let intfs = NetworkInterface::show().unwrap();
    let mac = intfs
        .iter()
        .filter_map(|intf| intf.mac_addr.as_deref())
        .next()
        .unwrap_or_default();
    let mac = RE_NON_HEX.replace_all(mac, "").to_ascii_lowercase();

    format!("{:x}", md5::compute(mac))
}
