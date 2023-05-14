use anyhow::{anyhow, bail, Result};
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use slog::{debug, error, trace, Logger};

use crate::game::Game;
use crate::net::http_client_builder;

use super::Account;

pub async fn login_with_steam(logger: &Logger, game: &Game, ticket: Vec<u8>) -> Result<Account> {
    debug!(logger, "Fetching FLS account info");

    let request = LoginWithSteamRequest {
        title_id: TITLE_ID.to_string(),
        create_account: false,
        steam_ticket: hex::encode_upper(ticket),
        params: GetPlayerCombinedInfoRequestParams {
            user_account_info: true,
            ..Default::default()
        },
    };

    let json = match post_request(game, "Client/LoginWithSteam", request).await {
        Ok(json) => json,
        Err(err) => {
            error!(logger, "Error fetching FLS response"; "error" => %err);
            return Err(err);
        }
    };

    trace!(logger, "Fetched FLS response"; "json" => %json);

    let response: LoginWithSteamResponse = match parse_response(&json) {
        Ok(response) => response,
        Err(err) => {
            error!(logger, "Error parsing FLS response"; "error" => %err, "response" => %json);
            return Err(err);
        }
    };

    let acct_info = match response.info_result.account_info {
        Some(info) => info,
        None => {
            error!(logger, "Missing account info in FLS response"; "response" => %json);
            bail!("missing account info in FLS response");
        }
    };

    let master_id = acct_info.id;
    let title_id = acct_info.title_info.title_account.id;
    let display_name = display_name_fixup(acct_info.title_info.display_name);
    let platform_id = match acct_info.steam_info {
        Some(info) => info.id,
        None => {
            error!(logger, "Missing Steam info in FLS response"; "response" => %json);
            bail!("missing Steam info in FLS response");
        }
    };

    Ok(Account {
        master_id,
        title_id,
        display_name,
        platform_id,
    })
}

async fn post_request<R: Serialize>(game: &Game, endpoint: &str, request: R) -> Result<Value> {
    let client = make_client(game)?;
    Ok(client
        .post(endpoint_url(endpoint))
        .query(&[("sdk", SDK)])
        .json(&request)
        .send()
        .await?
        .json()
        .await?)
}

fn parse_response<'de, R: Deserialize<'de>>(json: &'de Value) -> Result<R> {
    let json = json
        .as_object()
        .ok_or_else(|| anyhow!("expected a JSON object in response"))?;
    let code = json
        .get("code")
        .ok_or_else(|| anyhow!("cannot find 'code' key in response"))?
        .as_u64()
        .ok_or_else(|| anyhow!("expected a positive number in 'code' key"))?;
    if code != 200 {
        bail!("response code was not 200");
    }
    let data = json
        .get("data")
        .ok_or_else(|| anyhow!("cannot find 'data' key in response"))?;
    Ok(R::deserialize(data)?)
}

fn display_name_fixup(name: String) -> String {
    if name.len() <= 5 {
        name
    } else {
        let split_pos = name.len() - 5;
        format!("{}#{}", &name[..split_pos], &name[split_pos..])
    }
}

#[derive(Serialize)]
struct LoginWithSteamRequest {
    #[serde(rename = "TitleId")]
    title_id: String,

    #[serde(rename = "CreateAccount")]
    create_account: bool,

    #[serde(rename = "SteamTicket")]
    steam_ticket: String,

    #[serde(rename = "InfoRequestParameters")]
    params: GetPlayerCombinedInfoRequestParams,
}

#[derive(Serialize, Default)]
struct GetPlayerCombinedInfoRequestParams {
    #[serde(rename = "GetCharacterInventories")]
    character_inventories: bool,

    #[serde(rename = "GetCharacterList")]
    character_list: bool,

    #[serde(rename = "GetPlayerProfile")]
    player_profile: bool,

    #[serde(rename = "GetPlayerStatistics")]
    player_statistics: bool,

    #[serde(rename = "GetTitleData")]
    title_data: bool,

    #[serde(rename = "GetUserAccountInfo")]
    user_account_info: bool,

    #[serde(rename = "GetUserData")]
    user_data: bool,

    #[serde(rename = "GetUserReadOnlyData")]
    user_read_only_data: bool,

    #[serde(rename = "GetUserInventory")]
    user_inventory: bool,

    #[serde(rename = "GetUserVirtualCurrency")]
    user_virtual_currency: bool,
}

#[derive(Debug, Deserialize)]
struct LoginWithSteamResponse {
    #[serde(rename = "InfoResultPayload")]
    info_result: GetPlayerCombinedInfoResultPayload,
}

#[derive(Debug, Deserialize)]
struct GetPlayerCombinedInfoResultPayload {
    #[serde(rename = "AccountInfo")]
    account_info: Option<UserAccountInfo>,
}

#[derive(Debug, Deserialize)]
struct UserAccountInfo {
    #[serde(rename = "PlayFabId")]
    id: String,

    #[serde(rename = "TitleInfo")]
    title_info: UserTitleInfo,

    #[serde(rename = "SteamInfo")]
    steam_info: Option<UserSteamInfo>,
}

#[derive(Debug, Deserialize)]
struct UserSteamInfo {
    #[serde(rename = "SteamId")]
    id: String,
}

#[derive(Debug, Deserialize)]
struct UserTitleInfo {
    #[serde(rename = "DisplayName")]
    display_name: String,

    #[serde(rename = "TitlePlayerAccount")]
    title_account: EntityKey,
}

#[derive(Debug, Deserialize)]
struct EntityKey {
    #[serde(rename = "Id")]
    id: String,
}

const TITLE_ID: &str = "A5B4F";
const SDK: &str = "UE4MKPL-1.31.200121";

fn make_client(game: &Game) -> Result<Client> {
    let mut default_headers = HeaderMap::new();
    default_headers.insert("X-PlayFabSDK", HeaderValue::from_static(SDK));

    Ok(http_client_builder(game)
        .default_headers(default_headers)
        .build()?)
}

fn endpoint_url(path: &str) -> String {
    format!("https://{}.playfabapi.com/{}", TITLE_ID, path)
}
