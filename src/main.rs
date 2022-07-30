use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};

use fltk::app::{self, App};
use fltk::dialog;
use servers::ServerList;
use steamlocate::SteamDir;

mod gui;
mod servers;

use gui::{Action, LauncherWindow, ServerBrowserAction, ServerBrowserUpdate, Update};

struct Game {
    root: PathBuf,
}

impl Game {
    fn locate() -> Option<Self> {
        let mut steam = SteamDir::locate()?;
        let app = steam.app(&440900)?;
        Some(Self {
            root: app.path.clone(),
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

struct ServerBrowserData {
    all_servers: ServerList,
    sorted_servers: ServerList,
}

#[tokio::main]
async fn main() {
    let launcher = App::default();

    let game = std::rc::Rc::new({
        match Game::locate() {
            Some(game) => game,
            None => {
                dialog::alert_default(
                    "Cannot locate Conan Exiles installation. Please verify that you have Conan \
                    Exiles installed in a Steam library and try again.",
                );
                return;
            }
        }
    });

    let (tx, rx) = app::channel();

    let on_action = {
        let game = game.clone();
        let server_browser_data: Arc<Mutex<Option<ServerBrowserData>>> = Arc::new(Mutex::new(None));
        move |action| match action {
            Action::Continue => {
                let _ = game.launch(true, &["-continuesession"])?;
                app::quit();
                Ok(())
            }
            Action::ServerBrowser(ServerBrowserAction::LoadServers(sort_criteria)) => {
                let tx = tx.clone();
                let server_browser_data = server_browser_data.clone();
                tokio::spawn(async move {
                    let server_list = servers::fetch_server_list()
                        .await
                        .map_err(anyhow::Error::msg)
                        .map(|all_servers| {
                            let sorted_servers = all_servers.sorted(sort_criteria);
                            let mut data = server_browser_data.lock().unwrap();
                            *data = Some(ServerBrowserData {
                                all_servers,
                                sorted_servers: sorted_servers.clone(),
                            });
                            sorted_servers
                        });
                    tx.send(Update::ServerBrowser(ServerBrowserUpdate::PopulateServers(
                        server_list,
                    )));
                });
                Ok(())
            }
            Action::ServerBrowser(ServerBrowserAction::SortServers(sort_criteria)) => {
                let tx = tx.clone();
                let server_list = {
                    let mut data = server_browser_data.lock().unwrap();
                    let data_ref = data.as_mut().unwrap();
                    let sorted_servers = data_ref.all_servers.sorted(sort_criteria);
                    data_ref.sorted_servers = sorted_servers.clone();
                    sorted_servers
                };
                tx.send(Update::ServerBrowser(ServerBrowserUpdate::PopulateServers(
                    Ok(server_list),
                )));
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
