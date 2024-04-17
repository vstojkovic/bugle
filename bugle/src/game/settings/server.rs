use ini_persist::load::LoadProperty;
use ini_persist::save::SaveProperty;

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
