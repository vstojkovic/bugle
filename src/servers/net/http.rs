use anyhow::anyhow;
use futures::future::try_join_all;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{Client, Response, Result};
use serde::Deserialize;
use slog::{info, warn, Logger};

use crate::servers::{DeserializationContext, Server};

const SERVER_DIRECTORY_URL: &str = "https://ce-fcsd-winoff-ams.funcom.com";

pub async fn fetch_server_list<'dc>(
    logger: Logger,
    ctx: DeserializationContext<'dc>,
) -> anyhow::Result<Vec<Server>> {
    info!(logger, "Fetching server bucket list");
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

    info!(
        logger,
        "Fetching servers from {num_buckets} buckets",
        num_buckets = bucket_list.buckets.len()
    );
    let responses = try_join_all(bucket_list.buckets.iter().map(|bucket| {
        client
            .get(format!("{}/buckets/{}", SERVER_DIRECTORY_URL, bucket))
            .send()
    }))
    .await?;
    let servers = try_join_all(
        responses
            .into_iter()
            .map(|response| parse_servers(&logger, response, &ctx)),
    )
    .await?
    .into_iter()
    .flatten()
    .collect::<Vec<Server>>();

    info!(
        logger,
        "Fetched {num_servers} servers",
        num_servers = servers.len()
    );

    Ok(servers)
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

async fn parse_servers<'dc>(
    logger: &Logger,
    response: Response,
    ctx: &'dc DeserializationContext<'dc>,
) -> anyhow::Result<Vec<Server>> {
    let json = response.json::<serde_json::Value>().await?;
    let json = json
        .as_object()
        .ok_or(anyhow!("expected a JSON object in response"))?
        .get("sessions")
        .ok_or(anyhow!("cannot find 'sessions' key in response"))?
        .as_array()
        .ok_or(anyhow!("expected a JSON array in 'sessions' key"))?;

    let mut result = Vec::with_capacity(json.len());
    for server in json {
        match Server::deserialize(server, ctx) {
            Ok(server) => result.push(server),
            Err(err) => warn!(logger, "Error parsing server"; "error" => %err, "server" => %server),
        }
    }

    Ok(result)
}
