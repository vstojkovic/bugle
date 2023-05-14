use std::fs::File;
use std::io::Write;
use std::rc::Rc;

use fltk::button::Button;
use fltk::enums::{Align, CallbackTrigger, Font};
use fltk::frame::Frame;
use fltk::group::Group;
use fltk::misc::InputChoice;
use fltk::prelude::*;
use slog::{error, FilterLevel, Logger};
use tempfile::tempdir;

use crate::auth::AuthState;
use crate::config::{BattlEyeUsage, Config, LogLevel};
use crate::game::Game;
use crate::workers::TaskState;

use super::{alert_error, widget_auto_height, widget_col_width, CleanupFn, Handler};
use super::{button_auto_height, prelude::*, ReadOnlyText};

pub enum HomeAction {
    Launch,
    Continue,
    ConfigureLogLevel(LogLevel),
    ConfigureBattlEye(BattlEyeUsage),
    RefreshAuthState,
}

pub enum HomeUpdate {
    AuthState(AuthState),
}

pub struct Home {
    root: Group,
    platform_user_id_text: ReadOnlyText,
    platform_user_name_text: ReadOnlyText,
    refresh_platform_button: Button,
    fls_acct_id_text: ReadOnlyText,
    fls_acct_name_text: ReadOnlyText,
    refresh_fls_button: Button,
    online_play_text: ReadOnlyText,
    sp_play_text: ReadOnlyText,
}

