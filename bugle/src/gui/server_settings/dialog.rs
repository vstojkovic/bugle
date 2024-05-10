use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

use fltk::button::{Button, ReturnButton};
use fltk::dialog::{FileDialogOptions, FileDialogType, NativeFileChooser};
use fltk::enums::Shortcut;
use fltk::menu::{MenuButton, MenuFlag};
use fltk::prelude::*;
use fltk::window::Window;
use fltk_float::grid::{Grid, GridBuilder};
use slog::{error, warn, Logger};

use crate::game::settings::server::{Preset, ServerSettings};
use crate::game::Game;
use crate::gui::{alert_error, wrapper_factory};
use crate::util::weak_cb;

use super::tabs::SettingsTabs;

pub struct ServerSettingsDialog {
    logger: Logger,
    game: Arc<Game>,
    window: Window,
    tabs: SettingsTabs,
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

        let tabs = SettingsTabs::new(&mut window, true, &settings.general, &settings.survival);

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
        let window = window.group();

        let this = Rc::new(Self {
            logger: logger.clone(),
            game,
            window: window.clone(),
            tabs,
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
        let private = self.tabs.private_tabs.as_ref().unwrap();
        ServerSettings {
            general: self.tabs.general_tab.values(),
            progression: self.tabs.progression_tab.values(),
            daylight: self.tabs.daylight_tab.values(),
            survival: self.tabs.survival_tab.values(),
            combat: self.tabs.combat_tab.values(),
            harvesting: self.tabs.harvesting_tab.values(),
            crafting: self.tabs.crafting_tab.values(),
            building: private.building_tab.values(),
            chat: private.chat_tab.values(),
            followers: private.followers_tab.values(),
            maelstrom: private.maelstrom_tab.values(),
        }
    }

    fn set_values(&self, settings: &ServerSettings) {
        let private = self.tabs.private_tabs.as_ref().unwrap();
        self.tabs.general_tab.set_values(&settings.general);
        self.tabs.progression_tab.set_values(&settings.progression);
        self.tabs.daylight_tab.set_values(&settings.daylight);
        self.tabs.survival_tab.set_values(&settings.survival);
        self.tabs.combat_tab.set_values(&settings.combat);
        self.tabs.harvesting_tab.set_values(&settings.harvesting);
        self.tabs.crafting_tab.set_values(&settings.crafting);
        private.building_tab.set_values(&settings.building);
        private.chat_tab.set_values(&settings.chat);
        private.followers_tab.set_values(&settings.followers);
        private.maelstrom_tab.set_values(&settings.maelstrom);
    }
}

const DLG_FILTER_INI: &str = "Ini Files\t*.ini";
const ERR_IMPORTING_SETTINGS: &str = "Error while importing the settings.";
const ERR_EXPORTING_SETTINGS: &str = "Error while exporting the settings.";
