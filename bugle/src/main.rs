#![cfg_attr(windows, windows_subsystem = "windows")]

use std::cell::{Cell, Ref, RefCell};
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

use anyhow::{anyhow, bail, Result};
use auth::{CachedUser, CachedUsers};
use bit_vec::BitVec;
use config::{BattlEyeUsage, Config, ConfigPersister, IniConfigPersister, TransientConfig};
use fltk::app::{self, App};
use fltk::dialog::{self, FileDialogOptions, FileDialogType, NativeFileChooser};
use fltk::prelude::WindowExt;
use gui::{alert_error, ServerSettingsDialog};
use lazy_static::lazy_static;
use regex::Regex;
use servers::{Confidence, PingResponse};
use slog::{debug, error, info, trace, warn, FilterLevel, Logger};
use uuid::Uuid;

mod auth;
mod battleye;
mod config;
mod env;
mod game;
mod gui;
mod logger;
mod net;
mod parser_utils;
mod servers;
mod util;
mod workers;

use self::auth::{Account, AuthState, Capability, PlatformUser};
use self::config::ModMismatchChecks;
use self::game::platform::steam::{Steam, SteamClient, SteamModDirectory};
use self::game::platform::ModDirectory;
use self::game::{
    list_mod_controllers, Branch, Game, Launch, LaunchOptions, MapRef, ModEntry, ModRef, Mods,
    ServerRef, Session,
};
use self::gui::theme::Theme;
use self::gui::{
    prompt_confirm, Action, Dialog, HomeAction, HomeUpdate, LauncherWindow, ModManagerAction,
    ModManagerUpdate, ModUpdateProgressDialog, ModUpdateSelectionDialog, ServerBrowserAction,
    ServerBrowserUpdate, SinglePlayerAction, Update,
};
use self::logger::create_root_logger;
use self::servers::{SavedServers, Server, Similarity};
use self::util::weak_cb;
use self::workers::{FlsWorker, SavedGamesWorker, ServerLoaderWorker, TaskState};

pub enum Message {
    Update(Update),
    ServerList(Result<Vec<Server>>),
    ServerPong(PingResponse),
    Account(Result<Account>),
    PlatformReady,
}

type CachedUsersPersister = fn(&Game, &CachedUsers) -> Result<()>;

struct Launcher {
    logger: Logger,
    log_level: Option<Arc<AtomicUsize>>,
    app: App,
    steam: Rc<SteamClient>,
    game: Arc<Game>,
    config: RefCell<Config>,
    config_persister: Box<dyn ConfigPersister + Send + Sync>,
    saved_servers: Option<RefCell<SavedServers>>,
    tx: app::Sender<Message>,
    rx: app::Receiver<Message>,
    mod_directory: Rc<dyn ModDirectory>,
    main_window: LauncherWindow,
    pong_accumulator: RefCell<Vec<PingResponse>>,
    cached_users: RefCell<CachedUsers>,
    cached_users_persister: CachedUsersPersister,
    waiting_for_server_load: Cell<bool>,
    server_loader_worker: Arc<ServerLoaderWorker>,
    saved_games_worker: Arc<SavedGamesWorker>,
    fls_worker: Arc<FlsWorker>,
}

impl Launcher {
    fn new(
        logger: Logger,
        log_level: Option<Arc<AtomicUsize>>,
        can_switch_branch: bool,
        app: App,
        steam: Steam,
        game: Game,
        config: Config,
        config_persister: Box<dyn ConfigPersister + Send + Sync>,
        saved_servers: Option<SavedServers>,
    ) -> Rc<Self> {
        let game = Arc::new(game);
        let (tx, rx) = app::channel();
        let steam = steam.init_client(&*game, tx.clone());

        let mod_directory: Rc<dyn ModDirectory> = SteamModDirectory::new(
            logger.clone(),
            Rc::clone(&steam),
            tx.clone(),
            game.installed_mods(),
        );

        let main_window = LauncherWindow::new(
            logger.clone(),
            Arc::clone(&game),
            &config,
            Rc::clone(&mod_directory),
            log_level.is_none(),
            can_switch_branch,
            saved_servers.is_some(),
        );

        let (cached_users, cached_users_persister) = match game.load_cached_users() {
            Ok(cached_users) => (
                cached_users,
                Game::save_cached_users as CachedUsersPersister,
            ),
            Err(err) => {
                warn!(logger, "Error loading cached users"; "error" => %err);
                fn noop_persister(_: &Game, _: &CachedUsers) -> Result<()> {
                    Ok(())
                }
                (CachedUsers::new(), noop_persister as CachedUsersPersister)
            }
        };

        let server_loader_worker =
            ServerLoaderWorker::new(logger.clone(), Arc::clone(&game), tx.clone());

        let saved_games_worker = SavedGamesWorker::new(Arc::clone(&game), tx.clone());
        let fls_worker = FlsWorker::new(logger.clone(), Arc::clone(&game), tx.clone());

        let launcher = Rc::new(Self {
            logger,
            log_level,
            app,
            steam,
            game,
            config: RefCell::new(config),
            config_persister,
            saved_servers: saved_servers.map(RefCell::new),
            tx,
            rx,
            mod_directory,
            main_window,
            pong_accumulator: RefCell::new(vec![]),
            cached_users: RefCell::new(cached_users),
            cached_users_persister,
            waiting_for_server_load: Cell::new(false),
            server_loader_worker,
            saved_games_worker,
            fls_worker,
        });

        launcher.main_window.set_on_action({
            let this = Rc::downgrade(&launcher);
            move |action| {
                if let Some(this) = this.upgrade() {
                    this.on_action(action)
                } else {
                    Ok(())
                }
            }
        });

        launcher
    }

