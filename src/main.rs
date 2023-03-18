use std::collections::HashSet;
use std::fs::File;
use std::sync::{Arc, Mutex, MutexGuard};

use anyhow::Result;
use config::{BattlEyeUsage, Config, ConfigPersister, IniConfigPersister, TransientConfig};
use fltk::app::{self, App};
use fltk::dialog::{self, FileDialogOptions, FileDialogType, NativeFileChooser};
use game::{list_mod_controllers, ModRef};
use gui::{prompt_confirm, ModManagerAction, SinglePlayerAction};
use regex::Regex;
use servers::DeserializationContext;
use slog::{info, o, warn, Logger};
use tokio::task::JoinHandle;

mod config;
mod game;
mod gui;
mod net;
mod servers;

use crate::game::Mods;
use crate::gui::{ModManagerUpdate, SinglePlayerUpdate};

use self::game::Game;
use self::gui::{Action, LauncherWindow, ServerBrowserAction, ServerBrowserUpdate, Update};
use self::servers::{fetch_server_list, PingClient, PingRequest, Server};

struct Launcher {
    logger: Logger,
    app: App,
    game: Game,
    config: Mutex<Config>,
    config_persister: Box<dyn ConfigPersister + Send + Sync>,
    tx: app::Sender<Update>,
    rx: app::Receiver<Update>,
    server_loader: Mutex<ServerLoader>,
}

impl Launcher {
    fn new(
        logger: Logger,
        app: App,
        game: Game,
        config_persister: Box<dyn ConfigPersister + Send + Sync>,
    ) -> Arc<Self> {
        let config = config_persister.load().unwrap_or_else(|err| {
            warn!(logger, "Error while loading the configuration"; "error" => err.to_string());
            Config::default()
        });
        let (tx, rx) = app::channel();
        Arc::new(Self {
            logger,
            app,
            game,
            config: Mutex::new(config),
            config_persister,
            tx,
            rx,
            server_loader: Mutex::new(Default::default()),
        })
    }

