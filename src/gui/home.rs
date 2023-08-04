use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::rc::Rc;
use std::sync::Arc;

use fltk::button::{Button, LightButton};
use fltk::enums::{Align, CallbackTrigger, Color, Font};
use fltk::frame::Frame;
use fltk::group::Group;
use fltk::misc::InputChoice;
use fltk::prelude::*;
use fltk_float::button::ButtonElement;
use fltk_float::grid::Grid;
use fltk_float::{LayoutElement, LayoutWidgetWrapper};
use slog::{error, FilterLevel, Logger};
use tempfile::tempdir;
use unic_langid::LanguageIdentifier;

use crate::auth::AuthState;
use crate::config::{BattlEyeUsage, Config, LogLevel, ThemeChoice};
use crate::game::{Branch, Game, MapRef, Maps, ServerRef, Session};
use crate::l10n::{localization, use_l10n, Localizer};
use crate::workers::TaskState;

use super::prelude::*;
use super::theme::Theme;
use super::widgets::ReadOnlyText;
use super::{alert_error, wrapper_factory, CleanupFn, Handler};

pub enum HomeAction {
    Launch,
    Continue,
    SwitchBranch(Branch),
    ConfigureLogLevel(LogLevel),
    ConfigureBattlEye(BattlEyeUsage),
    ConfigureLocale(Option<LanguageIdentifier>),
    ConfigureTheme(ThemeChoice),
    RefreshAuthState,
}

pub enum HomeUpdate {
    LastSession,
    AuthState(AuthState),
}

