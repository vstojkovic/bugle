use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::Arc;

use anyhow::Result;
use fltk::app::{self, App};
use fltk::dialog;
use net::{is_valid_ip, is_valid_port};
use servers::{fetch_server_list, Server, ServerQueryClient, ServerQueryRequest, Validity};
use steamlocate::SteamDir;

mod config;
mod gui;
mod net;
mod servers;

use gui::{Action, LauncherWindow, ServerBrowserAction, ServerBrowserUpdate, Update};

struct Game {
    root: PathBuf,
    build_id: u32,
}

impl Game {
    fn locate() -> Option<PathBuf> {
        let mut steam = SteamDir::locate()?;
        let app = steam.app(&440900)?;

        Some(app.path.clone())
    }

    fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut engine_ini_path = path.as_ref().to_path_buf();
        engine_ini_path.extend([
            "ConanSandbox",
            "Saved",
            "Config",
            "WindowsNoEditor",
            "Engine.ini",
        ]);

        let engine_ini = config::load_ini(engine_ini_path)?;
        let build_id = engine_ini
            .get_from(Some("OnlineSubsystem"), "BuildIdOverride")
            .ok_or_else(|| anyhow::Error::msg("Missing build ID override"))
            .and_then(|s| Ok(s.parse::<u32>()?))?;

        Ok(Self {
            root: path.as_ref().into(),
            build_id,
        })
    }

    fn launch(&self, enable_battleye: bool, args: &[&str]) -> Result<Child> {
        let mut exe_path = self.root.clone();
        exe_path.extend(["ConanSandbox", "Binaries", "Win64"]);
        exe_path.push(if enable_battleye { "ConanSandbox_BE.exe" } else { "ConanSandbox.exe" });

        let mut cmd = Command::new(exe_path);
        cmd.args(args);
        if enable_battleye {
            cmd.arg("-BattlEye");
        }

        Ok(cmd.spawn()?)
    }

    fn continue_session(&self, enable_battleye: bool) -> Result<Child> {
        self.launch(enable_battleye, &["-continuesession"])
    }

    fn join_server(&self, addr: SocketAddr, enable_battleye: bool) -> Result<Child> {
        let mut game_ini_path = self.root.clone();
        game_ini_path.extend([
            "ConanSandbox",
            "Saved",
            "Config",
            "WindowsNoEditor",
            "Game.ini",
        ]);

        let mut game_ini = config::load_ini(&game_ini_path)?;
        game_ini
            .with_section(Some("SavedServers"))
            .set("LastConnected", addr.to_string());
        game_ini
            .with_section(Some("SavedCoopData"))
            .set("StartedListenServerSession", "False");
        config::save_ini(&game_ini, &game_ini_path)?;

        self.continue_session(enable_battleye)
    }
}

async fn load_servers(game: Arc<Game>) -> Result<Vec<Server>> {
    let mut servers = fetch_server_list().await?;

    for server in servers.iter_mut() {
        if server.build_id != game.build_id {
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

#[tokio::main]
async fn main() {
    let launcher = App::default();

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
    let game = Arc::new(match Game::new(game_root) {
        Ok(game) => game,
        Err(err) => {
            gui::alert_error(
                "There was a problem with your Conan Exiles installation.",
                &err,
            );
            return;
        }
    });

    let (tx, rx) = app::channel();

    // TODO: Refactor and organize
    let on_action = {
        let game = Arc::clone(&game);
        move |action| match action {
            Action::Continue => {
                let _ = game.continue_session(true)?;
                app::quit();
                Ok(())
            }
            Action::ServerBrowser(ServerBrowserAction::LoadServers) => {
                let tx = tx.clone();
                let game = Arc::clone(&game);
                tokio::spawn(async move {
                    let servers = load_servers(Arc::clone(&game)).await;
                    if let Ok(servers) = &servers {
                        let tx = tx.clone();
                        let query_client = ServerQueryClient::new(game.build_id, move |response| {
                            tx.send(Update::ServerBrowser(ServerBrowserUpdate::UpdateServer(
                                response,
                            )));
                        })
                        .unwrap(); // FIXME: Show error
                        query_client.send(servers.iter().enumerate().filter_map(
                            |(idx, server)| ServerQueryRequest::for_server(idx, server),
                        ));
                    };
                    tx.send(Update::ServerBrowser(ServerBrowserUpdate::PopulateServers(
                        servers,
                    )));
                });
                Ok(())
            }
            Action::ServerBrowser(ServerBrowserAction::JoinServer(addr)) => {
                let _ = game.join_server(addr, true)?;
                app::quit();
                Ok(())
            }
        }
    };

    let mut main_win = LauncherWindow::new(game.build_id, on_action);
    main_win.show();

    while launcher.wait() {
        while let Some(mut update) = rx.recv() {
            while let Some(next) = rx.recv() {
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
