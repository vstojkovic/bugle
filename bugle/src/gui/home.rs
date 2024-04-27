use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

use dynabus::Bus;
use fltk::button::{Button, CheckButton, LightButton};
use fltk::enums::{Align, CallbackTrigger, Color, Event, FrameType};
use fltk::frame::Frame;
use fltk::group::Group;
use fltk::input::Input;
use fltk::prelude::*;
use fltk_float::button::ButtonElement;
use fltk_float::grid::Grid;
use fltk_float::{LayoutElement, LayoutWidgetWrapper};
use slog::{error, FilterLevel, Logger};

use crate::auth::AuthState;
use crate::auth_manager::AuthManager;
use crate::bus::AppBus;
use crate::config::{BattlEyeUsage, ConfigManager, LogLevel, ModMismatchChecks, ThemeChoice};
use crate::env;
use crate::game::{Branch, Game, MapRef, Maps, ServerRef, Session};
use crate::launcher::Launcher;
use crate::util::weak_cb;
use crate::workers::TaskState;

use super::assets::Assets;
use super::prelude::*;
use super::theme::Theme;
use super::widgets::{DropDownList, ReadOnlyText};
use super::{alert_error, wrapper_factory};

#[derive(dynabus::Event)]
pub struct UpdateLastSession;

#[derive(dynabus::Event)]
pub struct UpdateAuthState(pub AuthState);

pub struct HomeTab {
    grid: Grid,
    root: Group,
    game: Arc<Game>,
    platform_user_id_text: ReadOnlyText,
    platform_user_name_text: ReadOnlyText,
    refresh_platform_button: Button,
    fls_acct_id_text: ReadOnlyText,
    fls_acct_name_text: ReadOnlyText,
    refresh_fls_button: Button,
    online_play_text: ReadOnlyText,
    sp_play_text: ReadOnlyText,
    last_session_text: ReadOnlyText,
}

