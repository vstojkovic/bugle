use std::path::{Path, PathBuf};
use std::process::{Child, Command};

use anyhow::Result;
use fltk::app::{self, App};
use fltk::dialog;
use steamlocate::SteamDir;

mod config;
mod gui;
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
    let game = std::rc::Rc::new(match Game::new(game_root) {
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

    let on_action = {
        let game = game.clone();
        move |action| match action {
            Action::Continue => {
                let _ = game.launch(true, &["-continuesession"])?;
                app::quit();
                Ok(())
            }
            Action::ServerBrowser(ServerBrowserAction::LoadServers) => {
                let tx = tx.clone();
                tokio::spawn(async move {
                    let servers = servers::fetch_server_list()
                        .await
                        .map_err(anyhow::Error::msg);
                    tx.send(Update::ServerBrowser(ServerBrowserUpdate::PopulateServers(
                        servers,
                    )));
                });
                Ok(())
            }
        }
    };

    let mut main_win = LauncherWindow::new(on_action);
    main_win.show();

    while launcher.wait() {
        if let Some(update) = rx.recv() {
            main_win.handle_update(update);
        }
    }
}
