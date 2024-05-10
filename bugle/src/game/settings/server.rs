use std::fs::OpenOptions;
use std::path::Path;

use anyhow::Result;
use ini_persist::load::{IniLoad, LoadProperty};
use ini_persist::save::{IniSave, SaveProperty};
use strum_macros::{EnumIter, FromRepr};

use crate::config;

mod building;
mod chat;
mod combat;
mod crafting;
mod daylight;
mod followers;
mod general;
mod harvesting;
mod maelstrom;
mod presets;
mod progression;
mod survival;

pub use self::building::{BuildingSettings, CreativeMode};
pub use self::chat::ChatSettings;
pub use self::combat::{CombatSettings, PublicCombatSettings};
pub use self::crafting::{CraftingSettings, PublicCraftingSettings};
pub use self::daylight::{DaylightSettings, PublicDaylightSettings};
pub use self::followers::FollowerSettings;
pub use self::general::{
    CombatModeModifier, Community, EventLogPrivacy, GeneralSettings, OnlinePlayerInfoVisibility,
    PublicGeneralSettings,
};
pub use self::harvesting::{HarvestingSettings, PublicHarvestingSettings};
pub use self::maelstrom::MaelstromSettings;
pub use self::progression::{ProgressionSettings, PublicProgressionSettings};
pub use self::survival::{DropOnDeath, PublicSurvivalSettings, SurvivalSettings};

use super::Nudity;

#[derive(Debug, Clone, Default, IniLoad, IniSave)]
pub struct ServerSettingsFile {
    #[ini(section = "ServerSettings")]
    pub settings: ServerSettings,
}

#[derive(Debug, Clone, Default, LoadProperty, SaveProperty)]
pub struct ServerSettings {
    #[ini(flatten)]
    pub general: GeneralSettings,

    #[ini(flatten)]
    pub progression: ProgressionSettings,

    #[ini(flatten)]
    pub daylight: DaylightSettings,

    #[ini(flatten)]
    pub survival: SurvivalSettings,

    #[ini(flatten)]
    pub combat: CombatSettings,

    #[ini(flatten)]
    pub harvesting: HarvestingSettings,

    #[ini(flatten)]
    pub crafting: CraftingSettings,

    #[ini(flatten)]
    pub building: BuildingSettings,

    #[ini(flatten)]
    pub chat: ChatSettings,

    #[ini(flatten)]
    pub followers: FollowerSettings,

    #[ini(flatten)]
    pub maelstrom: MaelstromSettings,
}

#[derive(Debug, Clone, Copy, EnumIter, FromRepr)]
pub enum Preset {
    Civilized,
    Decadent,
    Barbaric,
}

impl ServerSettings {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path.as_ref())?;
        let ini = config::load_ini_from_file(file)?;
        let mut file = ServerSettingsFile::default();
        file.load_from_ini(&ini)?;
        Ok(file.settings)
    }

    pub fn save_to_file<P: AsRef<Path>>(self, path: P) -> Result<()> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path.as_ref())?;
        let mut ini = config::load_ini_from_file(file)?;
        let file = ServerSettingsFile { settings: self };
        file.save_to_ini(&mut ini);
        config::save_ini(&ini, path.as_ref())
    }

    pub fn preset(preset: Preset, nudity: Nudity) -> ServerSettings {
        let mut result = match preset {
            Preset::Civilized => presets::civilized(),
            Preset::Decadent => presets::decadent(),
            Preset::Barbaric => presets::barbaric(),
        };
        result.general.max_nudity = nudity;
        result
    }
}
