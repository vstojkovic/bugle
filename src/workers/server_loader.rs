use std::sync::{Arc, Mutex};

use anyhow::Result;
use fltk::app;
use slog::{o, Logger};
use tokio::task::JoinHandle;

use crate::game::Game;
use crate::gui::{ServerBrowserUpdate, Update};
use crate::servers::{fetch_server_list, DeserializationContext, PingClient, PingRequest, Server};

pub struct ServerLoaderWorker {
    logger: Logger,
    game: Arc<Game>,
    tx: app::Sender<Update>,
    server_loader: Mutex<ServerLoader>,
}

#[derive(Default)]
struct ServerLoader {
    generation: u32,
    state: ServerLoaderState,
}

#[derive(Default)]
enum ServerLoaderState {
    #[default]
    Inactive,
    Fetching(JoinHandle<()>),
    Pinging(PingClient),
}

impl ServerLoaderWorker {
    pub fn new(logger: Logger, game: Arc<Game>, tx: app::Sender<Update>) -> Arc<Self> {
        Arc::new(Self {
            logger,
            game,
            tx,
            server_loader: Mutex::new(Default::default()),
        })
    }

    pub fn load_servers(self: &Arc<Self>) -> Result<()> {
        let this = Arc::clone(self);
        let mut server_loader = this.server_loader.lock().unwrap();
        if let ServerLoaderState::Fetching(_) = &server_loader.state {
            return Ok(());
        }
        let fetch_generation = server_loader.generation.wrapping_add(1);
        server_loader.generation = fetch_generation;
        server_loader.state =
            ServerLoaderState::Fetching(Arc::clone(self).spawn_fetcher(fetch_generation));
        Ok(())
    }

    pub fn ping_server(&self, request: PingRequest) -> Result<()> {
        if let ServerLoaderState::Pinging(client) = &self.server_loader.lock().unwrap().state {
            client.priority_send(request);
        }
        Ok(())
    }

    fn spawn_fetcher(self: Arc<Self>, generation: u32) -> JoinHandle<()> {
        tokio::spawn(async move {
            let servers = self.fetch_servers().await;

            let mut server_loader = self.server_loader.lock().unwrap();
            if server_loader.generation != generation {
                return;
            }

            let ping_generation = generation.wrapping_add(1);
            server_loader.generation = ping_generation;

            let servers_and_state = servers.and_then(|servers| {
                let ping_client = Arc::clone(&self).make_ping_client(ping_generation)?;
                ping_client.send(
                    servers
                        .iter()
                        .enumerate()
                        .filter_map(|(idx, server)| PingRequest::for_server(idx, server)),
                );
                Ok((servers, ServerLoaderState::Pinging(ping_client)))
            });
            let (servers, state) = match servers_and_state {
                Ok((servers, state)) => (Ok(servers), state),
                Err(err) => (Err(err), ServerLoaderState::Inactive),
            };
            server_loader.state = state;

            let update = Update::ServerBrowser(ServerBrowserUpdate::PopulateServers(servers));
            self.tx.send(update);
        })
    }

    fn make_ping_client(self: Arc<Self>, generation: u32) -> Result<PingClient> {
        let ping_logger = self.logger.new(o!("ping_generation" => generation));
        Ok(PingClient::new(
            ping_logger,
            self.game.build_id(),
            move |response| {
                if self.server_loader.lock().unwrap().generation != generation {
                    return;
                }
                self.tx
                    .send(Update::ServerBrowser(ServerBrowserUpdate::UpdateServer(
                        response,
                    )));
            },
        )?)
    }

    async fn fetch_servers(&self) -> Result<Vec<Server>> {
        let favorites = self.game.load_favorites()?;
        Ok(fetch_server_list(
            self.logger.clone(),
            DeserializationContext {
                build_id: self.game.build_id(),
                favorites: &&favorites,
            },
        )
        .await?)
    }
}
