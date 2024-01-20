use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

use fltk::button::{Button, CheckButton, LightButton};
use fltk::enums::{Align, CallbackTrigger, Color, Event, Font, FrameType};
use fltk::frame::Frame;
use fltk::group::Group;
use fltk::input::Input;
use fltk::misc::InputChoice;
use fltk::prelude::*;
use fltk_float::button::ButtonElement;
use fltk_float::grid::Grid;
use fltk_float::{LayoutElement, LayoutWidgetWrapper};
use slog::{error, FilterLevel, Logger};

use crate::auth::AuthState;
use crate::config::{BattlEyeUsage, Config, LogLevel, ModMismatchChecks, ThemeChoice};
use crate::game::{Branch, Game, MapRef, Maps, ServerRef, Session};
use crate::workers::TaskState;

use super::assets::Assets;
use super::prelude::*;
use super::theme::Theme;
use super::widgets::ReadOnlyText;
use super::{alert_error, wrapper_factory, Handler};

pub enum HomeAction {
    Launch,
    Continue,
    SwitchBranch(Branch),
    ConfigureLogLevel(LogLevel),
    ConfigureBattlEye(BattlEyeUsage),
    ConfigureUseAllCores(bool),
    ConfigureExtraArgs(String),
    ConfigureModMismatchChecks(ModMismatchChecks),
    ConfigureTheme(ThemeChoice),
    RefreshAuthState,
}

pub enum HomeUpdate {
    LastSession,
    AuthState(AuthState),
}