impl HomeTab {
    pub fn new(
        logger: &Logger,
        bus: Rc<RefCell<AppBus>>,
        game: Arc<Game>,
        config: Rc<ConfigManager>,
        log_level: Option<Arc<AtomicUsize>>,
        auth: Rc<AuthManager>,
        launcher: Rc<Launcher>,
        can_switch_branch: bool,
    ) -> Rc<Self> {
        let (branch_name, other_branch_name, other_branch) = match game.branch() {
            Branch::Main => ("Live", "TestLive", Branch::PublicBeta),
            Branch::PublicBeta => ("TestLive", "Live", Branch::Main),
        };

        let mut grid = Grid::builder_with_factory(wrapper_factory())
            .with_col_spacing(10)
            .with_row_spacing(10);

        grid.col().add();
        grid.col().with_stretch(1).add();
        grid.col().add();
        grid.col().with_stretch(1).add();
        grid.col().add();

        grid.row().add();
        grid.span(1, 5)
            .unwrap()
            .wrap(Frame::default())
            .with_label("Welcome to");

        grid.row().add();
        let mut bugle_label = grid.span(1, 5).unwrap().wrap(Frame::default());
        bugle_label.set_image(Some(Assets::bugle_logo()));

        grid.row().add();
        grid.span(1, 5)
            .unwrap()
            .wrap(Frame::default())
            .with_label("Butt-Ugly Game Launcher for Exiles");

        grid.row().add();
        grid.cell()
            .unwrap()
            .wrap(create_info_label("BUGLE Version:"));
        grid.span(1, 4)
            .unwrap()
            .wrap(ReadOnlyText::new(env!("CARGO_PKG_VERSION").to_string()));

        grid.row().add();
        grid.cell()
            .unwrap()
            .wrap(create_info_label("Conan Exiles Installation Path:"));
        grid.span(1, 4).unwrap().wrap(ReadOnlyText::new(
            game.installation_path().to_string_lossy().into_owned(),
        ));

        grid.row().add();
        grid.cell()
            .unwrap()
            .wrap(create_info_label("Conan Exiles Revision:"));
        grid.cell().unwrap().wrap(ReadOnlyText::new({
            let (revision, snapshot) = game.version();
            format!("#{}/{} ({})", revision, snapshot, branch_name)
        }));
        grid.cell()
            .unwrap()
            .wrap(create_info_label("Conan Exiles Build ID:"));
        grid.span(1, 2)
            .unwrap()
            .wrap(ReadOnlyText::new(format!("{}", game.build_id())));

        grid.row().add();
        grid.cell()
            .unwrap()
            .wrap(create_info_label("Steam Account ID:"));
        let platform_user_id_text = grid.cell().unwrap().wrap(ReadOnlyText::default());
        grid.cell()
            .unwrap()
            .wrap(create_info_label("Steam Account Name:"));
        let platform_user_name_text = grid.cell().unwrap().wrap(ReadOnlyText::default());
        let mut refresh_platform_button = grid
            .cell()
            .unwrap()
            .wrap(Button::default())
            .with_label("Refresh");

        grid.row().add();
        grid.cell()
            .unwrap()
            .wrap(create_info_label("FLS Account ID:"));
        let fls_acct_id_text = grid.cell().unwrap().wrap(ReadOnlyText::default());
        grid.cell()
            .unwrap()
            .wrap(create_info_label("FLS Account Name:"));
        let fls_acct_name_text = grid.cell().unwrap().wrap(ReadOnlyText::default());
        let mut refresh_fls_button = grid
            .cell()
            .unwrap()
            .wrap(Button::default())
            .with_label("Refresh");

        grid.row().add();
        grid.cell()
            .unwrap()
            .wrap(create_info_label("Can Play Online?"));
        let online_play_text = grid.cell().unwrap().wrap(ReadOnlyText::default());
        grid.cell()
            .unwrap()
            .wrap(create_info_label("Can Play Singleplayer?"));
        let sp_play_text = grid.span(1, 2).unwrap().wrap(ReadOnlyText::default());

        grid.row().add();
        grid.cell()
            .unwrap()
            .wrap(create_info_label("BattlEye Installed?"));
        let battleye_installed = match game.battleye_installed() {
            Some(true) => "Yes",
            Some(false) => "No",
            None => "Unable to determine",
        };
        grid.cell()
            .unwrap()
            .wrap(ReadOnlyText::new(battleye_installed.to_string()));
        grid.cell().unwrap().skip();
        let mut privacy_switch = grid
            .span(1, 2)
            .unwrap()
            .wrap(LightButton::default())
            .with_label("Hide Private Information");

        grid.row().add();
        grid.span(1, 5)
            .unwrap()
            .wrap(Frame::default())
            .set_frame(FrameType::ThinDownFrame);

        grid.row().add();
        grid.cell()
            .unwrap()
            .wrap(create_info_label("Enable BattlEye:"));
        let mut battleye_input = grid.cell().unwrap().wrap(DropDownList::default_fill());
        battleye_input.add("Always");
        battleye_input.add("Never");
        battleye_input.add("Only when required");
        grid.cell()
            .unwrap()
            .wrap(create_info_label("Use all CPU cores:"));
        let mut use_all_cores_button = grid.span(1, 2).unwrap().wrap(CheckButton::default());
        use_all_cores_button.clear_visible_focus();

        grid.row().add();
        grid.cell()
            .unwrap()
            .wrap(create_info_label("Additional Launch Options:"));
        let mut extra_args_input = grid.span(1, 4).unwrap().wrap(Input::default());

        grid.row().add();
        grid.span(1, 5)
            .unwrap()
            .wrap(Frame::default())
            .set_frame(FrameType::ThinDownFrame);

        grid.row().add();
        grid.cell()
            .unwrap()
            .wrap(create_info_label("BUGLE Logging Level:"));
        let mut log_level_input = grid.cell().unwrap().wrap(DropDownList::default_fill());
        log_level_input.add("Off");
        log_level_input.add("Trace");
        log_level_input.add("Debug");
        log_level_input.add("Info");
        log_level_input.add("Warning");
        log_level_input.add("Error");
        log_level_input.add("Critical");
        grid.cell().unwrap().wrap(create_info_label("Theme:"));
        let mut theme_input = grid.span(1, 2).unwrap().wrap(DropDownList::default_fill());
        theme_input.add("Light");
        theme_input.add("Dark");

        grid.row().add();
        grid.cell()
            .unwrap()
            .wrap(create_info_label("Mod Mismatch Detection:"));
        let mut mod_mismatch_check_button = grid.cell().unwrap().wrap(CheckButton::default());
        mod_mismatch_check_button.clear_visible_focus();
        grid.span(1, 3).unwrap().skip();

        grid.row().with_stretch(1).add();
        grid.span(1, 5).unwrap().skip();

        let mut action_grid = Grid::builder().with_col_spacing(10).with_row_spacing(10);
        action_grid.col().with_stretch(1).batch(4);

        let mut last_session_grid = Grid::builder_with_factory(wrapper_factory())
            .with_col_spacing(10)
            .with_row_spacing(10);
        last_session_grid.col().add();
        last_session_grid.col().with_stretch(1).add();
        last_session_grid.row().add();
        last_session_grid
            .cell()
            .unwrap()
            .wrap(create_info_label("Last Session:"));
        let last_session_text = last_session_grid
            .cell()
            .unwrap()
            .wrap(ReadOnlyText::new(last_session_text(&*game)));
        let last_session_grid = last_session_grid.end();

        action_grid.row().add();
        action_grid.span(1, 2).unwrap().skip();
        action_grid.span(1, 2).unwrap().add(last_session_grid);

        action_grid.row().with_stretch(1).add();
        let cell = action_grid.cell().unwrap();
        let switch_branch_button = if can_switch_branch {
            let switch_label = format!("Switch to {}", other_branch_name);
            let button = Button::default().with_label(&switch_label);
            action_grid
                .cell()
                .unwrap()
                .add(BigButtonElement::wrap(button.clone()));
            Some(button)
        } else {
            cell.skip();
            None
        };
        action_grid.cell().unwrap().skip();
        let mut launch_button = Button::default().with_label("Launch");
        action_grid
            .cell()
            .unwrap()
            .add(BigButtonElement::wrap(launch_button.clone()));
        let mut continue_button = Button::default().with_label("Continue");
        action_grid
            .cell()
            .unwrap()
            .add(BigButtonElement::wrap(continue_button.clone()));
        let action_grid = action_grid.end();

        grid.row().add();
        grid.span(1, 5).unwrap().add(action_grid);

        let grid = grid.end();
        grid.layout_children();

        let mut root = grid.group();
        root.hide();

        refresh_platform_button.set_callback({
            let auth = Rc::clone(&auth);
            move |_| auth.check_auth_state()
        });
        refresh_fls_button.set_callback({
            let auth = Rc::clone(&auth);
            move |_| auth.check_auth_state()
        });
        refresh_platform_button.deactivate();
        refresh_fls_button.deactivate();

        battleye_input.set_value(match config.get().use_battleye {
            BattlEyeUsage::Always(true) => 0,
            BattlEyeUsage::Always(false) => 1,
            BattlEyeUsage::Auto => 2,
        });
        battleye_input.set_callback({
            let config = Rc::clone(&config);
            move |input| {
                let use_battleye = match input.value() {
                    0 => BattlEyeUsage::Always(true),
                    1 => BattlEyeUsage::Always(false),
                    2 => BattlEyeUsage::Auto,
                    _ => unreachable!(),
                };
                config.update(|config| config.use_battleye = use_battleye);
            }
        });

        use_all_cores_button.set_checked(config.get().use_all_cores);
        use_all_cores_button.set_callback({
            let config = Rc::clone(&config);
            move |input| {
                config.update(|config| config.use_all_cores = input.is_checked());
            }
        });

        extra_args_input.set_value(&config.get().extra_args);
        let extra_args_dirty = Rc::new(Cell::new(false));
        extra_args_input.set_trigger(CallbackTrigger::Changed);
        extra_args_input.set_callback({
            let extra_args_dirty = Rc::clone(&extra_args_dirty);
            move |_| extra_args_dirty.set(true)
        });
        extra_args_input.handle({
            let config = Rc::clone(&config);
            move |input, event| {
                if let Event::Unfocus | Event::Hide = event {
                    if extra_args_dirty.take() {
                        config.update(|config| config.extra_args = input.value());
                    }
                }
                false
            }
        });

        mod_mismatch_check_button.set_checked(match config.get().mod_mismatch_checks {
            ModMismatchChecks::Enabled => true,
            ModMismatchChecks::Disabled => false,
        });
        mod_mismatch_check_button.set_callback({
            let config = Rc::clone(&config);
            move |input| {
                let checks = if input.is_checked() {
                    ModMismatchChecks::Enabled
                } else {
                    ModMismatchChecks::Disabled
                };
                config.update(|config| config.mod_mismatch_checks = checks);
            }
        });

        log_level_input.set_value(log_level_to_index(&config.get().log_level));
        log_level_input.set_activated(log_level.is_some());
        log_level_input.set_callback({
            let config = Rc::clone(&config);
            move |input| {
                let new_log_level = index_to_log_level(input.value());
                config.update(|config| config.log_level = new_log_level);
                if let Some(log_level) = log_level.as_ref() {
                    log_level.store(
                        new_log_level.0.as_usize(),
                        std::sync::atomic::Ordering::Relaxed,
                    );
                }
            }
        });

        theme_input.set_value(match config.get().theme {
            ThemeChoice::Light => 0,
            ThemeChoice::Dark => 1,
        });
        theme_input.set_callback({
            let config = Rc::clone(&config);
            move |input| {
                let theme = match input.value() {
                    0 => ThemeChoice::Light,
                    1 => ThemeChoice::Dark,
                    _ => unreachable!(),
                };
                Theme::from_config(theme).apply();
                config.update(|config| config.theme = theme);
            }
        });

        privacy_switch.clear_visible_focus();

        privacy_switch.set_callback({
            let mut platform_user_id_text = platform_user_id_text.clone();
            let mut platform_user_name_text = platform_user_name_text.clone();
            let mut fls_acct_id_text = fls_acct_id_text.clone();
            let mut fls_acct_name_text = fls_acct_name_text.clone();
            let mut last_session_text = match &*game.last_session() {
                Some(Session::Online(_)) => Some(last_session_text.clone()),
                _ => None,
            };
            move |btn| {
                let color = if btn.value() { Color::Light2 } else { Color::Foreground };
                platform_user_id_text.set_text_color(color);
                platform_user_name_text.set_text_color(color);
                fls_acct_id_text.set_text_color(color);
                fls_acct_name_text.set_text_color(color);
                platform_user_id_text.redraw();
                platform_user_name_text.redraw();
                fls_acct_id_text.redraw();
                fls_acct_name_text.redraw();
                if let Some(last_session_text) = last_session_text.as_mut() {
                    last_session_text.set_text_color(color);
                    last_session_text.redraw();
                }
            }
        });

        launch_button.set_callback({
            let launcher = Rc::clone(&launcher);
            let logger = logger.clone();
            move |_| {
                if let Err(err) = launcher.launch_game() {
                    error!(logger, "Error launching game"; "error" => %err);
                    alert_error(ERR_LAUNCHING_GAME, &err);
                }
            }
        });
        continue_button.set_callback({
            let launcher = Rc::clone(&launcher);
            let logger = logger.clone();
            move |_| {
                if let Err(err) = launcher.continue_last_session() {
                    error!(logger, "Error launching game"; "error" => %err);
                    alert_error(ERR_LAUNCHING_GAME, &err);
                }
            }
        });
        if let Some(mut button) = switch_branch_button {
            button.set_callback({
                let config = Rc::clone(&config);
                let logger = logger.clone();
                let branch = game.branch();
                move |_| {
                    let result = config
                        .try_update(|config| config.branch = branch)
                        .and_then(|_| Ok(env::restart_process()?));
                    match result {
                        Ok(_) => fltk::app::quit(),
                        Err(err) => {
                            error!(
                                logger,
                                "Error switching to other branch";
                                "branch" => ?other_branch,
                                "error" => %err,
                            );
                            let err_msg = match branch {
                                Branch::Main => ERR_SWITCHING_TO_MAIN,
                                Branch::PublicBeta => ERR_SWITCHING_TO_PUBLIC_BETA,
                            };
                            alert_error(err_msg, &err);
                        }
                    }
                }
            });
        }

        let _ = launch_button.take_focus();

        let this = Rc::new(Self {
            grid,
            root,
            game,
            platform_user_id_text,
            platform_user_name_text,
            refresh_platform_button: refresh_platform_button.clone(),
            fls_acct_id_text,
            fls_acct_name_text,
            refresh_fls_button: refresh_fls_button.clone(),
            online_play_text,
            sp_play_text,
            last_session_text,
        });

        {
            let mut bus = bus.borrow_mut();
            bus.subscribe_consumer(weak_cb!(
                [this] => |UpdateLastSession| this.update_last_session()
            ));
            bus.subscribe_consumer(weak_cb!(
                [this] => |UpdateAuthState(state)| this.update_auth_state(state)));
        }

        this
    }

