use anyhow::anyhow;
use futures::future::try_join_all;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{Client, Response, Result};
use serde::Deserialize;
use slog::{debug, info, warn, Logger};

use crate::game::{Branch, Game};
use crate::net::http_client_builder;
use crate::servers::Server;

pub async fn fetch_server_list<'dc>(logger: &Logger, game: &Game) -> anyhow::Result<Vec<Server>> {
    let url = directory_url(game.branch());

    debug!(logger, "Fetching server list");
    let client = make_client(game)?;
    let bucket_list = client
        .get(format!("{}/buckets/index_Windows.json", url))
        .send()
        .await?
        .json::<BucketList>()
        .await?;

    debug!(
        logger,
        "Fetching servers from buckets";
        "num_buckets" => bucket_list.buckets.len()
    );
    let responses = try_join_all(
        bucket_list
            .buckets
            .iter()
            .map(|bucket| client.get(format!("{}/buckets/{}", url, bucket)).send()),
    )
    .await?;

    debug!(logger, "Parsing servers from responses");
    let servers = try_join_all(
        responses
            .into_iter()
            .map(|response| parse_servers(&logger, response)),
    )
    .await?
    .into_iter()
    .flatten()
    .collect::<Vec<Server>>();

    info!(
        logger,
        "Fetched server list";
        "num_servers" => servers.len()
    );

    Ok(servers)
}

#[derive(Debug, Deserialize)]
struct BucketList {
    buckets: Vec<String>,
}

fn directory_url(branch: Branch) -> &'static str {
    match branch {
        Branch::Main => "https://ce-fcsd-winoff-ams.funcom.com",
        Branch::PublicBeta => "https://ce-fcsd-winoff-wdc.funcom.com",
    }
}

fn make_client(game: &Game) -> Result<Client> {
    let mut default_headers = HeaderMap::new();
    default_headers.insert(
        "X-API-Key",
        HeaderValue::from_static(
            "aWAWirTCDr49G569tL8Cgv5D7WyvfCzFTHMcCGvbXeHY08i3G64uv1TWKkiHMFDE",
        ),
    );

    http_client_builder(game)
        .default_headers(default_headers)
        .gzip(true)
        .build()
}

async fn parse_servers<'dc>(logger: &Logger, response: Response) -> anyhow::Result<Vec<Server>> {
    let json = response.json::<serde_json::Value>().await?;
    let json = json
        .as_object()
        .ok_or_else(|| anyhow!("expected a JSON object in response"))?
        .get("sessions")
        .ok_or_else(|| anyhow!("cannot find 'sessions' key in response"))?
        .as_array()
        .ok_or_else(|| anyhow!("expected a JSON array in 'sessions' key"))?;

    let mut result = Vec::with_capacity(json.len());
    for server in json {
        match <Server as Deserialize>::deserialize(server) {
            Ok(server) => result.push(server),
            Err(err) => warn!(logger, "Error parsing server"; "error" => %err, "server" => %server),
        }
    }

    Ok(result)
}
