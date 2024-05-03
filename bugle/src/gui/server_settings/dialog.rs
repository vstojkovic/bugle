use std::borrow::Borrow;
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

use fltk::button::{Button, RadioButton, ReturnButton};
use fltk::dialog::{FileDialogOptions, FileDialogType, NativeFileChooser};
use fltk::enums::Shortcut;
use fltk::group::{Group, Wizard};
use fltk::menu::{MenuButton, MenuFlag};
use fltk::prelude::*;
use fltk::window::Window;
use fltk_float::grid::{CellAlign, Grid, GridBuilder};
use fltk_float::overlay::OverlayBuilder;
use fltk_float::{LayoutElement, WrapperFactory};
use slog::{error, warn, Logger};

use crate::game::settings::server::{Preset, ServerSettings};
use crate::game::Game;
use crate::gui::{alert_error, wrapper_factory};
use crate::util::weak_cb;

use super::building::BuildingTab;
use super::chat::ChatTab;
use super::combat::CombatTab;
use super::crafting::CraftingTab;
use super::daylight::DaylightTab;
use super::followers::FollowersTab;
use super::general::GeneralTab;
use super::harvesting::HarvestingTab;
use super::maelstrom::MaelstromTab;
use super::progression::ProgressionTab;
use super::survival::SurvivalTab;

pub struct ServerSettingsDialog {
    logger: Logger,
    game: Arc<Game>,
    window: Window,
    general_tab: Rc<GeneralTab>,
    progression_tab: Rc<ProgressionTab>,
    daylight_tab: Rc<DaylightTab>,
    survival_tab: Rc<SurvivalTab>,
    combat_tab: Rc<CombatTab>,
    harvesting_tab: Rc<HarvestingTab>,
    crafting_tab: Rc<CraftingTab>,
    building_tab: Rc<BuildingTab>,
    chat_tab: Rc<ChatTab>,
    followers_tab: Rc<FollowersTab>,
    maelstrom_tab: Rc<MaelstromTab>,
    result: Cell<Option<ServerSettings>>,
}