    fn run(self: Arc<Self>) {
        let mut main_win = LauncherWindow::new(&self.game, &*self.config(), {
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
                let use_battleye = if let BattlEyeUsage::Always(true) = self.config().use_battleye {
                    true
                } else {
                    false
                };
                let _ = self.game.launch(use_battleye, &[])?;
                app::quit();
                Ok(())
            }
            Action::Continue => {
                let use_battleye = if let BattlEyeUsage::Always(true) = self.config().use_battleye {
                    true
                } else {
                    false
                };
                let _ = self.game.continue_session(use_battleye)?;
                app::quit();
                Ok(())
            }
            Action::ConfigureBattlEye(use_battleye) => {
                self.update_config(|config| config.use_battleye = use_battleye)
            }
            Action::ServerBrowser(ServerBrowserAction::LoadServers) => {
                Arc::clone(self).on_load_servers()
            }
            Action::ServerBrowser(ServerBrowserAction::JoinServer {
                addr,
                battleye_required,
            }) => {
                let use_battleye = match self.config().use_battleye {
                    BattlEyeUsage::Auto => battleye_required,
                    BattlEyeUsage::Always(enabled) => enabled,
                };
                let _ = self.game.join_server(addr, use_battleye)?;
                app::quit();
                Ok(())
            }
            Action::ServerBrowser(ServerBrowserAction::PingServer(request)) => {
                self.on_ping_server(request)
            }
            Action::ServerBrowser(ServerBrowserAction::UpdateFavorites(favorites)) => {
                self.game.save_favorites(favorites)
            }
            Action::ServerBrowser(ServerBrowserAction::UpdateConfig(sb_cfg)) => {
                self.update_config(|config| config.server_browser = sb_cfg)
            }
            Action::SinglePlayer(SinglePlayerAction::ListSavedGames) => {
                Arc::clone(self).on_list_saved_games()
            }
            Action::SinglePlayer(SinglePlayerAction::NewSavedGame { map_id }) => {
                {
                    let _ = File::create(self.game.in_progress_game_path(map_id))?;
                }
                self.launch_single_player(map_id)
            }
            Action::SinglePlayer(SinglePlayerAction::ContinueSavedGame { map_id }) => {
                self.launch_single_player(map_id)
            }
            Action::SinglePlayer(SinglePlayerAction::LoadSavedGame {
                map_id,
                backup_name,
            }) => {
                let src_db_path = self.game.save_path().join(backup_name);
                let dest_db_path = self
                    .game
                    .save_path()
                    .join(&self.game.maps()[map_id].db_name);
                let _ = std::fs::copy(src_db_path, dest_db_path)?;
                Ok(())
            }
            Action::SinglePlayer(SinglePlayerAction::SaveGame {
                map_id,
                backup_name,
            }) => {
                let src_db_path = self
                    .game
                    .save_path()
                    .join(&self.game.maps()[map_id].db_name);
                let dest_db_path = self.game.save_path().join(backup_name);
                let _ = std::fs::copy(src_db_path, dest_db_path)?;
                Ok(())
            }
            Action::SinglePlayer(SinglePlayerAction::DeleteSavedGame { backup_name }) => {
                std::fs::remove_file(self.game.save_path().join(backup_name))?;
                Ok(())
            }
            Action::ModManager(ModManagerAction::LoadModList) => {
                let active_mods = self.game.load_mod_list()?;
                self.tx
                    .send(Update::ModManager(ModManagerUpdate::PopulateModList(
                        active_mods,
                    )));
                Ok(())
            }
            Action::ModManager(ModManagerAction::SaveModList(active_mods)) => {
                self.game.save_mod_list(active_mods.iter())
            }
            Action::ModManager(ModManagerAction::ImportModList) => {
                let mut dialog = NativeFileChooser::new(FileDialogType::BrowseFile);
                dialog.set_filter(DLG_FILTER_MODLIST);
                dialog.set_directory(&self.game.save_path())?;
                dialog.show();

                let mod_list_path = dialog.filename();
                if mod_list_path.as_os_str().is_empty() {
                    return Ok(());
                }

                let active_mods = self.game.load_mod_list_from(&mod_list_path)?;
                self.game.save_mod_list(&active_mods)?;
                self.tx
                    .send(Update::ModManager(ModManagerUpdate::PopulateModList(
                        active_mods,
                    )));

                Ok(())
            }
            Action::ModManager(ModManagerAction::ExportModList(active_mods)) => {
                let mut dialog = NativeFileChooser::new(FileDialogType::BrowseSaveFile);
                dialog.set_filter(DLG_FILTER_MODLIST);
                dialog.set_directory(&self.game.save_path())?;
                dialog.set_option(FileDialogOptions::SaveAsConfirm);
                dialog.show();

                let mod_list_path = dialog.filename();
                if mod_list_path.as_os_str().is_empty() {
                    return Ok(());
                }

                self.game
                    .save_mod_list_to(&mod_list_path, active_mods.iter())
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

    fn on_list_saved_games(self: Arc<Self>) -> Result<()> {
        tokio::spawn(async move {
            let games = self.game.load_saved_games();
            self.tx
                .send(Update::SinglePlayer(SinglePlayerUpdate::PopulateList(
                    games,
                )));
        });
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

    fn launch_single_player(&self, map_id: usize) -> Result<()> {
        fn join_mod_names(heading: &str, mods: &Mods, refs: HashSet<ModRef>) -> String {
            let mut result = String::new();
            if refs.is_empty() {
                return result;
            }

            result.push_str("\n\n");
            result.push_str(heading);
            for mod_ref in refs {
                result.push('\n');
                match mod_ref {
                    ModRef::Installed(idx) => result.push_str(&mods[idx].name),
                    ModRef::UnknownFolder(folder) => result.push_str(&format!("??? ({})", folder)),
                    ModRef::UnknownPakPath(path) => {
                        result.push_str(&format!("??? ({})", path.display()))
                    }
                };
            }
            result
        }

        if let Some(mismatch) = self.validate_single_player_mods(map_id)? {
            let installed_mods = self.game.installed_mods();
            let prompt = format!(
                "{}{}{}",
                PROMPT_SP_MOD_MISMATCH,
                join_mod_names(TXT_MISSING_MODS, installed_mods, mismatch.missing_mods),
                join_mod_names(TXT_ADDED_MODS, installed_mods, mismatch.added_mods),
            );
            if !prompt_confirm(&prompt) {
                return Ok(());
            }
        }
        let use_battleye =
            if let BattlEyeUsage::Always(true) = self.config().use_battleye { true } else { false };
        let _ = self.game.launch_single_player(map_id, use_battleye)?;
        app::quit();
        Ok(())
    }

    fn validate_single_player_mods(&self, map_id: usize) -> Result<Option<ModMismatch>> {
        let installed_mods = self.game.installed_mods();
        let mod_list = self.game.load_mod_list()?;
        let mut added_mods: HashSet<ModRef> = mod_list.into_iter().collect();

        let db_path = self.game.in_progress_game_path(map_id);
        let db_metadata = std::fs::metadata(&db_path)?;

        let mod_controllers =
            if db_metadata.len() != 0 { list_mod_controllers(db_path)? } else { Vec::new() };
        let mut missing_mods = HashSet::new();
        let folder_regex = Regex::new("/Game/Mods/([^/]+)/.*").unwrap();
        for controller in mod_controllers {
            if let Some(captures) = folder_regex.captures(&controller) {
                let folder = captures.get(1).unwrap().as_str();
                missing_mods.insert(installed_mods.by_folder(folder));
            }
        }

        added_mods.retain(|mod_ref| !missing_mods.contains(mod_ref));
        missing_mods.retain(|mod_ref| !added_mods.contains(mod_ref));

        if added_mods.is_empty() && missing_mods.is_empty() {
            Ok(None)
        } else {
            Ok(Some(ModMismatch {
                missing_mods,
                added_mods,
            }))
        }
    }

    fn config(&self) -> MutexGuard<Config> {
        self.config.lock().unwrap()
    }

    fn update_config(&self, mutator: impl FnOnce(&mut Config)) -> Result<()> {
        let mut config = self.config();
        mutator(&mut config);
        if let Err(err) = self.config_persister.save(&config) {
            warn!(self.logger, "Error while saving the configuration"; "error" => err.to_string());
        }
        Ok(())
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

struct ModMismatch {
    missing_mods: HashSet<ModRef>,
    added_mods: HashSet<ModRef>,
}

fn create_root_logger() -> Logger {
    use slog::Drain;

    let drain = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(drain).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    Logger::root(drain, o!())
}

const PROMPT_SP_MOD_MISMATCH: &str =
    "It looks like your mod list doesn't match this game. Launch anyway?";
const TXT_MISSING_MODS: &str = "Missing mods:";
const TXT_ADDED_MODS: &str = "Added mods:";
const DLG_FILTER_MODLIST: &str = "Modlist Files\t*.modlist";

#[tokio::main]
async fn main() {
    let app = App::default();

    let root_logger = create_root_logger();

    let game_location = match Game::locate() {
        Some(root) => root,
        None => {
            dialog::alert_default(
                "Cannot locate Conan Exiles installation. Please verify that you have Conan \
                 Exiles installed in a Steam library and try again.",
            );
            return;
        }
    };
    let game = match Game::new(root_logger.clone(), game_location) {
        Ok(game) => game,
        Err(err) => {
            gui::alert_error(
                "There was a problem with your Conan Exiles installation.",
                &err,
            );
            return;
        }
    };

    let config_persister: Box<dyn ConfigPersister + Send + Sync> =
        match IniConfigPersister::for_current_exe() {
            Ok(persister) => Box::new(persister),
            Err(err) => {
                warn!(
                    root_logger,
                    "Error trying to load or create the config file. \
                     Proceeding with transient config.";
                    "error" => err.to_string()
                );
                Box::new(TransientConfig)
            }
        };

    let launcher = Launcher::new(root_logger.clone(), app, game, config_persister);
    launcher.run();

    info!(root_logger, "Shutting down launcher");
}
