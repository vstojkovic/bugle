use std::cell::{Cell, Ref};
use std::net::SocketAddr;
use std::rc::Rc;
use std::sync::Arc;

use anyhow::{bail, Result};
use fltk::app;
use slog::{trace, Logger};

use crate::auth_manager::AuthManager;
use crate::config::{BattlEyeUsage, ConfigManager};
use crate::game::platform::steam::SteamClient;
use crate::game::settings::server::ServerSettings;
use crate::game::{Game, Launch, LaunchOptions, MapRef, ServerRef, Session};
use crate::gui::Dialog;
use crate::mod_manager::ModManager;
use crate::saved_games_manager::SavedGamesManager;
use crate::server_manager::ServerManager;
use crate::util::weak_cb;
use crate::workers::TaskState;

pub struct Launcher {
    logger: Logger,
    config: Rc<ConfigManager>,
    game: Arc<Game>,
    steam: Rc<SteamClient>,
    auth: Rc<AuthManager>,
    servers: Rc<ServerManager>,
    mods: Rc<ModManager>,
    saves: Rc<SavedGamesManager>,
}

pub struct ConnectionInfo {
    pub addr: SocketAddr,
    pub password: Option<String>,
    pub battleye_required: Option<bool>,
}

impl Launcher {
    pub fn new(
        logger: &Logger,
        config: Rc<ConfigManager>,
        game: Arc<Game>,
        steam: Rc<SteamClient>,
        auth: Rc<AuthManager>,
        servers: Rc<ServerManager>,
        mods: Rc<ModManager>,
        saves: Rc<SavedGamesManager>,
    ) -> Rc<Self> {
        Rc::new(Self {
            logger: logger.clone(),
            config,
            game,
            steam,
            auth,
            servers,
            mods,
            saves,
        })
    }