    pub fn root(&self) -> &impl WidgetExt {
        &self.root
    }

    fn update_last_session(&self) {
        self.last_session_text
            .set_value(last_session_text(&self.game));
    }

    fn update_auth_state(&self, state: AuthState) {
        let (id, name, can_refresh) = match state.platform_user {
            Ok(user) => (user.id, user.display_name, false),
            Err(err) => {
                let err_str = format!("<{}>", err);
                (err_str.clone(), err_str, true)
            }
        };
        self.platform_user_id_text.set_value(id);
        self.platform_user_name_text.set_value(name);
        self.refresh_platform_button
            .clone()
            .set_activated(can_refresh);

        let (id, name, can_refresh) = match state.fls_account {
            TaskState::Pending => (
                "<Fetching...>".to_string(),
                "<Fetching...>".to_string(),
                false,
            ),
            TaskState::Ready(Ok(acct)) => (acct.master_id, acct.display_name, false),
            TaskState::Ready(Err(err)) => {
                let err_str = format!("<{}>", err);
                (err_str.clone(), err_str, true)
            }
        };
        self.fls_acct_id_text.set_value(id);
        self.fls_acct_name_text.set_value(name);
        self.refresh_fls_button.clone().set_activated(can_refresh);

        let online_play_str = match state.online_capability {
            TaskState::Pending => "<Checking...>".to_string(),
            TaskState::Ready(Ok(())) => "Yes".to_string(),
            TaskState::Ready(Err(err)) => format!("No, {}", err),
        };
        self.online_play_text.set_value(online_play_str);

        let sp_play_str = match state.sp_capability {
            TaskState::Pending => "<Checking...>".to_string(),
            TaskState::Ready(Ok(())) => "Yes".to_string(),
            TaskState::Ready(Err(err)) => format!("No, {}", err),
        };
        self.sp_play_text.set_value(sp_play_str);
    }
}