impl ServerSettingsDialog {
    pub fn new(logger: &Logger, game: Arc<Game>, settings: ServerSettings) -> Rc<Self> {
        let mut window = GridBuilder::with_factory(
            Window::default()
                .with_size(800, 600)
                .with_label("Server Settings"),
            wrapper_factory(),
        )
        .with_padding(10, 10, 10, 10);

        window
            .row()
            .with_stretch(1)
            .with_default_align(CellAlign::Stretch)
            .add();

        window.col().add();
        let mut tabs = Grid::builder_with_factory(wrapper_factory());
        tabs.col().with_stretch(1).add();
        let mut general_button = tab_button(&mut tabs, "General");
        let mut progression_button = tab_button(&mut tabs, "Progression");
        let mut daylight_button = tab_button(&mut tabs, "Daylight");
        let mut survival_button = tab_button(&mut tabs, "Survival");
        let mut combat_button = tab_button(&mut tabs, "Combat");
        let mut harvesting_button = tab_button(&mut tabs, "Harvesting");
        let mut crafting_button = tab_button(&mut tabs, "Crafting");
        let mut building_button = tab_button(&mut tabs, "Building");
        let mut chat_button = tab_button(&mut tabs, "Chat");
        let mut followers_button = tab_button(&mut tabs, "Followers");
        let mut maelstrom_button = tab_button(&mut tabs, "Maelstrom");
        window.cell().unwrap().add(tabs.end());

        window.col().with_stretch(1).add();
        let mut content = OverlayBuilder::with_factory(Wizard::default(), wrapper_factory())
            .with_padding(10, 10, 10, 10);
        let general_tab = add_tab(&mut content, GeneralTab::new(&settings.general));
        let progression_tab = add_tab(&mut content, ProgressionTab::new());
        let daylight_tab = add_tab(&mut content, DaylightTab::new());
        let survival_tab = add_tab(&mut content, SurvivalTab::new(&settings.survival));
        let combat_tab = add_tab(&mut content, CombatTab::new());
        let harvesting_tab = add_tab(&mut content, HarvestingTab::new());
        let crafting_tab = add_tab(&mut content, CraftingTab::new());
        let building_tab = add_tab(&mut content, BuildingTab::new());
        let chat_tab = add_tab(&mut content, ChatTab::new());
        let followers_tab = add_tab(&mut content, FollowersTab::new());
        let maelstrom_tab = add_tab(&mut content, MaelstromTab::new());
        let content = content.end();
        let mut content_group = content.group();
        window.cell().unwrap().add(content);

        window.row().add();
        let mut actions = Grid::builder_with_factory(wrapper_factory())
            .with_col_spacing(10)
            .with_top_padding(10);
        actions.row().add();
        let col_group = actions.col_group().add();
        actions.extend_group(col_group).batch(3);
        actions.col().with_stretch(1).add();
        actions.extend_group(col_group).batch(2);
        let mut import_button = actions
            .cell()
            .unwrap()
            .wrap(Button::default().with_label("Import..."));
        let mut export_button = actions
            .cell()
            .unwrap()
            .wrap(Button::default().with_label("Export..."));
        let mut preset_button = actions
            .cell()
            .unwrap()
            .wrap(MenuButton::default().with_label("Preset"));
        actions.cell().unwrap().skip();
        let mut ok_button = actions
            .cell()
            .unwrap()
            .wrap(ReturnButton::default().with_label("OK"));
        let mut cancel_button = actions
            .cell()
            .unwrap()
            .wrap(Button::default().with_label("Cancel"));
        window.span(1, 2).unwrap().add(actions.end());

        let window = window.end();
        window.layout_children();

        content_group.set_current_widget(&general_tab.root());
        general_button.set_value(true);

        set_tab_callback(&mut general_button, &content_group, general_tab.root());
        set_tab_callback(
            &mut progression_button,
            &content_group,
            progression_tab.root(),
        );
        set_tab_callback(&mut daylight_button, &content_group, daylight_tab.root());
        set_tab_callback(&mut survival_button, &content_group, survival_tab.root());
        set_tab_callback(&mut combat_button, &content_group, combat_tab.root());
        set_tab_callback(
            &mut harvesting_button,
            &content_group,
            harvesting_tab.root(),
        );
        set_tab_callback(&mut crafting_button, &content_group, crafting_tab.root());
        set_tab_callback(&mut building_button, &content_group, building_tab.root());
        set_tab_callback(&mut chat_button, &content_group, chat_tab.root());
        set_tab_callback(&mut followers_button, &content_group, followers_tab.root());
        set_tab_callback(&mut maelstrom_button, &content_group, maelstrom_tab.root());

        let window = window.group();

        let this = Rc::new(Self {
            logger: logger.clone(),
            game,
            window,
            general_tab,
            progression_tab,
            daylight_tab,
            survival_tab,
            combat_tab,
            harvesting_tab,
            crafting_tab,
            building_tab,
            chat_tab,
            followers_tab,
            maelstrom_tab,
            result: Cell::new(None),
        });
        this.set_values(&settings);

        ok_button.set_callback(weak_cb!([this] => |_| this.ok_clicked()));
        cancel_button.set_callback(weak_cb!([this] => |_| this.cancel_clicked()));
        import_button.set_callback(weak_cb!([this] => |_| this.import_clicked()));
        export_button.set_callback(weak_cb!([this] => |_| this.export_clicked()));
        preset_button.add(
            "Civilized",
            Shortcut::None,
            MenuFlag::Normal,
            weak_cb!([this] => |_| {
                this.preset_clicked(Preset::Civilized);
            }),
        );
        preset_button.add(
            "Decadent",
            Shortcut::None,
            MenuFlag::Normal,
            weak_cb!([this] => |_| {
                this.preset_clicked(Preset::Decadent);
            }),
        );
        preset_button.add(
            "Barbaric",
            Shortcut::None,
            MenuFlag::Normal,
            weak_cb!([this] => |_| {
                this.preset_clicked(Preset::Barbaric);
            }),
        );

        this
    }

    pub fn run(&self) -> Option<ServerSettings> {
        let mut window = self.window.clone();
        window.make_modal(true);
        window.show();

        while window.shown() && !fltk::app::should_program_quit() {
            fltk::app::wait();
        }

        self.result.take()
    }

    fn ok_clicked(&self) {
        self.result.set(Some(self.values()));
        self.window.clone().hide();
    }