    fn run(self: &Rc<Self>, disable_prefetch: bool) {
        self.main_window.show();

        if disable_prefetch {
            self.main_window
                .handle_update(ServerBrowserUpdate::PrefetchDisabled.into());
        } else {
            self.load_server_list();
        }

        self.check_mod_updates();

        self.main_window
            .handle_update(Update::HomeUpdate(HomeUpdate::AuthState(
                self.check_auth_state(),
            )));

        app::add_check(weak_cb!([this = self] => |_| this.background_loop()));

        while self.main_window.window().shown() {
            self.app.wait();
        }
    }

    fn background_loop(&self) {
        loop {
            self.steam.run_callbacks();

            match self.rx.recv() {
                Some(message) => {
                    if let Some(update) = self.process_message(message) {
                        self.main_window.handle_update(update);
                    }
                }
                None => {
                    let mut pong_accumulator = self.pong_accumulator.borrow_mut();
                    if !pong_accumulator.is_empty() {
                        self.main_window.handle_update(Update::ServerBrowser(
                            ServerBrowserUpdate::BatchProcessPongs(
                                pong_accumulator.drain(..).collect(),
                            ),
                        ));
                    }
                    return;
                }
            };
        }
    }

    fn process_message(&self, message: Message) -> Option<Update> {
        match message {
            Message::Update(update) => Some(update),
            Message::ServerList(mut servers) => {
                match servers.as_mut() {
                    Ok(servers) => {
                        self.merge_server_list(servers, Confidence::High);

                        match self.game.load_favorites() {
                            Err(err) => {
                                warn!(self.logger, "Failed to load favorites"; "error" => %err);
                            }
                            Ok(favorites) => {
                                for server in servers.iter_mut() {
                                    server.check_favorites(&favorites);
                                }
                            }
                        }

                        let build_id = self.game.build_id();
                        for server in servers.iter_mut() {
                            server.validate_build(build_id);
                            server.prepare_for_ping();
                        }

                        let mut last_session = self.game.last_session();
                        if let Some(Session::Online(server_ref)) = &mut *last_session {
                            let addr = match server_ref {
                                ServerRef::Known(server) => server.game_addr().unwrap(),
                                ServerRef::Unknown(addr) => *addr,
                            };
                            let server = servers
                                .iter()
                                .filter(|server| server.is_valid())
                                .find(|server| server.game_addr().unwrap() == addr);
                            *server_ref = match server {
                                Some(server) => ServerRef::Known(server.clone()),
                                None => ServerRef::Unknown(addr),
                            };
                            debug!(
                                self.logger,
                                "Determined last session server";
                                "server" => ?server_ref
                            );
                        }
                    }
                    Err(err) => error!(&self.logger, "Error fetching server list"; "error" => %err),
                }
                self.waiting_for_server_load.set(false);
                self.tx
                    .send(Message::Update(Update::HomeUpdate(HomeUpdate::LastSession)));
                Some(Update::ServerBrowser(
                    ServerBrowserUpdate::PopulateServers {
                        payload: servers,
                        done: true,
                    },
                ))
            }
            Message::ServerPong(pong) => {
                self.pong_accumulator.borrow_mut().push(pong);
                None
            }
            Message::Account(account) => {
                if let Ok(account) = &account {
                    if let Err(err) = self.cache_user(account) {
                        warn!(self.logger, "Error saving cached users"; "error" => %err);
                    }
                }

                let platform_user = self.steam.user().ok_or(anyhow!("Steam not running"));
                let fls_account = TaskState::Ready(account);
                let online_capability = self.online_capability(&platform_user, &fls_account);
                let sp_capability = self.sp_capability(&platform_user, &fls_account);
                let auth_state = AuthState {
                    platform_user,
                    fls_account,
                    online_capability,
                    sp_capability,
                };

                Some(Update::HomeUpdate(HomeUpdate::AuthState(auth_state)))
            }
            Message::PlatformReady => {
                self.check_mod_updates();
                None
            }
        }
    }