impl LayoutElement for HomeTab {
    fn min_size(&self) -> fltk_float::Size {
        self.grid.min_size()
    }

    fn layout(&self, x: i32, y: i32, width: i32, height: i32) {
        self.grid.layout(x, y, width, height)
    }
}

struct BigButtonElement {
    inner: ButtonElement<Button>,
}

impl LayoutWidgetWrapper<Button> for BigButtonElement {
    fn wrap(widget: Button) -> Self {
        Self {
            inner: ButtonElement::wrap(widget),
        }
    }
}

impl LayoutElement for BigButtonElement {
    fn min_size(&self) -> fltk_float::Size {
        let mut size = self.inner.min_size();
        size.height *= 2;
        size
    }

    fn layout(&self, x: i32, y: i32, width: i32, height: i32) {
        self.inner.layout(x, y, width, height);
    }
}

const ERR_LAUNCHING_GAME: &str = "Error while trying to launch the game.";
const ERR_SWITCHING_TO_MAIN: &str = "Error while trying to switch to Live.";
const ERR_SWITCHING_TO_PUBLIC_BETA: &str = "Error while trying to switch to TestLive.";

fn create_info_label(text: &str) -> Frame {
    Frame::default()
        .with_align(Align::Right | Align::Inside)
        .with_label(text)
}

