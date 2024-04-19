use std::sync::{Arc, Mutex};

use anyhow::Result;
use fltk::app;
use slog::{o, Logger};
use tokio::task::JoinHandle;

use crate::game::Game;
use crate::servers::{fetch_server_list, PingClient, PingRequest, Server};
use crate::Message;

pub struct ServerLoaderWorker {
    logger: Logger,
    game: Arc<Game>,
    tx: app::Sender<Message>,
    server_loader: Mutex<ServerLoader>,
}

#[derive(Default)]
struct ServerLoader {
    generation: u32,
    fetcher: Option<JoinHandle<()>>,
    pinger: Option<PingClient>,
}

impl ServerLoaderWorker {
    pub fn new(logger: Logger, game: Arc<Game>, tx: app::Sender<Message>) -> Arc<Self> {
        Arc::new(Self {
            logger,
            game,
            tx,
            server_loader: Mutex::new(Default::default()),
        })
    }

    pub fn load_servers(self: &Arc<Self>) {
        let mut server_loader = self.server_loader.lock().unwrap();
        if server_loader.fetcher.is_some() {
            return;
        }

        let generation = server_loader.generation.wrapping_add(1);
        server_loader.generation = generation;
        server_loader.fetcher = Some(Arc::clone(self).spawn_fetcher(generation));
        server_loader.pinger = None;
    }

    pub fn is_loading(&self) -> bool {
        self.server_loader.lock().unwrap().fetcher.is_some()
    }

    pub fn ping_servers(self: &Arc<Self>, requests: Vec<PingRequest>) -> Result<()> {
        self.with_ping_client(|client| client.send(requests))
    }

    pub fn ping_server(self: &Arc<Self>, request: PingRequest) -> Result<()> {
        self.with_ping_client(|client| client.priority_send(request))
    }

    fn spawn_fetcher(self: Arc<Self>, generation: u32) -> JoinHandle<()> {
        tokio::spawn(async move {
            let servers = self.fetch_servers().await;

            let mut server_loader = self.server_loader.lock().unwrap();
            if server_loader.generation != generation {
                return;
            }

            self.tx.send(Message::ServerList(servers));

            server_loader.fetcher = None;
        })
    }

    fn with_ping_client<R, F: FnOnce(&PingClient) -> R>(self: &Arc<Self>, cb: F) -> Result<R> {
        let mut server_loader = self.server_loader.lock().unwrap();
        if let None = &server_loader.pinger {
            let pinger = Arc::clone(self).make_ping_client(server_loader.generation)?;
            server_loader.pinger = Some(pinger);
        };
        Ok(cb(server_loader.pinger.as_ref().unwrap()))
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
                self.tx.send(Message::ServerPong(response));
            },
        )?)
    }

    async fn fetch_servers(&self) -> Result<Vec<Server>> {
        Ok(fetch_server_list(self.logger.clone(), &*self.game).await?)
    }
}
