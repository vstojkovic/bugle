use std::borrow::Borrow;
use std::rc::Rc;

use fltk::button::RadioButton;
use fltk::group::{Group, Wizard};
use fltk::prelude::*;
use fltk_float::grid::{CellAlign, Grid, GridBuilder};
use fltk_float::overlay::OverlayBuilder;
use fltk_float::{LayoutElement, WrapperFactory};

mod building;
mod chat;
mod combat;
mod crafting;
mod daylight;
mod followers;
mod general;
mod harvesting;
mod maelstrom;
mod progression;
mod survival;

use crate::game::settings::server::{PublicGeneralSettings, PublicSurvivalSettings};
use crate::gui::wrapper_factory;

use self::building::BuildingTab;
use self::chat::ChatTab;
use self::combat::CombatTab;
use self::crafting::CraftingTab;
use self::daylight::DaylightTab;
use self::followers::FollowersTab;
use self::general::GeneralTab;
use self::harvesting::HarvestingTab;
use self::maelstrom::MaelstromTab;
use self::progression::ProgressionTab;
use self::survival::SurvivalTab;

trait SettingsTab {
    fn root(&self) -> impl WidgetExt + 'static;
}

pub struct SettingsTabs {
    pub general_tab: Rc<GeneralTab>,
    pub progression_tab: Rc<ProgressionTab>,
    pub daylight_tab: Rc<DaylightTab>,
    pub survival_tab: Rc<SurvivalTab>,
    pub combat_tab: Rc<CombatTab>,
    pub harvesting_tab: Rc<HarvestingTab>,
    pub crafting_tab: Rc<CraftingTab>,
    pub private_tabs: Option<PrivateTabs>,
}

pub struct PrivateTabs {
    pub building_tab: Rc<BuildingTab>,
    pub chat_tab: Rc<ChatTab>,
    pub followers_tab: Rc<FollowersTab>,
    pub maelstrom_tab: Rc<MaelstromTab>,
}

impl SettingsTabs {
    pub fn new<G: GroupExt + Clone, F: Borrow<WrapperFactory>>(
        grid: &mut GridBuilder<G, F>,
        build_private: bool,
        general: &PublicGeneralSettings,
        survival: &PublicSurvivalSettings,
    ) -> Self {
        grid.row()
            .with_stretch(1)
            .with_default_align(CellAlign::Stretch)
            .add();

        grid.col().add();
        let mut tabs = Grid::builder_with_factory(wrapper_factory());
        tabs.col().with_stretch(1).add();
        let mut general_button = tab_button(&mut tabs, "General");
        let mut progression_button = tab_button(&mut tabs, "Progression");
        let mut daylight_button = tab_button(&mut tabs, "Daylight");
        let mut survival_button = tab_button(&mut tabs, "Survival");
        let mut combat_button = tab_button(&mut tabs, "Combat");
        let mut harvesting_button = tab_button(&mut tabs, "Harvesting");
        let mut crafting_button = tab_button(&mut tabs, "Crafting");
        let mut building_button = build_private.then(|| tab_button(&mut tabs, "Building"));
        let mut chat_button = build_private.then(|| tab_button(&mut tabs, "Chat"));
        let mut followers_button = build_private.then(|| tab_button(&mut tabs, "Followers"));
        let mut maelstrom_button = build_private.then(|| tab_button(&mut tabs, "Maelstrom"));
        grid.cell().unwrap().add(tabs.end());

        grid.col().with_stretch(1).add();
        let mut content = OverlayBuilder::with_factory(Wizard::default(), wrapper_factory())
            .with_padding(10, 10, 10, 10);
        let general_tab = add_tab(&mut content, GeneralTab::new(general, build_private));
        let progression_tab = add_tab(&mut content, ProgressionTab::new(build_private));
        let daylight_tab = add_tab(&mut content, DaylightTab::new(build_private));
        let survival_tab = add_tab(&mut content, SurvivalTab::new(survival, build_private));
        let combat_tab = add_tab(&mut content, CombatTab::new(build_private));
        let harvesting_tab = add_tab(&mut content, HarvestingTab::new(build_private));
        let crafting_tab = add_tab(&mut content, CraftingTab::new(build_private));
        let building_tab = build_private.then(|| add_tab(&mut content, BuildingTab::new()));
        let chat_tab = build_private.then(|| add_tab(&mut content, ChatTab::new()));
        let followers_tab = build_private.then(|| add_tab(&mut content, FollowersTab::new()));
        let maelstrom_tab = build_private.then(|| add_tab(&mut content, MaelstromTab::new()));
        let content = content.end();
        let mut content_group = content.group();
        grid.cell().unwrap().add(content);

        content_group.set_current_widget(&general_tab.root());
        general_button.set_value(true);

        set_tab_callback(&mut general_button, &content_group, &general_tab);
        set_tab_callback(&mut progression_button, &content_group, &progression_tab);
        set_tab_callback(&mut daylight_button, &content_group, &daylight_tab);
        set_tab_callback(&mut survival_button, &content_group, &survival_tab);
        set_tab_callback(&mut combat_button, &content_group, &combat_tab);
        set_tab_callback(&mut harvesting_button, &content_group, &harvesting_tab);
        set_tab_callback(&mut crafting_button, &content_group, &crafting_tab);
        set_opt_tab_callback(&mut building_button, &content_group, &building_tab);
        set_opt_tab_callback(&mut chat_button, &content_group, &chat_tab);
        set_opt_tab_callback(&mut followers_button, &content_group, &followers_tab);
        set_opt_tab_callback(&mut maelstrom_button, &content_group, &maelstrom_tab);

        let private_tabs = build_private.then(|| PrivateTabs {
            building_tab: building_tab.unwrap(),
            chat_tab: chat_tab.unwrap(),
            followers_tab: followers_tab.unwrap(),
            maelstrom_tab: maelstrom_tab.unwrap(),
        });

        Self {
            general_tab,
            progression_tab,
            daylight_tab,
            survival_tab,
            combat_tab,
            harvesting_tab,
            crafting_tab,
            private_tabs,
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

fn set_tab_callback<T: SettingsTab>(button: &mut RadioButton, wizard: &Wizard, tab: &Rc<T>) {
    let tab = tab.root();
    let mut wizard = wizard.clone();
    button.set_callback(move |_| wizard.set_current_widget(&tab));
}

fn set_opt_tab_callback<T: SettingsTab>(
    button: &mut Option<RadioButton>,
    wizard: &Wizard,
    tab: &Option<Rc<T>>,
) {
    let (Some(button), Some(tab)) = (button.as_mut(), tab) else {
        return;
    };
    set_tab_callback(button, wizard, tab);
}