impl Home {
    pub fn new(
        logger: Logger,
        game: &Game,
        config: &Config,
        log_level_overridden: bool,
        on_action: impl Handler<HomeAction> + 'static,
    ) -> Rc<Self> {
        let on_action = Rc::new(on_action);

        let mut root = Group::default_fill();

        let top_welcome_line = Frame::default_fill().with_label("Welcome to");
        let top_welcome_height = widget_auto_height(&top_welcome_line);
        let top_welcome_line = top_welcome_line
            .with_size_flex(0, top_welcome_height)
            .inside_parent(0, 0);

        let mut mid_welcome_line = Frame::default_fill().with_label("BUGLE");
        mid_welcome_line.set_label_font(install_crom_font());
        mid_welcome_line.set_label_size(mid_welcome_line.label_size() * 3);
        let mid_welcome_height = widget_auto_height(&mid_welcome_line);
        let mid_welcome_line = mid_welcome_line
            .with_size_flex(0, mid_welcome_height)
            .below_of(&top_welcome_line, 0);

        let btm_welcome_line =
            Frame::default_fill().with_label("Butt-Ugly Game Launcher for Exiles");
        let btm_welcome_height = widget_auto_height(&btm_welcome_line);
        let btm_welcome_line = btm_welcome_line
            .with_size_flex(0, btm_welcome_height)
            .below_of(&mid_welcome_line, 0);

        let info_pane = Group::default_fill();
        let version_label = create_info_label("BUGLE Version:");
        let version_text = ReadOnlyText::new(env!("CARGO_PKG_VERSION").to_string());
        let game_path_label = create_info_label("Conan Exiles Installation Path:");
        let game_path_text =
            ReadOnlyText::new(game.installation_path().to_string_lossy().into_owned());
        let build_id_label = create_info_label("Conan Exiles Build ID:");
        let build_id_text = ReadOnlyText::new(format!("{}", game.build_id()));
        let revision_label = create_info_label("Conan Exiles Revision:");
        let revision_text = ReadOnlyText::new({
            let (maj, min) = game.revision();
            format!("{}/{}", maj, min)
        });
        let platform_user_id_label = create_info_label("Steam Account ID:");
        let platform_user_id_text = ReadOnlyText::default();
        let platform_user_name_label = create_info_label("Steam Account Name:");
        let platform_user_name_text = ReadOnlyText::default();
        let refresh_platform_button = Button::default().with_label("Refresh");
        let fls_acct_id_label = create_info_label("FLS Account ID:");
        let fls_acct_id_text = ReadOnlyText::default();
        let fls_acct_name_label = create_info_label("FLS Account Name:");
        let fls_acct_name_text = ReadOnlyText::default();
        let refresh_fls_button = Button::default().with_label("Refresh");
        let online_play_label = create_info_label("Can Play Online?");
        let online_play_text = ReadOnlyText::default();
        let sp_play_label = create_info_label("Can Play Singleplayer?");
        let sp_play_text = ReadOnlyText::default();
        let log_level_label = create_info_label("BUGLE Logging Level:");
        let log_level_input = InputChoice::default_fill();
        let battleye_label = create_info_label("Use BattlEye:");
        let battleye_input = InputChoice::default_fill();
        info_pane.end();

        let left_width = widget_col_width(&[
            &version_label,
            &game_path_label,
            &build_id_label,
            &platform_user_id_label,
            &fls_acct_id_label,
        ]);
        let right_width = widget_col_width(&[
            &revision_label,
            &platform_user_name_label,
            &fls_acct_name_label,
        ]);
        let button_width = widget_col_width(&[&refresh_platform_button, &refresh_fls_button]);
        let button_height = button_auto_height(&refresh_platform_button);

        let launch_button = Button::default().with_label("Launch");
        let continue_button = Button::default().with_label("Continue");
        let action_width = root.w() / 4 - 5;
        let action_height = 2 * button_height;

        let mut continue_button = continue_button
            .with_size(action_width, action_height)
            .inside_parent(-action_width, -action_height);
        let mut launch_button = launch_button
            .with_size(action_width, action_height)
            .left_of(&continue_button, 10);

        let info_pane = info_pane.below_of(&btm_welcome_line, 10);
        let info_height = launch_button.y() - info_pane.y() - 10;
        let info_pane = info_pane.with_size_flex(0, info_height);
        let text_width = info_pane.w() - left_width - 10;
        let narrow_width = (info_pane.w() - left_width - right_width - 30) / 2;
        let text_height = widget_auto_height(&version_label);
        let version_label = version_label
            .with_size(left_width, text_height)
            .inside_parent(0, 0);
        let _ = version_text
            .widget()
            .clone()
            .with_size(text_width, text_height)
            .right_of(&version_label, 10);
        let game_path_label = game_path_label
            .with_size(left_width, text_height)
            .below_of(&version_label, 10);
        let _ = game_path_text
            .widget()
            .clone()
            .with_size(text_width, text_height)
            .right_of(&game_path_label, 10);
        let build_id_label = build_id_label
            .with_size(left_width, text_height)
            .below_of(&game_path_label, 10);
        let _ = build_id_text
            .widget()
            .clone()
            .with_size(narrow_width, text_height)
            .right_of(&build_id_label, 10);
        let revision_label = revision_label
            .with_size(right_width, text_height)
            .right_of(build_id_text.widget(), 10);
        revision_text
            .widget()
            .clone()
            .with_size(narrow_width, text_height)
            .right_of(&revision_label, 10);
        let platform_user_id_label = platform_user_id_label
            .with_size(left_width, text_height)
            .below_of(&build_id_label, 10);
        let _ = platform_user_id_text
            .widget()
            .clone()
            .with_size(narrow_width, text_height)
            .right_of(&platform_user_id_label, 10);
        let platform_user_name_label = platform_user_name_label
            .with_size(right_width, text_height)
            .right_of(platform_user_id_text.widget(), 10);
        let _ = platform_user_name_text
            .widget()
            .clone()
            .with_size(narrow_width - button_width - 10, text_height)
            .right_of(&platform_user_name_label, 10);
        let mut refresh_platform_button = refresh_platform_button
            .with_size(button_width, button_height)
            .right_of(platform_user_name_text.widget(), 10);
        refresh_platform_button.deactivate();
        let fls_acct_id_label = fls_acct_id_label
            .with_size(left_width, text_height)
            .below_of(&platform_user_id_label, 10);
        let _ = fls_acct_id_text
            .widget()
            .clone()
            .with_size(narrow_width, text_height)
            .right_of(&fls_acct_id_label, 10);
        let fls_acct_name_label = fls_acct_name_label
            .with_size(right_width, text_height)
            .right_of(fls_acct_id_text.widget(), 10);
        let _ = fls_acct_name_text
            .widget()
            .clone()
            .with_size(narrow_width - button_width - 10, text_height)
            .right_of(&fls_acct_name_label, 10);
        let mut refresh_fls_button = refresh_fls_button
            .with_size(button_width, button_height)
            .right_of(fls_acct_name_text.widget(), 10);
        refresh_fls_button.deactivate();
        let online_play_label = online_play_label
            .with_size(left_width, text_height)
            .below_of(&fls_acct_id_label, 10);
        let _ = online_play_text
            .widget()
            .clone()
            .with_size(narrow_width, text_height)
            .right_of(&online_play_label, 10);
        let sp_play_label = sp_play_label
            .with_size(right_width, text_height)
            .right_of(online_play_text.widget(), 10);
        let _ = sp_play_text
            .widget()
            .clone()
            .with_size(narrow_width, text_height)
            .right_of(&sp_play_label, 10);

        let log_level_label = log_level_label
            .with_size(left_width, text_height)
            .below_of(&online_play_label, 10);
        let mut log_level_input = log_level_input
            .with_size(narrow_width, text_height)
            .right_of(&log_level_label, 10);
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
                let log_level = index_to_log_leve(input.menu_button().value());
                on_action(HomeAction::ConfigureLogLevel(log_level)).unwrap();
            }
        });
        log_level_input.set_activated(!log_level_overridden);

        let battleye_label = battleye_label
            .with_size(right_width, text_height)
            .right_of(&log_level_input, 10);
        let mut battleye_input = battleye_input
            .with_size(narrow_width, text_height)
            .right_of(&battleye_label, 10);
        battleye_input.input().set_readonly(true);
        battleye_input.input().clear_visible_focus();
        battleye_input.add("Enabled");
        battleye_input.add("Disabled");
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

        refresh_platform_button.set_callback({
            let on_action = Rc::clone(&on_action);
            move |_| on_action(HomeAction::RefreshAuthState).unwrap()
        });
        refresh_fls_button.set_callback({
            let on_action = Rc::clone(&on_action);
            move |_| on_action(HomeAction::RefreshAuthState).unwrap()
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

        root.end();
        root.hide();

        let _ = launch_button.take_focus();

        Rc::new(Self {
            root,
            platform_user_id_text,
            platform_user_name_text,
            refresh_platform_button,
            fls_acct_id_text,
            fls_acct_name_text,
            refresh_fls_button,
            online_play_text,
            sp_play_text,
        })
    }

    pub fn show(&self) -> CleanupFn {
        let mut root = self.root.clone();
        root.show();

        Box::new(move || {
            root.hide();
        })
    }

    pub fn handle_update(&self, update: HomeUpdate) {
        match update {
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

const ERR_LAUNCHING_GAME: &str = "Error while trying to launch the game.";

fn install_crom_font() -> Font {
    try_install_crom_font().unwrap_or(Font::TimesBold)
}

fn try_install_crom_font() -> anyhow::Result<Font> {
    let dir = tempdir()?;
    let path = dir.path().join("Crom_v1.ttf");

    let mut file = File::create(&path)?;
    file.write_all(include_bytes!("Crom_v1.ttf"))?;
    drop(file);

    let font = Font::load_font(path)?;
    Font::set_font(Font::Zapfdingbats, &font);
    Ok(Font::Zapfdingbats)
}

fn create_info_label(text: &str) -> Frame {
    Frame::default()
        .with_align(Align::Right | Align::Inside)
        .with_label(text)
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

fn index_to_log_leve(idx: i32) -> LogLevel {
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