    fn on_action(self: &Rc<Self>, action: Action) -> Result<()> {
        match action {
            Action::HomeAction(HomeAction::Launch) => self.launch_game(),
            Action::HomeAction(HomeAction::Continue) => self.continue_last_session(),
            Action::HomeAction(HomeAction::SwitchBranch(branch)) => {
                self.update_config(|config| config.branch = branch);
                env::restart_process()?;
                app::quit();
                Ok(())
            }
            Action::HomeAction(HomeAction::ConfigureLogLevel(new_log_level)) => {
                self.update_config(|config| config.log_level = new_log_level);
                if let Some(log_level) = &self.log_level {
                    log_level.store(
                        new_log_level.0.as_usize(),
                        std::sync::atomic::Ordering::Relaxed,
                    );
                }
                Ok(())
            }
            Action::HomeAction(HomeAction::ConfigureBattlEye(use_battleye)) => {
                self.update_config(|config| config.use_battleye = use_battleye);
                Ok(())
            }
            Action::HomeAction(HomeAction::ConfigureUseAllCores(use_all_cores)) => {
                self.update_config(|config| config.use_all_cores = use_all_cores);
                Ok(())
            }
            Action::HomeAction(HomeAction::ConfigureExtraArgs(extra_args)) => {
                self.update_config(|config| config.extra_args = extra_args);
                Ok(())
            }
            Action::HomeAction(HomeAction::ConfigureModMismatchChecks(checks)) => {
                self.update_config(|config| config.mod_mismatch_checks = checks);
                Ok(())
            }
            Action::HomeAction(HomeAction::ConfigureTheme(theme)) => {
                self.update_config(|config| config.theme = theme);
                Ok(())
            }
            Action::HomeAction(HomeAction::RefreshAuthState) => {
                self.main_window
                    .handle_update(Update::HomeUpdate(HomeUpdate::AuthState(
                        self.check_auth_state(),
                    )));
                Ok(())
            }
            Action::ServerBrowser(ServerBrowserAction::LoadServers) => {
                self.load_server_list();
                Ok(())
            }
            Action::ServerBrowser(ServerBrowserAction::JoinServer {
                addr,
                password,
                battleye_required,
            }) => self.join_server(addr, password, battleye_required),
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
                self.update_config(|config| config.server_browser = sb_cfg);
                Ok(())
            }
            Action::ServerBrowser(ServerBrowserAction::ToggleSavedServer { server, idx }) => {
                if server.is_saved() {
                    self.unsave_server(server, idx)
                } else {
                    self.save_server(server, idx)
                }
            }
            Action::SinglePlayer(SinglePlayerAction::ListSavedGames) => {
                Arc::clone(&self.saved_games_worker).list_games()
            }
            Action::SinglePlayer(SinglePlayerAction::NewSavedGame { map_id }) => {
                self.start_new_singleplayer_game(map_id)
            }
            Action::SinglePlayer(SinglePlayerAction::ContinueSavedGame { map_id }) => {
                self.continue_singleplayer_game(map_id)
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
            Action::SinglePlayer(SinglePlayerAction::EditSettings) => {
                self.edit_server_settings();
                Ok(())
            }
            Action::ModManager(ModManagerAction::LoadModList) => {
                self.check_mod_updates();
                let active_mods = self.game.load_mod_list()?;
                self.tx.send(Message::Update(Update::ModManager(
                    ModManagerUpdate::PopulateModList(active_mods),
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
                self.tx.send(Message::Update(Update::ModManager(
                    ModManagerUpdate::PopulateModList(active_mods),
                )));

                Ok(())
            }
            Action::ModManager(ModManagerAction::ExportModList(active_mods)) => {
                let mut dialog = NativeFileChooser::new(FileDialogType::BrowseSaveFile);
                dialog.set_filter(DLG_FILTER_MODLIST);
                dialog.set_directory(&self.game.save_path())?;
                dialog.set_option(FileDialogOptions::SaveAsConfirm);
                dialog.show();

                let mut mod_list_path = dialog.filename();
                if mod_list_path.as_os_str().is_empty() {
                    return Ok(());
                }
                if mod_list_path.extension().is_none() {
                    mod_list_path.set_extension("txt");
                }

                self.game
                    .save_mod_list_to(&mod_list_path, active_mods.iter())
            }
            Action::ModManager(ModManagerAction::UpdateMods) => {
                let outdated_mods = self
                    .game
                    .installed_mods()
                    .iter()
                    .enumerate()
                    .filter(|(_, entry)| entry.needs_update())
                    .map(|(idx, _)| ModRef::Installed(idx))
                    .collect();
                if self.update_mods(outdated_mods) {
                    self.check_mod_updates();
                }
                Ok(())
            }
            Action::ModManager(ModManagerAction::FixModListErrors(mut mod_list)) => {
                if !self.fix_mod_list(&mut mod_list) {
                    dialog::alert_default("Could not fix all of the errors in the mod list.");
                }
                let result = self.game.save_mod_list(mod_list.iter());
                if result.is_ok() {
                    self.tx.send(Message::Update(Update::ModManager(
                        ModManagerUpdate::PopulateModList(mod_list),
                    )));
                }
                result
            }
        }
    }

    fn launch_game(&self) -> Result<()> {
        if !self.can_launch() {
            return Ok(());
        }

        let mod_list = self.game.load_mod_list()?;
        let outdated_mods = self.outdated_active_mods(&mod_list);
        if !self.update_mods(outdated_mods) {
            return Ok(());
        }

        let use_battleye = match self.config().use_battleye {
            BattlEyeUsage::Always(enabled) => enabled,
            BattlEyeUsage::Auto => {
                if let Some(enabled) = self.prompt_battleye() {
                    enabled
                } else {
                    return Ok(());
                }
            }
        };
        let launch_opts = self.launch_options(use_battleye);
        if self.monitor_launch(self.game.launch(launch_opts, &[])?)? {
            app::quit();
        }
        Ok(())
    }

    fn continue_last_session(&self) -> Result<()> {
        if !self.can_launch() {
            return Ok(());
        }

        if !self.steam.can_play_online() {
            match &*self.game.last_session() {
                Some(Session::Online(_)) => bail!(ERR_STEAM_NOT_ONLINE),
                Some(Session::SinglePlayer(_)) => {
                    let cached_users = self.cached_users();
                    let fls_account_id = self
                        .steam
                        .user()
                        .and_then(|user| cached_users.by_platform_id(&user.id))
                        .map(|user| user.account.master_id.as_str());
                    if fls_account_id.is_none() {
                        bail!(ERR_FLS_ACCOUNT_NOT_CACHED);
                    }
                    self.show_offline_singleplayer_bug_warning();
                }
                _ => (),
            }
        }

        let mod_list = self.game.load_mod_list()?;
        let outdated_mods = self.outdated_active_mods(&mod_list);

        if let Some(Session::SinglePlayer(MapRef::Known { map_id })) = &*self.game.last_session() {
            if !self.validate_single_player_mods(mod_list, *map_id)? {
                return Ok(());
            }
        }

        if !self.update_mods(outdated_mods) {
            return Ok(());
        }

        let use_battleye = if let Some(enabled) = self.determine_session_battleye() {
            enabled
        } else {
            return Ok(());
        };
        let launch_opts = self.launch_options(use_battleye);
        if self.monitor_launch(self.game.continue_session(launch_opts)?)? {
            app::quit();
        }
        Ok(())
    }

    fn join_server(
        &self,
        addr: SocketAddr,
        password: Option<String>,
        battleye_required: Option<bool>,
    ) -> Result<()> {
        if !self.can_launch() {
            return Ok(());
        }
        if !self.steam.can_play_online() {
            bail!(ERR_STEAM_NOT_ONLINE);
        }

        let mod_list = self.game.load_mod_list()?;
        let outdated_mods = self.outdated_active_mods(&mod_list);
        if !self.update_mods(outdated_mods) {
            return Ok(());
        }

        let use_battleye = match self.config().use_battleye {
            BattlEyeUsage::Always(enabled) => enabled,
            BattlEyeUsage::Auto => {
                if let Some(enabled) = battleye_required.or_else(|| self.prompt_battleye()) {
                    enabled
                } else {
                    return Ok(());
                }
            }
        };
        let launch_opts = self.launch_options(use_battleye);
        if self.monitor_launch(self.game.join_server(addr, password, launch_opts)?)? {
            app::quit();
        }
        Ok(())
    }

    fn start_new_singleplayer_game(&self, map_id: usize) -> Result<()> {
        if !self.can_launch() {
            return Ok(());
        }
        let cached_users = self.cached_users();
        let fls_account_id = self
            .steam
            .user()
            .and_then(|user| cached_users.by_platform_id(&user.id))
            .map(|user| user.account.master_id.as_str());
        if !self.steam.can_play_online() {
            if fls_account_id.is_none() {
                bail!(ERR_FLS_ACCOUNT_NOT_CACHED);
            }
            self.show_offline_singleplayer_bug_warning();
        }
        self.saved_games_worker
            .clear_progress(map_id, fls_account_id)?;
        self.launch_single_player(map_id, true)
    }

    fn continue_singleplayer_game(&self, map_id: usize) -> Result<()> {
        if !self.can_launch() {
            return Ok(());
        }
        let cached_users = self.cached_users();
        if !self.steam.can_play_online() {
            let cached_user = self
                .steam
                .user()
                .and_then(|user| cached_users.by_platform_id(&user.id));
            if cached_user.is_none() {
                bail!(ERR_FLS_ACCOUNT_NOT_CACHED);
            }
            self.show_offline_singleplayer_bug_warning();
        }
        self.launch_single_player(map_id, false)
    }

    fn load_server_list(&self) {
        if let Some(servers) = self.saved_servers.as_ref() {
            let servers = servers.borrow();
            if !servers.is_empty() {
                self.main_window.handle_update(Update::ServerBrowser(
                    ServerBrowserUpdate::PopulateServers {
                        payload: Ok(servers.iter().cloned().collect()),
                        done: false,
                    },
                ));
            }
        }
        self.server_loader_worker.load_servers();
    }

    fn merge_server_list(&self, servers: &mut Vec<Server>, min_confidence: Confidence) {
        struct MergeCandidate {
            list_idx: usize,
            saved_id: Uuid,
            similarity: Similarity,
        }

        let saved_servers = match self.saved_servers.as_ref() {
            Some(saved) => saved,
            None => return,
        };
        let mut saved_servers = saved_servers.borrow_mut();

        debug!(
            self.logger,
            "Merging server lists";
            "num_listed" => servers.len(),
            "num_saved" => saved_servers.len(),
        );

        let mut merge_candidates = Vec::new();
        let mut matches = HashSet::new();
        for (list_idx, list_server) in servers.iter().enumerate() {
            matches.extend(saved_servers.by_id(&list_server.id));
            matches.extend(saved_servers.by_name(&list_server.name));
            matches.extend(saved_servers.by_addr(list_server.ip, list_server.port));
            for saved_id in matches.drain() {
                let score = saved_servers[saved_id].similarity(list_server);
                merge_candidates.push(MergeCandidate {
                    list_idx,
                    saved_id,
                    similarity: score,
                });
            }
        }

        merge_candidates.sort_by(|lhs, rhs| rhs.similarity.cmp(&lhs.similarity));
        let mut tombstones = Vec::new();
        for candidate in merge_candidates {
            if !candidate.similarity.satisfies(min_confidence) {
                break;
            }
            let list_server = &mut servers[candidate.list_idx];
            let saved_server = &mut saved_servers[candidate.saved_id];
            if list_server.tombstone || saved_server.merged {
                continue;
            }
            trace!(
                self.logger,
                "Merging servers";
                "listed" => ?list_server,
                "saved" => ?saved_server,
                "similarity" => ?candidate.similarity,
            );
            saved_server.merge_from(list_server);
            tombstones.push(candidate.list_idx);
        }

        if !tombstones.is_empty() {
            saved_servers.reindex();
            if let Err(err) = saved_servers.save() {
                warn!(self.logger, "Error saving merged servers"; "error" => %err);
            }
        }

        tombstones.sort();
        for tombstone_idx in tombstones.into_iter().rev() {
            servers.swap_remove(tombstone_idx);
        }

        servers.extend(saved_servers.iter().cloned());
    }

    fn save_server(&self, server: Server, idx: Option<usize>) -> Result<()> {
        let servers = self.saved_servers.as_ref().unwrap();
        let mut servers = servers.borrow_mut();
        let id = servers.add(server);
        servers.save()?;
        self.tx.send(Message::Update(Update::ServerBrowser(
            ServerBrowserUpdate::UpdateServer {
                idx,
                server: servers[id].clone(),
            },
        )));
        Ok(())
    }

    fn unsave_server(&self, mut server: Server, idx: Option<usize>) -> Result<()> {
        let servers = self.saved_servers.as_ref().unwrap();
        let mut servers = servers.borrow_mut();
        servers.remove(server.saved_id.unwrap());
        servers.save()?;

        server.saved_id = None;
        if !server.merged {
            server.tombstone = true;
        }
        self.tx.send(Message::Update(Update::ServerBrowser(
            ServerBrowserUpdate::UpdateServer { idx, server },
        )));
        Ok(())
    }

    fn check_auth_state(&self) -> AuthState {
        let platform_user = self.steam.user().ok_or(anyhow!("Steam not running"));
        let fls_account = match &platform_user {
            Ok(user) => {
                if let Some(cached) = self.cached_users().by_platform_id(&user.id).as_deref() {
                    TaskState::Ready(Ok(cached.account.clone()))
                } else {
                    if self.steam.can_play_online() {
                        TaskState::Pending
                    } else {
                        TaskState::Ready(Err(anyhow!("Steam in offline mode")))
                    }
                }
            }
            Err(err) => TaskState::Ready(Err(anyhow!(err.to_string()))),
        };
        let online_capability = self.online_capability(&platform_user, &fls_account);
        let sp_capability = self.sp_capability(&platform_user, &fls_account);

        if let TaskState::Pending = &fls_account {
            Arc::clone(&self.fls_worker).login_with_steam(&*self.steam.auth_ticket().unwrap());
        }

        AuthState {
            platform_user,
            fls_account,
            online_capability,
            sp_capability,
        }
    }

    fn online_capability(
        &self,
        platform_user: &Result<PlatformUser>,
        fls_account: &TaskState<Result<Account>>,
    ) -> TaskState<Capability> {
        match &platform_user {
            Err(err) => TaskState::Ready(Err(anyhow!(err.to_string()))),
            Ok(_) => {
                if !self.steam.can_play_online() {
                    TaskState::Ready(Err(anyhow!("Steam in offline mode")))
                } else {
                    match &fls_account {
                        TaskState::Pending => TaskState::Pending,
                        TaskState::Ready(Ok(_)) => TaskState::Ready(Ok(())),
                        TaskState::Ready(Err(_)) => TaskState::Ready(Err(anyhow!("FLS error"))),
                    }
                }
            }
        }
    }

    fn sp_capability(
        &self,
        platform_user: &Result<PlatformUser>,
        fls_account: &TaskState<Result<Account>>,
    ) -> TaskState<Capability> {
        match &platform_user {
            Err(err) => TaskState::Ready(Err(anyhow!(err.to_string()))),
            Ok(_) => match &fls_account {
                TaskState::Pending => TaskState::Pending,
                TaskState::Ready(Ok(_)) => TaskState::Ready(Ok(())),
                TaskState::Ready(Err(_)) => {
                    TaskState::Ready(Err(anyhow!("FLS account not cached")))
                }
            },
        }
    }

    fn can_launch(&self) -> bool {
        if self.steam.can_launch() {
            return true;
        }

        let monitor = self.task_monitor(
            "Waiting for Steam",
            "Please ensure that Steam is running\nand you have Conan Exiles in your library.",
            "Cancel",
        );
        monitor.show();

        let should_poll = Rc::new(Cell::new(true));
        app::add_timeout3(1.0, {
            let logger = self.logger.clone();
            weak_cb!(
                [should_poll] => |handle| {
                    let poll_skipped = should_poll.replace(true);
                    trace!(logger, "Firing steam poll timer"; "poll_skipped" => poll_skipped);
                    app::repeat_timeout3(1.0, handle);
                    app::awake();
                }
            )
        });
        loop {
            if should_poll.replace(false) {
                if self.steam.can_launch() {
                    return true;
                }
            }
            if monitor.result().is_some() {
                return false;
            }
            if !self.app.wait() {
                return false;
            }
        }
    }

    fn prompt_battleye(&self) -> Option<bool> {
        let battleye_dialog = Dialog::default(
            self.main_window.window(),
            "Enable BattlEye?",
            "BUGLE could not determine whether BattlEye is required for this session.\nStart Conan \
            Exiles with BattlEye enabled or disabled?",
            &[("Enabled", true), ("Disabled", false)]
        );
        battleye_dialog.show();
        loop {
            let result = battleye_dialog.result();
            if result.is_some() {
                return result;
            }
            if !self.app.wait() {
                return None;
            }
        }
    }

    fn determine_session_battleye(&self) -> Option<bool> {
        match self.last_session_battleye() {
            SessionBattlEyeUsage::Resolved(enabled) => return Some(enabled),
            SessionBattlEyeUsage::AskUser => return self.prompt_battleye(),
            _ => (),
        };

        self.waiting_for_server_load.set(true);
        let monitor = self.task_monitor(
            "Checking server",
            "Determining if the server requires BattlEye",
            "Skip",
        );
        monitor.show();
        loop {
            if monitor.result().is_some() {
                break;
            }
            if !self.app.wait() {
                return None;
            }
            match self.last_session_battleye() {
                SessionBattlEyeUsage::Resolved(enabled) => return Some(enabled),
                SessionBattlEyeUsage::AskUser => break,
                _ => (),
            };
        }
        drop(monitor);

        self.prompt_battleye()
    }

    fn last_session_battleye(&self) -> SessionBattlEyeUsage {
        match self.config().use_battleye {
            BattlEyeUsage::Always(enabled) => SessionBattlEyeUsage::Resolved(enabled),
            BattlEyeUsage::Auto => match &*self.game.last_session() {
                Some(Session::Online(server_ref)) => match server_ref {
                    ServerRef::Known(server) => {
                        SessionBattlEyeUsage::Resolved(server.general.battleye_required)
                    }
                    _ => {
                        if self.server_loader_worker.is_loading()
                            || self.waiting_for_server_load.get()
                        {
                            SessionBattlEyeUsage::WaitForServerLoader
                        } else {
                            SessionBattlEyeUsage::AskUser
                        }
                    }
                },
                Some(_) => SessionBattlEyeUsage::Resolved(false),
                None => SessionBattlEyeUsage::AskUser,
            },
        }
    }

    fn monitor_launch(&self, mut launch: Launch) -> Result<bool> {
        if let TaskState::Ready(()) = launch.poll()? {
            return Ok(true);
        }

        let monitor = self.task_monitor(
            "Launching Conan Exiles",
            "Waiting for Conan Exiles to start...",
            "Cancel",
        );
        monitor.show();

        let should_poll = Rc::new(Cell::new(true));
        app::add_timeout3(1.0, {
            let logger = self.logger.clone();
            weak_cb!(
                [should_poll] => |handle| {
                    let poll_skipped = should_poll.replace(true);
                    trace!(logger, "Firing launch poll timer"; "poll_skipped" => poll_skipped);
                    app::repeat_timeout3(1.0, handle);
                    app::awake();
                }
            )
        });
        loop {
            if should_poll.replace(false) {
                if let TaskState::Ready(()) = launch.poll()? {
                    return Ok(true);
                }
            }
            if monitor.result().is_some() {
                launch.cancel();
                return Ok(false);
            }
            if !self.app.wait() {
                return Ok(true);
            }
        }
    }

    fn launch_single_player(&self, map_id: usize, skip_mod_checks: bool) -> Result<()> {
        let mod_list = self.game.load_mod_list()?;
        let outdated_mods = self.outdated_active_mods(&mod_list);

        if !skip_mod_checks && !self.validate_single_player_mods(mod_list, map_id)? {
            return Ok(());
        }

        if !self.update_mods(outdated_mods) {
            return Ok(());
        }

        let use_battleye =
            if let BattlEyeUsage::Always(true) = self.config().use_battleye { true } else { false };
        let launch_opts = self.launch_options(use_battleye);
        if self.monitor_launch(self.game.launch_single_player(map_id, launch_opts)?)? {
            app::quit();
        }
        Ok(())
    }

    fn validate_single_player_mods(&self, mod_list: Vec<ModRef>, map_id: usize) -> Result<bool> {
        fn push_name(s: &mut String, entry: &ModEntry) {
            if let Ok(info) = entry.info.as_ref() {
                s.push_str(&info.name);
            } else {
                s.push_str(&format!("??? ({})", entry.pak_path.display()));
            }
        }
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
                    ModRef::Installed(idx) => push_name(&mut result, &mods[idx]),
                    ModRef::Custom(entry) => push_name(&mut result, &entry),
                    ModRef::UnknownFolder(folder) => result.push_str(&format!("??? ({})", folder)),
                    ModRef::UnknownPakPath(path) => {
                        result.push_str(&format!("??? ({})", path.display()))
                    }
                };
            }
            result
        }

        if let ModMismatchChecks::Disabled = self.config().mod_mismatch_checks {
            return Ok(true);
        }

        if let Some(mismatch) = self.detect_single_player_mod_mismatch(mod_list, map_id)? {
            let installed_mods = self.game.installed_mods();
            let prompt = format!(
                "{}{}{}",
                PROMPT_SP_MOD_MISMATCH,
                join_mod_names(TXT_MISSING_MODS, installed_mods, mismatch.missing_mods),
                join_mod_names(TXT_ADDED_MODS, installed_mods, mismatch.added_mods),
            );
            Ok(prompt_confirm(&prompt))
        } else {
            Ok(true)
        }
    }