pub struct Home {
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

impl Home {
    pub fn new(
        logger: Logger,
        game: Arc<Game>,
        config: &Config,
        log_level_overridden: bool,
        can_switch_branch: bool,
        on_action: impl Handler<HomeAction> + 'static,
    ) -> Rc<Self> {
        let on_action = Rc::new(on_action);

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
        let mut bugle_label = grid
            .span(1, 5)
            .unwrap()
            .wrap(Frame::default())
            .with_label("BUGLE");
        bugle_label.set_label_font(install_crom_font());
        bugle_label.set_label_size(bugle_label.label_size() * 3);

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
        grid.span(1, 3).unwrap().skip();
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
        let mut battleye_input = grid.cell().unwrap().wrap(InputChoice::default_fill());
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
        let mut log_level_input = grid.cell().unwrap().wrap(InputChoice::default_fill());
        grid.cell().unwrap().wrap(create_info_label("Theme:"));
        let mut theme_input = grid.span(1, 2).unwrap().wrap(InputChoice::default_fill());

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
        root.resize_callback(move |_, _, _, _, _| grid.layout_children());

        refresh_platform_button.deactivate();
        refresh_fls_button.deactivate();

        battleye_input.input().set_readonly(true);
        battleye_input.input().clear_visible_focus();
        battleye_input.add("Always");
        battleye_input.add("Never");
        battleye_input.add("Only when required");
        battleye_input.set_value_index(match config.use_battleye {
            BattlEyeUsage::Always(true) => 0,
            BattlEyeUsage::Always(false) => 1,
            BattlEyeUsage::Auto => 2,
        });
        battleye_input.set_trigger(CallbackTrigger::Changed);
        battleye_input.set_callback({
            let on_action = Rc::clone(&on_action);
            move |input| {
                let use_battleye = match input.menu_button().value() {
                    0 => BattlEyeUsage::Always(true),
                    1 => BattlEyeUsage::Always(false),
                    2 => BattlEyeUsage::Auto,
                    _ => unreachable!(),
                };
                on_action(HomeAction::ConfigureBattlEye(use_battleye)).unwrap();
            }
        });

        use_all_cores_button.set_checked(config.use_all_cores);
        use_all_cores_button.set_callback({
            let on_action = Rc::clone(&on_action);
            move |input| {
                on_action(HomeAction::ConfigureUseAllCores(input.is_checked())).unwrap();
            }
        });

        extra_args_input.set_value(&config.extra_args);
        let extra_args_dirty = Rc::new(Cell::new(false));
        extra_args_input.set_trigger(CallbackTrigger::Changed);
        extra_args_input.set_callback({
            let extra_args_dirty = Rc::clone(&extra_args_dirty);
            move |_| extra_args_dirty.set(true)
        });
        extra_args_input.handle({
            let on_action = Rc::clone(&on_action);
            move |input, event| {
                if let Event::Unfocus | Event::Hide = event {
                    if extra_args_dirty.take() {
                        on_action(HomeAction::ConfigureExtraArgs(input.value())).unwrap();
                    }
                }
                false
            }
        });

        mod_mismatch_check_button.set_checked(match config.mod_mismatch_checks {
            ModMismatchChecks::Enabled => true,
            ModMismatchChecks::Disabled => false,
        });
        mod_mismatch_check_button.set_callback({
            let on_action = Rc::clone(&on_action);
            move |input| {
                let checks = if input.is_checked() {
                    ModMismatchChecks::Enabled
                } else {
                    ModMismatchChecks::Disabled
                };
                on_action(HomeAction::ConfigureModMismatchChecks(checks)).unwrap();
            }
        });

        log_level_input.input().set_readonly(true);
        log_level_input.input().clear_visible_focus();
        log_level_input.add("Off");
        log_level_input.add("Trace");
        log_level_input.add("Debug");
        log_level_input.add("Info");
        log_level_input.add("Warning");
        log_level_input.add("Error");
        log_level_input.add("Critical");
        log_level_input.set_value_index(log_level_to_index(&config.log_level));
        log_level_input.set_callback({
            let on_action = Rc::clone(&on_action);
            move |input| {
                let log_level = index_to_log_level(input.menu_button().value());
                on_action(HomeAction::ConfigureLogLevel(log_level)).unwrap();
            }
        });
        log_level_input.set_activated(!log_level_overridden);

        theme_input.input().set_readonly(true);
        theme_input.input().clear_visible_focus();
        theme_input.add("Light");
        theme_input.add("Dark");
        theme_input.set_value_index(match config.theme {
            ThemeChoice::Light => 0,
            ThemeChoice::Dark => 1,
        });
        theme_input.set_callback({
            let on_action = Rc::clone(&on_action);
            move |input| {
                let theme = match input.menu_button().value() {
                    0 => ThemeChoice::Light,
                    1 => ThemeChoice::Dark,
                    _ => unreachable!(),
                };
                Theme::from_config(theme).apply();
                on_action(HomeAction::ConfigureTheme(theme)).unwrap();
            }
        });

        privacy_switch.clear_visible_focus();

        refresh_platform_button.set_callback({
            let on_action = Rc::clone(&on_action);
            move |_| on_action(HomeAction::RefreshAuthState).unwrap()
        });
        refresh_fls_button.set_callback({
            let on_action = Rc::clone(&on_action);
            move |_| on_action(HomeAction::RefreshAuthState).unwrap()
        });

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
            let on_action = Rc::clone(&on_action);
            let logger = logger.clone();
            move |_| {
                if let Err(err) = on_action(HomeAction::Launch) {
                    error!(logger, "Error launching game"; "error" => %err);
                    alert_error(ERR_LAUNCHING_GAME, &err);
                }
            }
        });
        continue_button.set_callback({
            let on_action = Rc::clone(&on_action);
            let logger = logger.clone();
            move |_| {
                if let Err(err) = on_action(HomeAction::Continue) {
                    error!(logger, "Error launching game"; "error" => %err);
                    alert_error(ERR_LAUNCHING_GAME, &err);
                }
            }
        });
        if let Some(mut button) = switch_branch_button {
            button.set_callback({
                let on_action = Rc::clone(&on_action);
                let logger = logger.clone();
                let branch = game.branch();
                move |_| {
                    if let Err(err) = on_action(HomeAction::SwitchBranch(other_branch)) {
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
            });
        }

        let _ = launch_button.take_focus();

        Rc::new(Self {
            root,
            game,
            platform_user_id_text,
            platform_user_name_text,
            refresh_platform_button,
            fls_acct_id_text,
            fls_acct_name_text,
            refresh_fls_button,
            online_play_text,
            sp_play_text,
            last_session_text,
        })
    }

    pub fn root(&self) -> &impl WidgetExt {
        &self.root
    }

    pub fn handle_update(&self, update: HomeUpdate) {
        match update {
            HomeUpdate::LastSession => self
                .last_session_text
                .set_value(last_session_text(&self.game)),
            HomeUpdate::AuthState(state) => self.update_auth_state(state),
        }
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

fn install_crom_font() -> Font {
    Assets::crom_font().unwrap_or(Font::TimesBold)
}

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
