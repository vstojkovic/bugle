use futures::future::try_join_all;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{Client, Result};
use serde::Deserialize;

use crate::servers::Server;

const SERVER_DIRECTORY_URL: &str = "https://ce-fcsd-winoff-ams.funcom.com";

pub async fn fetch_server_list(finalizer: impl Fn(Server) -> Server) -> Result<Vec<Server>> {
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
            .map(|response| response.json::<ServersBucket>()),
    )
    .await?;
    Ok(servers
        .into_iter()
        .map(|list| list.servers)
        .flatten()
        .map(finalizer)
        .collect::<Vec<Server>>())
}

#[derive(Debug, Deserialize)]
struct ServersBucket {
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
