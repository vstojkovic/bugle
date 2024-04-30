use std::fs::OpenOptions;
use std::path::Path;

use anyhow::Result;
use ini_persist::load::{IniLoad, LoadProperty};
use ini_persist::save::{IniSave, SaveProperty};

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
mod progression;
mod survival;

pub use self::building::{BuildingSettings, CreativeMode};
pub use self::chat::ChatSettings;
pub use self::combat::{BaseCombatSettings, CombatSettings};
pub use self::crafting::{BaseCraftingSettings, CraftingSettings};
pub use self::daylight::{BaseDaylightSettings, DaylightSettings};
pub use self::followers::FollowerSettings;
pub use self::general::{
    BaseGeneralSettings, CombatModeModifier, Community, EventLogPrivacy, GeneralSettings,
    OnlinePlayerInfoVisibility,
};
pub use self::harvesting::{BaseHarvestingSettings, HarvestingSettings};
pub use self::maelstrom::MaelstromSettings;
pub use self::progression::{BaseProgressionSettings, ProgressionSettings};
pub use self::survival::{BaseSurvivalSettings, DropOnDeath, SurvivalSettings};

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
}
