#![cfg_attr(windows, windows_subsystem = "windows")]

use std::cell::RefCell;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

use dynabus::Bus;
use fltk::app::{self, App};
use fltk::dialog;
use fltk::prelude::WindowExt;
use game::platform::steam::SteamModDirectory;
use slog::{error, info, warn, FilterLevel, Logger};

mod auth;
mod auth_manager;
mod battleye;
mod bus;
mod config;
mod env;
mod game;
mod gui;
mod launcher;
mod logger;
mod mod_manager;
mod net;
mod parser_utils;
mod saved_games_manager;
mod server_manager;
mod servers;
mod util;
mod workers;

use self::auth_manager::AuthManager;
use self::bus::AppBus;
use self::config::{
    BattlEyeUsage, ConfigManager, ConfigPersister, IniConfigPersister, TransientConfig,
};
use self::game::platform::steam::{Steam, SteamClient};
use self::game::{Branch, Game};
use self::gui::theme::Theme;
use self::gui::LauncherWindow;
use self::launcher::Launcher;
use self::logger::create_root_logger;
use self::mod_manager::ModManager;
use self::saved_games_manager::SavedGamesManager;
use self::server_manager::ServerManager;
use self::util::weak_cb;

#[derive(dynabus::Event)]
pub struct Idle;

struct LauncherApp {
    app: App,
    bus: Rc<RefCell<AppBus>>,
    steam: Rc<SteamClient>,
    auth: Rc<AuthManager>,
    servers: Rc<ServerManager>,
    mods: Rc<ModManager>,
    main_window: LauncherWindow,
}

impl LauncherApp {
    fn new(
        logger: Logger,
        log_level: Option<Arc<AtomicUsize>>,
        can_switch_branch: bool,
        app: App,
        steam: Steam,
        game: Game,
        config: Rc<ConfigManager>,
    ) -> Rc<Self> {
        let game = Arc::new(game);
        let bus = bus::bus();

        let steam = steam.init_client(&*game, Rc::clone(&bus));
        let mod_directory = SteamModDirectory::new(
            &logger,
            Rc::clone(&steam),
            bus.borrow().sender().clone(),
            game.installed_mods(),
        );

        let auth = AuthManager::new(
            &logger,
            Rc::clone(&bus),
            Arc::clone(&game),
            Rc::clone(&steam),
        );

        let servers = ServerManager::new(&logger, Rc::clone(&bus), Arc::clone(&game));

        let mods = ModManager::new(
            &logger,
            Rc::clone(&config),
            Rc::clone(&bus),
            Arc::clone(&game),
            Rc::<SteamModDirectory>::clone(&mod_directory),
        );

        let saves = SavedGamesManager::new(Rc::clone(&bus), Arc::clone(&game));

        let launcher = Launcher::new(
            &logger,
            Rc::clone(&config),
            Arc::clone(&game),
            Rc::clone(&steam),
            Rc::clone(&auth),
            Rc::clone(&servers),
            Rc::clone(&mods),
            Rc::clone(&saves),
        );

        let main_window = LauncherWindow::new(
            &logger,
            Rc::clone(&bus),
            Arc::clone(&game),
            Rc::clone(&config),
            log_level,
            Rc::clone(&auth),
            Rc::clone(&launcher),
            Rc::clone(&servers),
            Rc::clone(&saves),
            Rc::clone(&mods),
            can_switch_branch,
        );

        let this = Rc::new(Self {
            bus,
            app,
            steam,
            auth,
            servers,
            mods,
            main_window,
        });

        this
    }

    fn run(self: &Rc<Self>, disable_prefetch: bool) {
        self.main_window.show();

        if !disable_prefetch {
            self.servers.load_server_list();
        }

        self.mods.check_mod_updates();
        self.auth.check_auth_state();

        app::add_check(weak_cb!([this = self] => |_| this.background_loop()));

        while self.main_window.window().shown() && !app::should_program_quit() {
            self.app.wait();
        }
    }

    fn background_loop(&self) {
        loop {
            self.steam.run_callbacks();

            let bus = self.bus.borrow();
            if !bus.recv().unwrap().unwrap_or_default() {
                bus.publish(Idle);
                return;
            }
        }
    }
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

    let config_persister: Box<dyn ConfigPersister> = match IniConfigPersister::new() {
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
    let config = ConfigManager::new(&root_logger, config_persister);

    if log_level_override.is_none() {
        log_level.store(
            config.get().log_level.0.as_usize(),
            std::sync::atomic::Ordering::Relaxed,
        );
    }

    let app = App::default();
    Theme::from_config(config.get().theme).apply();
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
        .locate_game(match config.get().branch {
            Branch::Live => Branch::TestLive,
            Branch::TestLive => Branch::Live,
        })
        .is_ok();
    let game = steam
        .locate_game(config.get().branch)
        .and_then(|loc| steam.init_game(loc));
    let game = match game {
        Ok(game) => game,
        Err(err) => {
            error!(root_logger, "Error with Conan Exiles installation"; "error" => %err);
            if can_switch_branch {
                let (this_name, other_name, other_branch) = match config.get().branch {
                    Branch::Live => ("Live", "TestLive", Branch::TestLive),
                    Branch::TestLive => ("TestLive", "Live", Branch::Live),
                };
                let should_switch = gui::prompt_confirm(&format!(
                    "There was a problem with your {} installation of Conan Exiles.\nHowever, \
                        BUGLE has detected that the {} installation is also available.\nDo you \
                        want to restart BUGLE and switch to {1} installation?",
                    this_name, other_name,
                ));
                if should_switch {
                    if let Err(err) = config.try_update(|config| config.branch = other_branch) {
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
        && (config.get().use_battleye != BattlEyeUsage::Always(false))
    {
        if gui::prompt_confirm(
            "BattlEye is not installed on your computer. Do you want to configure BUGLE\nto launch \
            Conan Exiles with BattlEye disabled?",
        ) {
            config.update(|config| config.use_battleye = BattlEyeUsage::Always(false));
        }
    }

    let app = LauncherApp::new(
        root_logger.clone(),
        if log_level_override.is_none() { Some(log_level) } else { None },
        can_switch_branch,
        app,
        steam,
        game,
        config,
    );
    app.run(disable_prefetch);

    info!(root_logger, "Shutting down launcher");
    drop(log_guard);
}
