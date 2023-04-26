#![cfg_attr(windows, windows_subsystem = "windows")]

use std::cell::{Cell, Ref, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::str::FromStr;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

use anyhow::Result;
use config::{BattlEyeUsage, Config, ConfigPersister, IniConfigPersister, TransientConfig};
use fltk::app::{self, App};
use fltk::dialog::{self, FileDialogOptions, FileDialogType, NativeFileChooser};
use game::{list_mod_controllers, Launch, ModRef};
use gui::{prompt_confirm, LaunchMonitor, ModManagerAction, SinglePlayerAction};
use regex::Regex;
use slog::{info, trace, warn, FilterLevel, Logger};
use workers::{SavedGamesWorker, ServerLoaderWorker};

mod config;
mod env;
mod game;
mod gui;
mod logger;
mod net;
mod servers;
mod workers;

use crate::game::{LaunchState, Mods};
use crate::gui::ModManagerUpdate;

use self::game::Game;
use self::gui::{Action, LauncherWindow, ServerBrowserAction, Update};
use self::logger::create_root_logger;

struct Launcher {
    logger: Logger,
    app: App,
    game: Arc<Game>,
    config: RefCell<Config>,
    config_persister: Box<dyn ConfigPersister + Send + Sync>,
    tx: app::Sender<Update>,
    rx: app::Receiver<Update>,
    pending_update: RefCell<Option<Update>>,
    main_window: LauncherWindow,
    server_loader_worker: Arc<ServerLoaderWorker>,
    saved_games_worker: Arc<SavedGamesWorker>,
}

impl Launcher {
    fn new(
        logger: Logger,
        app: App,
        game: Game,
        config: Config,
        config_persister: Box<dyn ConfigPersister + Send + Sync>,
    ) -> Rc<Self> {
        let game = Arc::new(game);
        let (tx, rx) = app::channel();

        let server_loader_worker =
            ServerLoaderWorker::new(logger.clone(), Arc::clone(&game), tx.clone());

        let saved_games_worker = SavedGamesWorker::new(Arc::clone(&game), tx.clone());

        let main_window = LauncherWindow::new(&*game, &config);

        let launcher = Rc::new(Self {
            logger,
            app,
            game,
            config: RefCell::new(config),
            config_persister,
            tx,
            rx,
            pending_update: RefCell::new(None),
            main_window,
            server_loader_worker,
            saved_games_worker,
        });

        launcher.main_window.set_on_action({
            let this = Rc::clone(&launcher);
            move |action| this.on_action(action)
        });

        launcher
    }

    fn run(&self) {
        self.main_window.show();
        self.server_loader_worker.load_servers();

        while self.app.wait() {
            while self.run_loop_iteration() {
                app::check();
            }
        }
    }

    fn run_loop_iteration(&self) -> bool {
        let mut pending_ref = self.pending_update.borrow_mut();
        let pending_update = pending_ref.take();
        let next_update = self.rx.recv();
        let (ready_update, pending_update) = match (pending_update, next_update) {
            (Some(pending), Some(next)) => match pending.try_consolidate(next) {
                Ok(consolidated) => (None, Some(consolidated)),
                Err((pending, next)) => (Some(pending), Some(next)),
            },
            (Some(pending), None) => (Some(pending), None),
            (None, Some(next)) => (None, Some(next)),
            (None, None) => (None, None),
        };
        if let Some(update) = ready_update {
            self.main_window.handle_update(update);
        }
        *pending_ref = pending_update;
        pending_ref.is_some()
    }

    fn on_action(self: &Rc<Self>, action: Action) -> Result<()> {
        match action {
            Action::Launch => {
                let use_battleye = if let BattlEyeUsage::Always(true) = self.config().use_battleye {
                    true
                } else {
                    false
                };
                if self.monitor_launch(self.game.launch(use_battleye, &[])?)? {
                    app::quit();
                }
                Ok(())
            }
            Action::Continue => {
                let use_battleye = if let BattlEyeUsage::Always(true) = self.config().use_battleye {
                    true
                } else {
                    false
                };
                if self.monitor_launch(self.game.continue_session(use_battleye)?)? {
                    app::quit();
                }
                Ok(())
            }
            Action::ConfigureBattlEye(use_battleye) => {
                self.update_config(|config| config.use_battleye = use_battleye)
            }
            Action::ServerBrowser(ServerBrowserAction::LoadServers) => {
                self.server_loader_worker.load_servers();
                Ok(())
            }
            Action::ServerBrowser(ServerBrowserAction::JoinServer {
                addr,
                battleye_required,
            }) => {
                let use_battleye = match self.config().use_battleye {
                    BattlEyeUsage::Auto => battleye_required,
                    BattlEyeUsage::Always(enabled) => enabled,
                };
                if self.monitor_launch(self.game.join_server(addr, use_battleye)?)? {
                    app::quit();
                }
                Ok(())
            }
            Action::ServerBrowser(ServerBrowserAction::PingServer(request)) => {
                self.server_loader_worker.ping_server(request)
            }
            Action::ServerBrowser(ServerBrowserAction::PingServers(requests)) => {
                self.server_loader_worker.ping_servers(requests)
            }
            Action::ServerBrowser(ServerBrowserAction::UpdateFavorites(favorites)) => {
                self.game.save_favorites(favorites)
            }
            Action::ServerBrowser(ServerBrowserAction::UpdateConfig(sb_cfg)) => {
                self.update_config(|config| config.server_browser = sb_cfg)
            }
            Action::SinglePlayer(SinglePlayerAction::ListSavedGames) => {
                Arc::clone(&self.saved_games_worker).list_games()
            }
            Action::SinglePlayer(SinglePlayerAction::NewSavedGame { map_id }) => {
                self.saved_games_worker.clear_progress(map_id)?;
                self.launch_single_player(map_id)
            }
            Action::SinglePlayer(SinglePlayerAction::ContinueSavedGame { map_id }) => {
                self.launch_single_player(map_id)
            }
            Action::SinglePlayer(SinglePlayerAction::LoadSavedGame {
                map_id,
                backup_name,
            }) => self.saved_games_worker.restore_backup(map_id, backup_name),
            Action::SinglePlayer(SinglePlayerAction::SaveGame {
                map_id,
                backup_name,
            }) => self.saved_games_worker.create_backup(map_id, backup_name),
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

    fn monitor_launch(&self, mut launch: Launch) -> Result<bool> {
        if let LaunchState::Ready = launch.poll()? {
            return Ok(true);
        }

        let monitor = LaunchMonitor::new(self.main_window.window());
        monitor.show();

        let should_poll = Rc::new(Cell::new(true));
        app::add_timeout3(1.0, {
            let should_poll = Rc::downgrade(&should_poll);
            let logger = self.logger.clone();
            move |handle| {
                if let Some(should_poll) = should_poll.upgrade() {
                    let poll_skipped = should_poll.replace(true);
                    trace!(logger, "Firing poll timer"; "poll_skipped" => poll_skipped);
                    app::repeat_timeout3(1.0, handle);
                    app::awake();
                }
            }
        });
        loop {
            if should_poll.replace(false) {
                if let LaunchState::Ready = launch.poll()? {
                    return Ok(true);
                }
            }
            if monitor.cancel_requested() {
                launch.cancel();
                return Ok(false);
            }
            if self.run_loop_iteration() {
                app::check();
            } else if !self.app.wait() {
                return Ok(true);
            }
        }
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
        if self.monitor_launch(self.game.launch_single_player(map_id, use_battleye)?)? {
            app::quit();
        }
        Ok(())
    }

    fn validate_single_player_mods(&self, map_id: usize) -> Result<Option<ModMismatch>> {
        let installed_mods = self.game.installed_mods();
        let mod_list = self.game.load_mod_list()?;
        let mut active_mods: HashSet<ModRef> = mod_list.into_iter().collect();

        let db_path = self.game.in_progress_game_path(map_id);
        let db_metadata = std::fs::metadata(&db_path)?;
        let mod_controllers =
            if db_metadata.len() != 0 { list_mod_controllers(db_path)? } else { Vec::new() };

        let mut required_folders = HashMap::new();
        let folder_regex = Regex::new("/Game/Mods/([^/]+)/.*").unwrap();
        for controller in mod_controllers {
            if let Some(captures) = folder_regex.captures(&controller) {
                let folder = captures.get(1).unwrap().as_str();
                required_folders.insert(folder.to_string(), false);
            }
        }

        let mut added_mods = HashSet::new();
        for mod_ref in active_mods.drain() {
            if let Some(mod_info) = installed_mods.get(&mod_ref) {
                if let Some(active) = required_folders.get_mut(&mod_info.folder_name) {
                    *active = true;
                    continue;
                }
            }
            added_mods.insert(mod_ref);
        }

        let mut missing_mods = HashSet::new();
        for (folder, active) in required_folders.drain() {
            if !active {
                missing_mods.insert(installed_mods.by_folder(folder));
            }
        }

        if added_mods.is_empty() && missing_mods.is_empty() {
            Ok(None)
        } else {
            Ok(Some(ModMismatch {
                missing_mods,
                added_mods,
            }))
        }
    }

    fn config(&self) -> Ref<Config> {
        self.config.borrow()
    }

    fn update_config(&self, mutator: impl FnOnce(&mut Config)) -> Result<()> {
        let mut config = self.config.borrow_mut();
        mutator(&mut config);
        if let Err(err) = self.config_persister.save(&config) {
            warn!(self.logger, "Error while saving the configuration"; "error" => err.to_string());
        }
        Ok(())
    }
}

struct ModMismatch {
    missing_mods: HashSet<ModRef>,
    added_mods: HashSet<ModRef>,
}

const PROMPT_SP_MOD_MISMATCH: &str =
    "It looks like your mod list doesn't match this game. Launch anyway?";
const TXT_MISSING_MODS: &str = "Missing mods:";
const TXT_ADDED_MODS: &str = "Added mods:";
const DLG_FILTER_MODLIST: &str = "Modlist Files\t*.modlist";

#[tokio::main]
async fn main() {
    let mut args = pico_args::Arguments::from_env();
    let log_level_override = args
        .opt_value_from_fn(["-l", "--log-level"], |s| {
            FilterLevel::from_str(s).map_err(|_| "")
        })
        .ok()
        .unwrap_or_default();
    let log_level = Arc::new(AtomicUsize::new(
        log_level_override.unwrap_or(FilterLevel::Info).as_usize(),
    ));
    let root_logger = create_root_logger(&log_level);

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
    let config = config_persister.load().unwrap_or_else(|err| {
        warn!(root_logger, "Error while loading the configuration"; "error" => err.to_string());
        Config::default()
    });

    if log_level_override.is_none() {
        log_level.store(
            config.log_level.0.as_usize(),
            std::sync::atomic::Ordering::Relaxed,
        );
    }

    let app = App::default();

    let game_location = match Game::locate(&root_logger) {
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

    let launcher = Launcher::new(root_logger.clone(), app, game, config, config_persister);
    launcher.run();

    info!(root_logger, "Shutting down launcher");
}