    fn detect_single_player_mod_mismatch(
        &self,
        mod_list: Vec<ModRef>,
        map_id: usize,
    ) -> Result<Option<ModMismatch>> {
        let installed_mods = self.game.installed_mods();
        let mut active_mods: HashSet<ModRef> = mod_list.into_iter().collect();

        let db_path = self.game.in_progress_game_path(map_id);
        let db_metadata = std::fs::metadata(&db_path)?;
        let mod_controllers =
            if db_metadata.len() != 0 { list_mod_controllers(db_path)? } else { Vec::new() };

        let mut required_folders = HashMap::new();
        for controller in mod_controllers {
            if let Some(captures) = MOD_CTRL_FOLDER_REGEX.captures(&controller) {
                let folder = captures.get(1).unwrap().as_str();
                required_folders.insert(folder.to_string(), false);
            }
        }

        let mut added_mods = HashSet::new();
        for mod_ref in active_mods.drain() {
            let info = installed_mods
                .get(&mod_ref)
                .and_then(|entry| entry.info.as_ref().ok());
            if let Some(info) = info {
                if let Some(active) = required_folders.get_mut(&info.folder_name) {
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

    fn outdated_active_mods(&self, mod_list: &Vec<ModRef>) -> Vec<ModRef> {
        self.check_mod_updates();

        let installed_mods = self.game.installed_mods();
        let mut outdated_mods = Vec::new();
        for mod_ref in mod_list {
            if let Some(entry) = installed_mods.get(mod_ref) {
                if entry.needs_update() {
                    outdated_mods.push(mod_ref.clone());
                }
            }
        }

        outdated_mods
    }

    fn update_mods(&self, outdated_mods: Vec<ModRef>) -> bool {
        if outdated_mods.is_empty() || !Rc::clone(&self.mod_directory).can_update() {
            return true;
        }

        let installed_mods = self.game.installed_mods();

        let dialog =
            ModUpdateSelectionDialog::new(self.main_window.window(), installed_mods, outdated_mods);
        let mods_to_update = match dialog.run() {
            None => return false,
            Some(mods) => mods,
        };
        if mods_to_update.is_empty() {
            return true;
        }

        let dialog = ModUpdateProgressDialog::new(
            self.main_window.window(),
            installed_mods,
            mods_to_update,
            Rc::clone(&self.mod_directory),
        );
        dialog.run();

        self.main_window.window().shown()
    }

    fn fix_mod_list(&self, mod_list: &mut Vec<ModRef>) -> bool {
        let installed_mods = self.game.installed_mods();
        let mut available_set = BitVec::from_elem(installed_mods.len(), true);

        for mod_ref in mod_list.iter() {
            if let ModRef::Installed(idx) = mod_ref {
                available_set.set(*idx, false);
            }
        }

        let mut fixed_all = true;
        for mod_ref in mod_list.iter_mut() {
            let pak_path = match mod_ref {
                ModRef::UnknownPakPath(path) => path,
                _ => continue,
            };
            let fixed_idx = installed_mods.iter().enumerate().find_map(|(idx, entry)| {
                if !available_set[idx] {
                    return None;
                }
                let root = installed_mods.root_for(entry.provenance)?;
                let suffix = entry.pak_path.strip_prefix(root).ok()?;
                if pak_path.ends_with(suffix) {
                    Some(idx)
                } else {
                    None
                }
            });
            if let Some(idx) = fixed_idx {
                *mod_ref = ModRef::Installed(idx);
                available_set.set(idx, false);
            } else {
                fixed_all = false;
            }
        }

        fixed_all
    }

    fn edit_server_settings(&self) {
        let settings = match self.game.load_server_settings() {
            Ok(settings) => settings,
            Err(err) => {
                alert_error(ERR_LOADING_SETTINGS, &err);
                return;
            }
        };
        let dialog = ServerSettingsDialog::new(settings);
        let Some(settings) = dialog.run() else { return };
        if let Err(err) = self.game.save_server_settings(settings) {
            alert_error(ERR_SAVING_SETTINGS, &err);
        }
    }

    fn config(&self) -> Ref<Config> {
        self.config.borrow()
    }

    fn update_config(&self, mutator: impl FnOnce(&mut Config)) {
        let mut config = self.config.borrow_mut();
        mutator(&mut config);
        if let Err(err) = self.config_persister.save(&config) {
            warn!(self.logger, "Error while saving the configuration"; "error" => err.to_string());
        }
    }

    fn launch_options(&self, use_battleye: bool) -> LaunchOptions {
        let config = self.config();
        LaunchOptions {
            enable_battleye: use_battleye,
            use_all_cores: config.use_all_cores,
            extra_args: config.extra_args.clone(),
        }
    }

    fn cached_users(&self) -> Ref<CachedUsers> {
        self.cached_users.borrow()
    }

    fn cache_user(&self, account: &Account) -> Result<()> {
        let mut cached_users = self.cached_users.borrow_mut();
        cached_users.insert(CachedUser::new(account.clone()));
        (self.cached_users_persister)(&self.game, &*cached_users)
    }

    fn check_mod_updates(&self) {
        if !self.steam.can_launch() {
            return;
        }

        for entry in self.game.installed_mods().iter() {
            match Rc::clone(&self.mod_directory).needs_update(entry) {
                Ok(needs_update) => entry.set_needs_update(needs_update),
                Err(err) => warn!(
                    self.logger,
                    "Error checking whether mod needs update";
                    "mod_name" => entry.info.as_ref().map(|info| info.name.as_str()).unwrap_or("???"),
                    "pak_path" => ?entry.pak_path,
                    "error" => %err,
                ),
            }
        }
    }

    fn task_monitor(&self, title: &str, message: &str, button: &str) -> Dialog<()> {
        Dialog::new(
            self.main_window.window(),
            title,
            message,
            320,
            90,
            &[(button, ())],
        )
    }

    fn show_message(&self, title: &str, message: &str, button: &str, width: i32, height: i32) {
        let dialog = Dialog::new(
            self.main_window.window(),
            title,
            message,
            width,
            height,
            &[(button, ())],
        );
        dialog.run();
    }

    fn show_offline_singleplayer_bug_warning(&self) {
        self.show_message(
            "Bug Warning",
            "ATTENTION: Conan Exiles currently has a bug that doesn't let you\n \
            automatically jump into a single-player game while in offline mode.\nWhen \
            the game starts, it will display \"Failed to Log In\" error message.\nYou \
            can still play offline, but you have to click on \"Singleplayer\"\n\
            and then on \"Continue\" in the main menu.",
            "OK",
            480,
            160,
        );
    }
}

enum SessionBattlEyeUsage {
    Resolved(bool),
    WaitForServerLoader,
    AskUser,
}

struct ModMismatch {
    missing_mods: HashSet<ModRef>,
    added_mods: HashSet<ModRef>,
}

const PROMPT_SP_MOD_MISMATCH: &str =
    "It looks like your mod list doesn't match this game. Launch anyway?";
const TXT_MISSING_MODS: &str = "Missing mods:";
const TXT_ADDED_MODS: &str = "Added mods:";
const DLG_FILTER_MODLIST: &str = "Mod List Files\t*.txt";
const ERR_STEAM_NOT_ONLINE: &str = "Steam is in offline mode. Online play is disabled.";
const ERR_FLS_ACCOUNT_NOT_CACHED: &str =
    "Steam is offline and the game has not stored your FLS account info. You need to start the \
    game in online mode at least once before you can play offline.";
const ERR_LOADING_SETTINGS: &str = "Error while loading the game settings.";
const ERR_SAVING_SETTINGS: &str = "Error while saving the game settings.";

lazy_static! {
    static ref MOD_CTRL_FOLDER_REGEX: Regex = Regex::new("/Game/Mods/([^/]+)/.*").unwrap();
}

#[tokio::main]
async fn main() {
    let mut args = pico_args::Arguments::from_env();
    let disable_prefetch = args.contains("--no-prefetch");
    let log_level_override = args
        .opt_value_from_fn(["-l", "--log-level"], |s| {
            FilterLevel::from_str(s).map_err(|_| "")
        })
        .ok()
        .unwrap_or_default();
    let log_level = Arc::new(AtomicUsize::new(
        log_level_override
            .unwrap_or(logger::DEFAULT_LOG_LEVEL)
            .as_usize(),
    ));
    let (root_logger, log_guard) = create_root_logger(&log_level);

    let config_persister: Box<dyn ConfigPersister + Send + Sync> = match IniConfigPersister::new() {
        Ok(persister) => {
            info!(
                root_logger,
                "Opened persistent config file";
                "path" => persister.path().display()
            );
            Box::new(persister)
        }
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
    let mut config = config_persister.load().unwrap_or_else(|err| {
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
    Theme::from_config(config.theme).apply();
    gui::glyph::add_symbols();

    let mut steam = match Steam::locate(&root_logger) {
        Some(steam) => steam,
        None => {
            dialog::alert_default(
                "Cannot locate Steam installation. Please verify that you have Steam installed and\
                 try again.",
            );
            return;
        }
    };
    let can_switch_branch = steam
        .locate_game(match config.branch {
            Branch::Main => Branch::PublicBeta,
            Branch::PublicBeta => Branch::Main,
        })
        .is_ok();
    let game = steam
        .locate_game(config.branch)
        .and_then(|loc| steam.init_game(loc));
    let game = match game {
        Ok(game) => game,
        Err(err) => {
            error!(root_logger, "Error with Conan Exiles installation"; "error" => %err);
            if can_switch_branch {
                let (this_name, other_name, other_branch) = match config.branch {
                    Branch::Main => ("Live", "TestLive", Branch::PublicBeta),
                    Branch::PublicBeta => ("TestLive", "Live", Branch::Main),
                };
                let should_switch = gui::prompt_confirm(&format!(
                    "There was a problem with your {} installation of Conan Exiles.\nHowever, \
                        BUGLE has detected that the {} installation is also available.\nDo you \
                        want to restart BUGLE and switch to {1} installation?",
                    this_name, other_name,
                ));
                if should_switch {
                    config.branch = other_branch;
                    if let Err(err) = config_persister.save(&config) {
                        error!(
                            root_logger,
                            "Error switching to other branch";
                            "branch" => ?other_branch,
                            "error" => %err,
                        );
                    }
                    if let Err(err) = env::restart_process() {
                        error!(
                            root_logger,
                            "Error restarting BUGLE";
                            "error" => %err,
                        );
                    }
                }
            } else {
                gui::alert_error(
                    "There was a problem with your Conan Exiles installation.",
                    &err,
                );
            }
            return;
        }
    };

    if game.needs_update() {
        if gui::prompt_confirm(
            "Conan Exiles needs to be updated. Do you want to close BUGLE?\n\
                Note that closing BUGLE will not automatically start the update.",
        ) {
            return;
        }
    }

    if !game.battleye_installed().unwrap_or(true)
        && (config.use_battleye != BattlEyeUsage::Always(false))
    {
        if gui::prompt_confirm(
            "BattlEye is not installed on your computer. Do you want to configure BUGLE\nto launch \
            Conan Exiles with BattlEye disabled?",
        ) {
            config.use_battleye = BattlEyeUsage::Always(false);
            if let Err(err) = config_persister.save(&config) {
                warn!(root_logger, "Error while saving the configuration"; "error" => %err);
            }
        }
    }

    let saved_servers = match SavedServers::new() {
        Ok(mut servers) => {
            if let Err(err) = servers.load() {
                warn!(
                    root_logger,
                    "Error loading the saved servers list";
                    "path" => servers.path().display(),
                    "error" => %err,
                );
            }
            Some(servers)
        }
        Err(err) => {
            warn!(root_logger, "Error opening the saved servers list"; "error" => %err);
            None
        }
    };

    let launcher = Launcher::new(
        root_logger.clone(),
        if log_level_override.is_none() { Some(log_level) } else { None },
        can_switch_branch,
        app,
        steam,
        game,
        config,
        config_persister,
        saved_servers,
    );
    launcher.run(disable_prefetch);

    info!(root_logger, "Shutting down launcher");
    drop(log_guard);
}
