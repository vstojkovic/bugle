use std::ops::{Deref, DerefMut};

use chrono::TimeDelta;
use ini_persist::load::LoadProperty;
use ini_persist::save::{default_remove, SaveProperty};
use serde::{Deserialize, Serialize};

use crate::game::settings::{display_seconds, parse_seconds, Multiplier, WeeklyHours};

#[derive(Clone, Debug, Default, Deserialize, Serialize, LoadProperty, SaveProperty)]
pub struct BaseCombatSettings {
    #[serde(rename = "S6", default)]
    #[ini(rename = "DurabilityMultiplier")]
    pub durability_mult: Multiplier,

    #[serde(rename = "So")]
    #[serde(
        skip_serializing_if = "is_default_thrall_wakeup_time",
        with = "secs_serde",
        default = "default_thrall_wakeup_time"
    )]
    #[ini(rename = "UnconsciousTimeSeconds", parse_with = parse_seconds, remove_with = default_remove, display_with = display_seconds)]
    pub thrall_wakeup_time: TimeDelta,
}

#[derive(Debug, Clone, LoadProperty, SaveProperty)]
pub struct CombatSettings {
    #[ini(flatten)]
    pub base: BaseCombatSettings,

    #[ini(rename = "PlayerDamageMultiplier")]
    pub player_dmg_mult: Multiplier,

    #[ini(rename = "PlayerDamageTakenMultiplier")]
    pub player_dmg_recv_mult: Multiplier,

    #[ini(rename = "NPCDamageMultiplier")]
    pub npc_dmg_mult: Multiplier,

    #[ini(rename = "NPCDamageTakenMultiplier")]
    pub npc_dmg_recv_mult: Multiplier,

    #[ini(rename = "ThrallDamageToPlayersMultiplier")]
    pub thrall_player_dmg_mult: Multiplier,

    #[ini(rename = "ThrallDamageToNPCsMultiplier")]
    pub thrall_npc_dmg_mult: Multiplier,

    #[ini(rename = "NPCRespawnMultiplier")]
    pub npc_respawn_mult: Multiplier,

    #[ini(rename = "FriendlyFireDamageMultiplier")]
    pub friendly_fire_dmg_mult: Multiplier,

    #[ini(rename = "BuildingDamageMultiplier")]
    pub raid_dmg_mult: Multiplier,

    #[ini(rename = "AvatarsDisabled")]
    pub gods_disabled: bool,

    #[ini(rename = "RestrictAvatarSummoningTime")]
    pub gods_restricted: bool,

    #[ini(rename = "AvatarSummoningTime")]
    pub gods_hours: WeeklyHours,

    #[ini(rename = "EnableTargetLock")]
    pub aim_lock_enabled: bool,
}

impl Deref for CombatSettings {
    type Target = BaseCombatSettings;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for CombatSettings {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl Default for CombatSettings {
    fn default() -> Self {
        Self {
            base: Default::default(),
            player_dmg_mult: Multiplier(1.0),
            player_dmg_recv_mult: Multiplier(1.0),
            npc_dmg_mult: Multiplier(1.0),
            npc_dmg_recv_mult: Multiplier(1.0),
            thrall_player_dmg_mult: Multiplier(0.5),
            thrall_npc_dmg_mult: Multiplier(0.5),
            npc_respawn_mult: Multiplier(1.0),
            friendly_fire_dmg_mult: Multiplier(0.2),
            raid_dmg_mult: Multiplier(1.0),
            gods_disabled: false,
            gods_restricted: false,
            gods_hours: Default::default(),
            aim_lock_enabled: true,
        }
    }
}

fn is_default_thrall_wakeup_time(time: &TimeDelta) -> bool {
    *time == default_thrall_wakeup_time()
}

fn default_thrall_wakeup_time() -> TimeDelta {
    TimeDelta::try_seconds(1800).unwrap()
}

mod secs_serde {
    use chrono::TimeDelta;
    use serde::de::{Error, Unexpected};
    use serde::{Deserialize, Serialize};

    pub fn serialize<S: serde::Serializer>(
        delta: &TimeDelta,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let secs = delta.num_seconds() as f64 + (delta.subsec_nanos() as f64) / 1_000_000_000.0;
        secs.serialize(serializer)
    }

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<TimeDelta, D::Error> {
        let secs = f64::deserialize(deserializer)?;
        let sec_part = secs as i64;
        let nano_part = (secs.fract().abs() * 1_000_000_000.0) as u32;
        TimeDelta::new(sec_part, nano_part).ok_or_else(|| {
            D::Error::invalid_value(Unexpected::Float(secs), &"an interval in seconds")
        })
    }
}
