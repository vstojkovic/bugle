use std::sync::{Arc, Mutex};

use anyhow::Result;
use fltk::app::{self, App};
use fltk::dialog;
use net::{is_valid_ip, is_valid_port};
use servers::{fetch_server_list, Server, ServerQueryClient, ServerQueryRequest, Validity};

mod config;
mod game;
mod gui;
mod net;
mod servers;

use self::game::Game;
use self::gui::{Action, LauncherWindow, ServerBrowserAction, ServerBrowserUpdate, Update};

struct Launcher {
    app: App,
    game: Game,
    tx: app::Sender<Update>,
    rx: app::Receiver<Update>,
    query_client: Mutex<Option<ServerQueryClient>>,
}

impl Launcher {
    fn new(app: App, game: Game) -> Arc<Self> {
        let (tx, rx) = app::channel();
        Arc::new(Self {
            app,
            game,
            tx,
            rx,
            query_client: Mutex::new(None),
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
        }
    }

    fn on_load_servers(self: Arc<Self>) -> Result<()> {
        tokio::spawn(async move {
            let servers = self.load_servers().await;
            if let Ok(servers) = &servers {
                let tx = self.tx.clone();
                let query_client = ServerQueryClient::new(self.game.build_id(), move |response| {
                    tx.send(Update::ServerBrowser(ServerBrowserUpdate::UpdateServer(
                        response,
                    )));
                })
                .unwrap(); // FIXME: Show error
                query_client.send(
                    servers
                        .iter()
                        .enumerate()
                        .filter_map(|(idx, server)| ServerQueryRequest::for_server(idx, server)),
                );
                *self.query_client.lock().unwrap() = Some(query_client);
            };
            self.tx
                .send(Update::ServerBrowser(ServerBrowserUpdate::PopulateServers(
                    servers,
                )));
        });
        Ok(())
    }

    fn on_ping_server(&self, request: ServerQueryRequest) -> Result<()> {
        if let Some(client) = self.query_client.lock().unwrap().as_ref() {
            client.send([request]);
        }
        Ok(())
    }

    async fn load_servers(&self) -> Result<Vec<Server>> {
        let mut servers = fetch_server_list().await?;

        for server in servers.iter_mut() {
            if server.build_id != self.game.build_id() {
                server.validity.insert(Validity::INVALID_BUILD);
            }
            if !is_valid_ip(server.ip()) {
                server.validity.insert(Validity::INVALID_ADDR);
            }
            if !is_valid_port(server.port) {
                server.validity.insert(Validity::INVALID_PORT);
            }
        }

        Ok(servers)
    }
}

#[tokio::main]
async fn main() {
    let app = App::default();

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
    let game = match Game::new(game_root) {
        Ok(game) => game,
        Err(err) => {
            gui::alert_error(
                "There was a problem with your Conan Exiles installation.",
                &err,
            );
            return;
        }
    };

    let launcher = Launcher::new(app, game);
    launcher.run();
}
