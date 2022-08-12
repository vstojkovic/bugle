use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::rc::Rc;

use anyhow::Result;
use fltk::app::{self, App};
use fltk::dialog;
use net::{is_valid_ip, is_valid_port};
use servers::{ServerQueryClient, ServerQueryRequest, Validity};
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

    fn launch(&self, enable_battleye: bool, args: &[&str]) -> std::io::Result<Child> {
        let mut exe_path = self.root.clone();
        exe_path.extend(["ConanSandbox", "Binaries", "Win64"]);
        exe_path.push(if enable_battleye { "ConanSandbox_BE.exe" } else { "ConanSandbox.exe" });

        let mut cmd = Command::new(exe_path);
        cmd.args(args);
        if enable_battleye {
            cmd.arg("-BattlEye");
        }

        cmd.spawn()
    }
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
    let game = Rc::new(match Game::new(game_root) {
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
        let game = Rc::clone(&game);
        move |action| match action {
            Action::Continue => {
                let _ = game.launch(true, &["-continuesession"])?;
                app::quit();
                Ok(())
            }
            Action::ServerBrowser(ServerBrowserAction::LoadServers) => {
                let tx = tx.clone();
                let build_id = game.build_id;
                tokio::spawn(async move {
                    let mut servers = servers::fetch_server_list()
                        .await
                        .map_err(anyhow::Error::msg);
                    if let Ok(servers) = &mut servers {
                        for server in servers.iter_mut() {
                            if server.build_id != build_id {
                                server.validity.insert(Validity::INVALID_BUILD);
                            }
                            if !is_valid_ip(server.ip()) {
                                server.validity.insert(Validity::INVALID_ADDR);
                            }
                            if !is_valid_port(server.port) {
                                server.validity.insert(Validity::INVALID_PORT);
                            }
                        }

                        let tx = tx.clone();
                        let query_client = ServerQueryClient::new(build_id, move |response| {
                            tx.send(Update::ServerBrowser(ServerBrowserUpdate::UpdateServer(
                                response,
                            )));
                        })
                        .unwrap(); // FIXME: Show error
                        query_client.send(servers.iter().enumerate().filter_map(
                            |(idx, server)| {
                                if server.build_id == build_id {
                                    ServerQueryRequest::for_server(idx, server)
                                } else {
                                    None
                                }
                            },
                        ));
                    };
                    tx.send(Update::ServerBrowser(ServerBrowserUpdate::PopulateServers(
                        servers,
                    )));
                });
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
