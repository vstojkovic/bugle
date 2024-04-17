use std::borrow::Borrow;
use std::cell::Cell;
use std::rc::Rc;

use fltk::button::{Button, RadioButton, ReturnButton};
use fltk::group::{Group, Wizard};
use fltk::prelude::*;
use fltk::window::Window;
use fltk_float::grid::{CellAlign, Grid, GridBuilder};
use fltk_float::overlay::OverlayBuilder;
use fltk_float::{LayoutElement, WrapperFactory};

use crate::game::settings::server::ServerSettings;
use crate::gui::prelude::declare_weak_cb;
use crate::gui::wrapper_factory;

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
    pub fn new(settings: ServerSettings) -> Rc<Self> {
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
        let general_tab = add_tab(&mut content, GeneralTab::new(settings.general));
        let progression_tab = add_tab(&mut content, ProgressionTab::new(settings.progression));
        let daylight_tab = add_tab(&mut content, DaylightTab::new(settings.daylight));
        let survival_tab = add_tab(&mut content, SurvivalTab::new(settings.survival));
        let combat_tab = add_tab(&mut content, CombatTab::new(settings.combat));
        let harvesting_tab = add_tab(&mut content, HarvestingTab::new(settings.harvesting));
        let crafting_tab = add_tab(&mut content, CraftingTab::new(settings.crafting));
        let building_tab = add_tab(&mut content, BuildingTab::new(settings.building));
        let chat_tab = add_tab(&mut content, ChatTab::new(settings.chat));
        let followers_tab = add_tab(&mut content, FollowersTab::new(settings.followers));
        let maelstrom_tab = add_tab(&mut content, MaelstromTab::new(settings.maelstrom));
        let content = content.end();
        let mut content_group = content.group();
        window.cell().unwrap().add(content);

        window.row().add();
        let mut actions = Grid::builder_with_factory(wrapper_factory())
            .with_col_spacing(10)
            .with_top_padding(10);
        actions.row().add();
        actions.col().with_stretch(1).add();
        let col_group = actions.col_group().add();
        actions.extend_group(col_group).batch(2);
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

        ok_button.set_callback(this.weak_cb(Self::ok_clicked));
        cancel_button.set_callback(this.weak_cb(Self::cancel_clicked));

        this
    }

    pub fn run(&self) -> Option<ServerSettings> {
        let mut window = self.window.clone();
        window.make_modal(true);
        window.show();

        while window.shown() {
            fltk::app::wait();
        }

        self.result.take()
    }

    declare_weak_cb!();

    fn ok_clicked(&self) {
        self.result.set(Some(self.values()));
        self.window.clone().hide();
    }

    fn cancel_clicked(&self) {
        self.result.set(None);
        self.window.clone().hide();
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
}

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
