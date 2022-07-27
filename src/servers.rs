use futures::future::try_join_all;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{Client, Result};
use serde::Deserialize;
use serde_repr::Deserialize_repr;

#[derive(Clone, Copy, Debug, Deserialize_repr, PartialEq, Eq)]
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

#[derive(Clone, Debug, Deserialize)]
pub struct Server {
    #[serde(rename = "serverUID")]
    pub id: String,

    #[serde(rename = "Name")]
    pub name: Option<String>,

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
    info_30: u8, // 1 = PVE-C, don't know what other values mean

    #[serde(rename = "buildId")]
    pub build_id: u64,
}

impl Server {
    pub fn is_conflict(&self) -> bool {
        self.info_30 == 1
    }
}

const SERVER_DIRECTORY_URL: &str = "https://ce-fcsd-winoff-ams.funcom.com";

pub async fn fetch_server_list() -> Result<Vec<Server>> {
    let client = make_client()?;
    let bucket_list = client
        .get(format!(
            "{}/buckets/index_Windows.json",
            SERVER_DIRECTORY_URL
        ))
        .send()
        .await?
        .json::<BucketList>()
        .await?;
    let responses = try_join_all(bucket_list.buckets.iter().map(|bucket| {
        client
            .get(format!("{}/buckets/{}", SERVER_DIRECTORY_URL, bucket))
            .send()
    }))
    .await?;
    let servers = try_join_all(
        responses
            .into_iter()
            .map(|response| response.json::<ServerList>()),
    )
    .await?;
    Ok(servers
        .into_iter()
        .map(|list| list.servers)
        .flatten()
        .collect())
}

#[derive(Debug, Deserialize)]
struct ServerList {
    #[serde(rename = "sessions")]
    pub servers: Vec<Server>,
}

#[derive(Debug, Deserialize)]
struct BucketList {
    buckets: Vec<String>,
}

fn make_client() -> Result<Client> {
    let mut default_headers = HeaderMap::new();
    default_headers.insert(
        "X-API-Key",
        HeaderValue::from_static(
            "aWAWirTCDr49G569tL8Cgv5D7WyvfCzFTHMcCGvbXeHY08i3G64uv1TWKkiHMFDE",
        ),
    );

    Client::builder()
        .user_agent("game=ConanSandbox, engine=UE4, version=354133")
        .default_headers(default_headers)
        .gzip(true)
        .build()
}
