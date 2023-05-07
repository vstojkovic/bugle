use std::borrow::Borrow;
use std::collections::HashMap;
use std::hash::Hash;

use anyhow::Result;
use nom::IResult;

use crate::parser_utils::{extract_value, parse_map, parse_quoted, ParserError};

pub struct CachedUser {
    pub master_account_id: String,
    #[allow(dead_code)]
    pub title_player_id: String,
    #[allow(dead_code)]
    pub title_display_name: String,
    pub platform_id: String,
    #[allow(dead_code)]
    pub user_token: String,
}

impl CachedUser {
    pub fn parse(input: &str) -> Result<Self> {
        Ok(extract_value(parse_cached_user_impl(input))
            .map_err(|err| ParserError::from_err(input, err))?)
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
        self.by_platform_id.insert(user.platform_id.clone(), user);
    }

    pub fn by_platform_id<Q>(&self, platform_id: &Q) -> Option<&CachedUser>
    where
        String: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.by_platform_id.get(platform_id)
    }
}

const KEY_MASTER_ACCOUNT_ID: &str = "MasterAccountId";
const KEY_TITLE_PLAYER_ID: &str = "TitlePlayerId";
const KEY_TITLE_DISPLAY_NAME: &str = "TitleDisplayName";
const KEY_PLATFORM_ID: &str = "PlatformId";
const KEY_USER_TOKEN: &str = "UserToken";

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
        master_account_id,
        title_player_id,
        title_display_name,
        platform_id,
        user_token,
    };
    Ok((input, user))
}