pub struct Home {
    root: Group,
    localizer: Rc<Localizer>,
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
        available_locales: HashMap<LanguageIdentifier, String>,
        default_locale: LanguageIdentifier,
        log_level_overridden: bool,
        can_switch_branch: bool,
        on_action: impl Handler<HomeAction> + 'static,
    ) -> Rc<Self> {
        let on_action = Rc::new(on_action);

        let localizer = localization().localizer("home");
        use_l10n!(localizer);

        let (branch_name, other_branch_name, other_branch) = match game.branch() {
            Branch::Main => (
                l10n!(branch_live),
                l10n!(branch_testlive),
                Branch::PublicBeta,
            ),
            Branch::PublicBeta => (l10n!(branch_testlive), l10n!(branch_live), Branch::Main),
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
            .with_label(l10n!(&top_welcome));

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
            .with_label(l10n!(&bottom_welcome));

        grid.row().add();
        grid.cell()
            .unwrap()
            .wrap(create_info_label(l10n!(&bugle_version)));
        grid.span(1, 4)
            .unwrap()
            .wrap(ReadOnlyText::new(env!("CARGO_PKG_VERSION").to_string()));

        grid.row().add();
        grid.cell()
            .unwrap()
            .wrap(create_info_label(l10n!(&game_path)));
        grid.span(1, 4).unwrap().wrap(ReadOnlyText::new(
            game.installation_path().to_string_lossy().into_owned(),
        ));

        grid.row().add();
        grid.cell()
            .unwrap()
            .wrap(create_info_label(l10n!(&game_revision)));
        grid.cell().unwrap().wrap(ReadOnlyText::new({
            let (revision, snapshot) = game.version();
            format!("#{}/{} ({})", revision, snapshot, branch_name)
        }));
        grid.cell()
            .unwrap()
            .wrap(create_info_label(l10n!(&game_build_id)));
        grid.span(1, 2)
            .unwrap()
            .wrap(ReadOnlyText::new(format!("{}", game.build_id())));

        grid.row().add();
        grid.cell()
            .unwrap()
            .wrap(create_info_label(l10n!(&steam_acct_id)));
        let platform_user_id_text = grid.cell().unwrap().wrap(ReadOnlyText::default());
        grid.cell()
            .unwrap()
            .wrap(create_info_label(l10n!(&steam_acct_name)));
        let platform_user_name_text = grid.cell().unwrap().wrap(ReadOnlyText::default());
        let mut refresh_platform_button = grid
            .cell()
            .unwrap()
            .wrap(Button::default())
            .with_label(l10n!(&refresh));

        grid.row().add();
        grid.cell()
            .unwrap()
            .wrap(create_info_label(l10n!(&fls_acct_id)));
        let fls_acct_id_text = grid.cell().unwrap().wrap(ReadOnlyText::default());
        grid.cell()
            .unwrap()
            .wrap(create_info_label(l10n!(&fls_acct_name)));
        let fls_acct_name_text = grid.cell().unwrap().wrap(ReadOnlyText::default());
        let mut refresh_fls_button = grid
            .cell()
            .unwrap()
            .wrap(Button::default())
            .with_label(l10n!(&refresh));

        grid.row().add();
        grid.cell()
            .unwrap()
            .wrap(create_info_label(l10n!(&online_capability)));
        let online_play_text = grid.cell().unwrap().wrap(ReadOnlyText::default());
        grid.cell()
            .unwrap()
            .wrap(create_info_label(l10n!(&singleplayer_capability)));
        let sp_play_text = grid.span(1, 2).unwrap().wrap(ReadOnlyText::default());

        grid.row().add();
        grid.cell()
            .unwrap()
            .wrap(create_info_label(l10n!(&battleye)));
        let mut battleye_input = grid.cell().unwrap().wrap(InputChoice::default_fill());
        grid.cell()
            .unwrap()
            .wrap(create_info_label(l10n!(&log_level)));
        let mut log_level_input = grid.span(1, 2).unwrap().wrap(InputChoice::default_fill());

        grid.row().add();
        grid.cell()
            .unwrap()
            .wrap(create_info_label(l10n!(&language)));
        let mut language_input = grid.cell().unwrap().wrap(InputChoice::default_fill());
        grid.cell().unwrap().wrap(create_info_label(l10n!(&theme)));
        let mut theme_input = grid.span(1, 2).unwrap().wrap(InputChoice::default_fill());

        grid.row().add();
        grid.span(1, 3).unwrap().skip();
        let mut privacy_switch = grid
            .span(1, 2)
            .unwrap()
            .wrap(LightButton::default())
            .with_label(l10n!(&privacy_switch));

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
            .wrap(create_info_label(l10n!(&last_session)));
        let last_session_text = last_session_grid
            .cell()
            .unwrap()
            .wrap(ReadOnlyText::new(last_session_text(&localizer, &*game)));
        let last_session_grid = last_session_grid.end();

        action_grid.row().add();
        action_grid.span(1, 2).unwrap().skip();
        action_grid.span(1, 2).unwrap().add(last_session_grid);

        action_grid.row().with_stretch(1).add();
        let cell = action_grid.cell().unwrap();
        let switch_branch_button = if can_switch_branch {
            let button = Button::default()
                .with_label(&l10n!(switch_branch, branch => other_branch_name.as_str()));
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
        let mut launch_button = Button::default().with_label(l10n!(&launch));
        action_grid
            .cell()
            .unwrap()
            .add(BigButtonElement::wrap(launch_button.clone()));
        let mut continue_button = Button::default().with_label(l10n!(&continue));
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

        refresh_platform_button.deactivate();
        refresh_fls_button.deactivate();

        battleye_input.input().set_readonly(true);
        battleye_input.input().clear_visible_focus();
        battleye_input.add(l10n!(&battleye.always));
        battleye_input.add(l10n!(&battleye.never));
        battleye_input.add(l10n!(&battleye.auto));
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

        log_level_input.input().set_readonly(true);
        log_level_input.input().clear_visible_focus();
        log_level_input.add(l10n!(&log_level.off));
        log_level_input.add(l10n!(&log_level.trace));
        log_level_input.add(l10n!(&log_level.debug));
        log_level_input.add(l10n!(&log_level.info));
        log_level_input.add(l10n!(&log_level.warn));
        log_level_input.add(l10n!(&log_level.error));
        log_level_input.add(l10n!(&log_level.crit));
        log_level_input.set_value_index(log_level_to_index(&config.log_level));
        log_level_input.set_callback({
            let on_action = Rc::clone(&on_action);
            move |input| {
                let log_level = index_to_log_level(input.menu_button().value());
                on_action(HomeAction::ConfigureLogLevel(log_level)).unwrap();
            }
        });
        log_level_input.set_activated(!log_level_overridden);

        let mut sorted_locales: Vec<_> = available_locales.keys().cloned().collect();
        sorted_locales.sort_by(|lhs, rhs| available_locales[lhs].cmp(&available_locales[rhs]));
        language_input.input().set_readonly(true);
        language_input.input().clear_visible_focus();
        language_input
            .add(&l10n!(language.default, locale => available_locales[&default_locale].as_str()));
        language_input.set_value_index(0);
        for (idx, locale) in sorted_locales.iter().enumerate() {
            language_input.add(&available_locales[locale]);
            if config.locale.as_ref() == Some(locale) {
                language_input.set_value_index((idx + 1) as _);
            }
        }
        language_input.set_callback({
            let on_action = Rc::clone(&on_action);
            move |input| {
                let locale = match input.menu_button().value() {
                    0 => None,
                    idx @ _ => Some(sorted_locales[(idx - 1) as usize].clone()),
                };
                on_action(HomeAction::ConfigureLocale(locale)).unwrap();
            }
        });

        theme_input.input().set_readonly(true);
        theme_input.input().clear_visible_focus();
        theme_input.add(l10n!(&theme.light));
        theme_input.add(l10n!(&theme.dark));
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
                let color = if btn.value() { Color::Background2 } else { Color::Foreground };
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
            let localizer = Rc::clone(&localizer);
            let logger = logger.clone();
            move |_| {
                use_l10n!(localizer => inner_l10n);
                if let Err(err) = on_action(HomeAction::Launch) {
                    error!(logger, "Error launching game"; "error" => %err);
                    alert_error(inner_l10n!(&err_launching_game), &err);
                }
            }
        });
        continue_button.set_callback({
            let on_action = Rc::clone(&on_action);
            let localizer = Rc::clone(&localizer);
            let logger = logger.clone();
            move |_| {
                use_l10n!(localizer => inner_l10n);
                if let Err(err) = on_action(HomeAction::Continue) {
                    error!(logger, "Error launching game"; "error" => %err);
                    alert_error(inner_l10n!(&err_launching_game), &err);
                }
            }
        });
        if let Some(mut button) = switch_branch_button {
            button.set_callback({
                let on_action = Rc::clone(&on_action);
                let localizer = Rc::clone(&localizer);
                let logger = logger.clone();
                move |_| {
                    use_l10n!(localizer => inner_l10n);
                    if let Err(err) = on_action(HomeAction::SwitchBranch(other_branch)) {
                        error!(
                            logger,
                            "Error switching to other branch";
                            "branch" => ?other_branch,
                            "error" => %err,
                        );
                        alert_error(
                            &inner_l10n!(
                                err_switching_branch,
                                branch => other_branch_name.as_str()
                            ),
                            &err,
                        );
                    }
                }
            });
        }

        let _ = launch_button.take_focus();

        Rc::new(Self {
            root,
            localizer,
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

    pub fn show(&self) -> CleanupFn {
        let mut root = self.root.clone();
        root.show();

        Box::new(move || {
            root.hide();
        })
    }

    pub fn handle_update(&self, update: HomeUpdate) {
        match update {
            HomeUpdate::LastSession => self
                .last_session_text
                .set_value(last_session_text(&self.localizer, &self.game)),
            HomeUpdate::AuthState(state) => self.update_auth_state(state),
        }
    }

    fn update_auth_state(&self, state: AuthState) {
        use_l10n!(self.localizer);
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
            TaskState::Pending => (l10n!(fls_fetching), l10n!(fls_fetching), false),
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
            TaskState::Pending => l10n!(capability_checking),
            TaskState::Ready(Ok(())) => l10n!(capability_yes),
            TaskState::Ready(Err(err)) => l10n!(capability_no, reason => err.to_string()),
        };
        self.online_play_text.set_value(online_play_str);

        let sp_play_str = match state.sp_capability {
            TaskState::Pending => l10n!(capability_checking),
            TaskState::Ready(Ok(())) => l10n!(capability_yes),
            TaskState::Ready(Err(err)) => l10n!(capability_no, reason => err.to_string()),
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

fn last_session_text(localizer: &Localizer, game: &Game) -> String {
    use_l10n!(localizer);
    match &*game.last_session() {
        None => l10n!(last_session.none),
        Some(Session::SinglePlayer(map_ref)) => {
            l10n!(last_session.singleplayer, map => map_ref_text(localizer, game.maps(), map_ref))
        }
        Some(Session::CoOp(map_ref)) => {
            l10n!(last_session.coop, map => map_ref_text(localizer, game.maps(), map_ref))
        }
        Some(Session::Online(server_ref)) => {
            l10n!(last_session.online, server => server_ref_text(server_ref))
        }
    }
}

fn map_ref_text(localizer: &Localizer, maps: &Maps, map_ref: &MapRef) -> String {
    use_l10n!(localizer);
    match map_ref {
        MapRef::Known { map_id } => maps[*map_id].display_name.clone(),
        MapRef::Unknown { asset_path } => {
            l10n!(last_session.unknown_map, asset_path => asset_path.as_str())
        }
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