fn last_session_text(game: &Game) -> String {
    match &*game.last_session() {
        None => "<none>".to_string(),
        Some(Session::SinglePlayer(map_ref)) => {
            format!("Singleplayer: {}", map_ref_text(game.maps(), map_ref))
        }
        Some(Session::CoOp(map_ref)) => format!("Co-op: {}", map_ref_text(game.maps(), map_ref)),
        Some(Session::Online(server_ref)) => format!("Online: {}", server_ref_text(server_ref)),
    }
}

fn map_ref_text(maps: &Maps, map_ref: &MapRef) -> String {
    match map_ref {
        MapRef::Known { map_id } => maps[*map_id].display_name.clone(),
        MapRef::Unknown { asset_path } => format!("<unknown map: {}>", asset_path),
    }
}

fn server_ref_text(server_ref: &ServerRef) -> String {
    match server_ref {
        ServerRef::Known(server) => server.name.clone(),
        ServerRef::Unknown(addr) => addr.to_string(),
    }
}

fn log_level_to_index(log_level: &LogLevel) -> i32 {
    match log_level.0 {
        FilterLevel::Off => 0,
        FilterLevel::Trace => 1,
        FilterLevel::Debug => 2,
        FilterLevel::Info => 3,
        FilterLevel::Warning => 4,
        FilterLevel::Error => 5,
        FilterLevel::Critical => 6,
    }
}

fn index_to_log_level(idx: i32) -> LogLevel {
    LogLevel(match idx {
        0 => FilterLevel::Off,
        1 => FilterLevel::Trace,
        2 => FilterLevel::Debug,
        3 => FilterLevel::Info,
        4 => FilterLevel::Warning,
        5 => FilterLevel::Error,
        6 => FilterLevel::Critical,
        _ => unreachable!(),
    })
}