    pub fn launch_game(&self) -> Result<()> {
        if !self.can_launch() {
            return Ok(());
        }

        let outdated_mods = self.mods.outdated_active_mods()?;
        self.mods.update_mods(outdated_mods);

        if app::should_program_quit() {
            return Ok(());
        }

        let use_battleye = match self.config.get().use_battleye {
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

    pub fn continue_last_session(&self) -> Result<()> {
        if !self.can_launch() {
            return Ok(());
        }

        if !self.steam.can_play_online() {
            match &*self.game.last_session() {
                Some(Session::Online(_)) => bail!(ERR_STEAM_NOT_ONLINE),
                Some(Session::SinglePlayer(_)) => {
                    let fls_account_id = self.auth.cached_user();
                    if fls_account_id.is_none() {
                        bail!(ERR_FLS_ACCOUNT_NOT_CACHED);
                    }
                    self.show_offline_singleplayer_bug_warning();
                }
                _ => (),
            }
        }

        let outdated_mods = self.mods.outdated_active_mods()?;

        if let Some(Session::SinglePlayer(MapRef::Known { map_id })) = &*self.game.last_session() {
            if !self.mods.validate_single_player_mods(*map_id)? {
                return Ok(());
            }
        }

        self.mods.update_mods(outdated_mods);
        if app::should_program_quit() {
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

    pub fn join_server(&self, conn_info: ConnectionInfo) -> Result<()> {
        let ConnectionInfo {
            addr,
            password,
            battleye_required,
        } = conn_info;
        if !self.can_launch() {
            return Ok(());
        }
        if !self.steam.can_play_online() {
            bail!(ERR_STEAM_NOT_ONLINE);
        }

        let outdated_mods = self.mods.outdated_active_mods()?;
        self.mods.update_mods(outdated_mods);

        if app::should_program_quit() {
            return Ok(());
        }

        let use_battleye = match self.config.get().use_battleye {
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

    pub fn start_new_singleplayer_game(
        &self,
        map_id: usize,
        settings: ServerSettings,
    ) -> Result<()> {
        if !self.can_launch() {
            return Ok(());
        }
        let fls_account_id = self
            .auth
            .cached_user()
            .map(|user| Ref::map(user, |user| user.account.master_id.as_str()));
        if !self.steam.can_play_online() {
            if fls_account_id.is_none() {
                bail!(ERR_FLS_ACCOUNT_NOT_CACHED);
            }
            self.show_offline_singleplayer_bug_warning();
        }
        self.game.save_server_settings(settings)?;
        self.saves
            .clear_progress(map_id, fls_account_id.as_deref())?;
        self.launch_single_player(map_id, true)
    }

    pub fn continue_singleplayer_game(&self, map_id: usize) -> Result<()> {
        if !self.can_launch() {
            return Ok(());
        }
        if !self.steam.can_play_online() {
            let cached_user = self.auth.cached_user();
            if cached_user.is_none() {
                bail!(ERR_FLS_ACCOUNT_NOT_CACHED);
            }
            self.show_offline_singleplayer_bug_warning();
        }
        self.launch_single_player(map_id, false)
    }

    fn launch_single_player(&self, map_id: usize, skip_mod_checks: bool) -> Result<()> {
        let outdated_mods = self.mods.outdated_active_mods()?;

        if !skip_mod_checks && !self.mods.validate_single_player_mods(map_id)? {
            return Ok(());
        }

        self.mods.update_mods(outdated_mods);
        if app::should_program_quit() {
            return Ok(());
        }

        let use_battleye = if let BattlEyeUsage::Always(true) = self.config.get().use_battleye {
            true
        } else {
            false
        };
        let launch_opts = self.launch_options(use_battleye);
        if self.monitor_launch(self.game.launch_single_player(map_id, launch_opts)?)? {
            app::quit();
        }
        Ok(())
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
            app::wait();
            if app::should_program_quit() {
                return false;
            }
        }
    }

    fn prompt_battleye(&self) -> Option<bool> {
        let battleye_dialog = Dialog::default(
            fltk::app::first_window().as_ref().unwrap(),
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
            app::wait();
            if app::should_program_quit() {
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
            app::wait();
            if app::should_program_quit() {
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
        match self.config.get().use_battleye {
            BattlEyeUsage::Always(enabled) => SessionBattlEyeUsage::Resolved(enabled),
            BattlEyeUsage::Auto => match &*self.game.last_session() {
                Some(Session::Online(server_ref)) => match server_ref {
                    ServerRef::Known(server) => {
                        SessionBattlEyeUsage::Resolved(server.general.battleye_required)
                    }
                    _ => {
                        if self.servers.is_loading() {
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

    fn launch_options(&self, use_battleye: bool) -> LaunchOptions {
        let config = self.config.get();
        LaunchOptions {
            enable_battleye: use_battleye,
            use_all_cores: config.use_all_cores,
            extra_args: config.extra_args.clone(),
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
            app::wait();
            if app::should_program_quit() {
                return Ok(true);
            }
        }
    }

    fn task_monitor(&self, title: &str, message: &str, button: &str) -> Dialog<()> {
        Dialog::new(
            fltk::app::first_window().as_ref().unwrap(),
            title,
            message,
            320,
            90,
            &[(button, ())],
        )
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

    fn show_message(&self, title: &str, message: &str, button: &str, width: i32, height: i32) {
        let dialog = Dialog::new(
            fltk::app::first_window().as_ref().unwrap(),
            title,
            message,
            width,
            height,
            &[(button, ())],
        );
        dialog.show();
        dialog.run();
    }
}
enum SessionBattlEyeUsage {
    Resolved(bool),
    WaitForServerLoader,
    AskUser,
}

const ERR_STEAM_NOT_ONLINE: &str = "Steam is in offline mode. Online play is disabled.";
const ERR_FLS_ACCOUNT_NOT_CACHED: &str =
    "Steam is offline and the game has not stored your FLS account info. You need to start the \
    game in online mode at least once before you can play offline.";