    fn cancel_clicked(&self) {
        self.result.set(None);
        self.window.clone().hide();
    }

    fn import_clicked(&self) {
        let mut dialog = NativeFileChooser::new(FileDialogType::BrowseFile);
        dialog.set_filter(DLG_FILTER_INI);
        dialog.set_directory(&self.game.config_path()).ok();
        dialog.show();

        let path = dialog.filename();
        if path.as_os_str().is_empty() {
            return;
        }

        let settings = match ServerSettings::load_from_file(path) {
            Ok(settings) => settings,
            Err(err) => {
                error!(self.logger, "Error importing settings"; "error" => %err);
                alert_error(ERR_IMPORTING_SETTINGS, &err);
                return;
            }
        };
        self.set_values(&settings);
    }

    fn export_clicked(&self) {
        let mut dialog = NativeFileChooser::new(FileDialogType::BrowseSaveFile);
        dialog.set_filter(DLG_FILTER_INI);
        dialog.set_directory(&self.game.config_path()).ok();
        dialog.set_option(FileDialogOptions::SaveAsConfirm);
        dialog.show();

        let mut path = dialog.filename();
        if path.as_os_str().is_empty() {
            return;
        }
        if path.extension().is_none() {
            path.set_extension("ini");
        }
        if let Err(err) = self.values().save_to_file(path) {
            error!(self.logger, "Error exporting settings"; "error" => %err);
            alert_error(ERR_EXPORTING_SETTINGS, &err);
        }
    }

    fn preset_clicked(&self, preset: Preset) {
        let nudity = match self.game.max_nudity() {
            Ok(nudity) => nudity,
            Err(err) => {
                warn!(self.logger, "Error reading game settings"; "error" => %err);
                Default::default()
            }
        };
        let settings = ServerSettings::preset(preset, nudity);
        self.set_values(&settings);
    }

    fn values(&self) -> ServerSettings {
        ServerSettings {
            general: self.general_tab.values(),
            progression: self.progression_tab.values(),
            daylight: self.daylight_tab.values(),
            survival: self.survival_tab.values(),
            combat: self.combat_tab.values(),
            harvesting: self.harvesting_tab.values(),
            crafting: self.crafting_tab.values(),
            building: self.building_tab.values(),
            chat: self.chat_tab.values(),
            followers: self.followers_tab.values(),
            maelstrom: self.maelstrom_tab.values(),
        }
    }

    fn set_values(&self, settings: &ServerSettings) {
        self.general_tab.set_values(&settings.general);
        self.progression_tab.set_values(&settings.progression);
        self.daylight_tab.set_values(&settings.daylight);
        self.survival_tab.set_values(&settings.survival);
        self.combat_tab.set_values(&settings.combat);
        self.harvesting_tab.set_values(&settings.harvesting);
        self.crafting_tab.set_values(&settings.crafting);
        self.building_tab.set_values(&settings.building);
        self.chat_tab.set_values(&settings.chat);
        self.followers_tab.set_values(&settings.followers);
        self.maelstrom_tab.set_values(&settings.maelstrom);
    }
}

const DLG_FILTER_INI: &str = "Ini Files\t*.ini";
const ERR_IMPORTING_SETTINGS: &str = "Error while importing the settings.";
const ERR_EXPORTING_SETTINGS: &str = "Error while exporting the settings.";

fn tab_button<F: Borrow<WrapperFactory>>(
    tabs: &mut GridBuilder<Group, F>,
    label: &str,
) -> RadioButton {
    tabs.row()
        .with_stretch(1)
        .with_default_align(CellAlign::Stretch)
        .add();
    let mut button = tabs
        .cell()
        .unwrap()
        .wrap(RadioButton::default().with_label(label));
    button.clear_visible_focus();
    button
}

fn add_tab<F: Borrow<WrapperFactory>, E: LayoutElement + 'static>(
    content: &mut OverlayBuilder<Wizard, F>,
    tab: Rc<E>,
) -> Rc<E> {
    content.add_shared(Rc::<E>::clone(&tab));
    tab
}

fn set_tab_callback(button: &mut RadioButton, wizard: &Wizard, tab: impl WidgetExt + 'static) {
    let mut wizard = wizard.clone();
    button.set_callback(move |_| wizard.set_current_widget(&tab));
}
