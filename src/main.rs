use std::sync::{Arc, Mutex};

use anyhow::Result;
use fltk::app::{self, App};
use fltk::dialog;
use servers::DeserializationContext;
use slog::{info, o, Logger};
use tokio::task::JoinHandle;

mod config;
mod game;
mod gui;
mod net;
mod servers;

use self::game::Game;
use self::gui::{Action, LauncherWindow, ServerBrowserAction, ServerBrowserUpdate, Update};
use self::servers::{fetch_server_list, PingClient, PingRequest, Server};

struct Launcher {
    logger: Logger,
    app: App,
    game: Game,
    tx: app::Sender<Update>,
    rx: app::Receiver<Update>,
    server_loader: Mutex<ServerLoader>,
}

impl Launcher {
    fn new(logger: Logger, app: App, game: Game) -> Arc<Self> {
        let (tx, rx) = app::channel();
        Arc::new(Self {
            logger,
            app,
            game,
            tx,
            rx,
            server_loader: Mutex::new(Default::default()),
        })
    }

    fn run(self: Arc<Self>) {
        let mut main_win = LauncherWindow::new(self.game.build_id(), {
            let this = Arc::clone(&self);
            move |action| this.on_action(action)
        });
        main_win.show();

        while self.app.wait() {
            while let Some(mut update) = self.rx.recv() {
                while let Some(next) = self.rx.recv() {
                    update = match update.try_consolidate(next) {
                        Ok(consolidated) => consolidated,
                        Err((update, next)) => {
                            main_win.handle_update(update);
                            app::check();
                            next
                        }
                    };
                }
                main_win.handle_update(update);
                app::check();
            }
        }
    }

    fn on_action(self: &Arc<Self>, action: Action) -> Result<()> {
        match action {
            Action::Launch => {
                let _ = self.game.launch(true, &[])?;
                app::quit();
                Ok(())
            }
            Action::Continue => {
                let _ = self.game.continue_session(true)?;
                app::quit();
                Ok(())
            }
            Action::ServerBrowser(ServerBrowserAction::LoadServers) => {
                Arc::clone(self).on_load_servers()
            }
            Action::ServerBrowser(ServerBrowserAction::JoinServer(addr)) => {
                let _ = self.game.join_server(addr, true)?;
                app::quit();
                Ok(())
            }
            Action::ServerBrowser(ServerBrowserAction::PingServer(request)) => {
                self.on_ping_server(request)
            }
            Action::ServerBrowser(ServerBrowserAction::UpdateFavorites(favorites)) => {
                self.game.save_favorites(favorites)
            }
        }
    }

    fn on_load_servers(self: Arc<Self>) -> Result<()> {
        let this = Arc::clone(&self);
        let mut server_loader = this.server_loader.lock().unwrap();
        if let ServerLoaderState::Fetching(_) = &server_loader.state {
            return Ok(());
        }
        let fetch_generation = server_loader.generation.wrapping_add(1);
        server_loader.generation = fetch_generation;
        server_loader.state =
            ServerLoaderState::Fetching(Arc::clone(&self).spawn_fetcher(fetch_generation));
        Ok(())
    }

    fn on_ping_server(&self, request: PingRequest) -> Result<()> {
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

fn create_root_logger() -> Logger {
    use slog::Drain;

    let drain = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(drain).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    Logger::root(drain, o!())
}

#[tokio::main]
async fn main() {
    let app = App::default();

    let root_logger = create_root_logger();

    let game_root = match Game::locate() {
        Some(root) => root,
        None => {
            dialog::alert_default(
                "Cannot locate Conan Exiles installation. Please verify that you have Conan \
                Exiles installed in a Steam library and try again.",
            );
            return;
        }
    };
    let game = match Game::new(root_logger.clone(), game_root) {
        Ok(game) => game,
        Err(err) => {
            gui::alert_error(
                "There was a problem with your Conan Exiles installation.",
                &err,
            );
            return;
        }
    };

    let launcher = Launcher::new(root_logger.clone(), app, game);
    launcher.run();

    info!(root_logger, "Shutting down launcher");
}
