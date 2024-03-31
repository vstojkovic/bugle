use std::ops::{Deref, DerefMut};

use ini_persist::load::LoadProperty;
use ini_persist::save::SaveProperty;
use serde::{Deserialize, Serialize};
use serde_repr::Serialize_repr;

use crate::game::settings::Multiplier;

#[derive(Clone, Debug, Deserialize, Serialize, LoadProperty, SaveProperty)]
pub struct BaseSurvivalSettings {
    #[serde(rename = "Sj", default)]
    #[ini(rename = "StaminaCostMultiplier")]
    pub stamina_cost_mult: Multiplier,

    #[serde(rename = "S21")]
    #[ini(rename = "PlayerIdleThirstMultiplier")]
    pub idle_thirst_mult: Multiplier,

    #[serde(rename = "S22")]
    #[ini(rename = "PlayerActiveThirstMultiplier")]
    pub active_thirst_mult: Multiplier,

    #[serde(rename = "S23")]
    #[ini(rename = "PlayerIdleHungerMultiplier")]
    pub idle_hunger_mult: Multiplier,

    #[serde(rename = "S24")]
    #[ini(rename = "PlayerActiveHungerMultiplier")]
    pub active_hunger_mult: Multiplier,

    #[serde(rename = "S7")]
    #[ini(rename = "DropEquipmentOnDeath")]
    pub drop_items_on_death: DropOnDeath,

    #[serde(rename = "Sa")]
    #[ini(rename = "EverybodyCanLootCorpse")]
    pub anyone_can_loot_corpse: bool,

    #[serde(rename = "S5", default = "default_offline_chars_in_world")]
    #[ini(rename = "LogoutCharactersRemainInTheWorld")]
    pub offline_chars_in_world: bool,
}

impl Default for BaseSurvivalSettings {
    fn default() -> Self {
        Self {
            stamina_cost_mult: Default::default(),
            idle_thirst_mult: Default::default(),
            active_thirst_mult: Default::default(),
            idle_hunger_mult: Default::default(),
            active_hunger_mult: Default::default(),
            drop_items_on_death: Default::default(),
            anyone_can_loot_corpse: false,
            offline_chars_in_world: default_offline_chars_in_world(),
        }
    }
}

#[derive(Debug, Clone, Default, LoadProperty, SaveProperty)]
pub struct SurvivalSettings {
    #[ini(flatten)]
    pub base: BaseSurvivalSettings,

    #[ini(rename = "ThrallCorruptionRemovalMultiplier")]
    pub corruption_removal_mult: Multiplier,

    #[ini(rename = "PlayerCorruptionGainMultiplier")]
    pub corruption_gain_mult: Multiplier,

    #[ini(rename = "PlayerCorruptionGainFromSorceryMultiplier")]
    pub sorcery_corruption_gain_mult: Multiplier,
}

impl Deref for SurvivalSettings {
    type Target = BaseSurvivalSettings;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for SurvivalSettings {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

#[derive(Clone, Copy, Debug, Serialize_repr, LoadProperty, SaveProperty)]
#[repr(u8)]
#[ini(repr)]
pub enum DropOnDeath {
    Nothing,
    All,
    Backpack,
}

impl Default for DropOnDeath {
    fn default() -> Self {
        Self::Nothing
    }
}

impl<'de> Deserialize<'de> for DropOnDeath {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Visitor;

        struct DropOnDeathVisitor;

        impl<'de> Visitor<'de> for DropOnDeathVisitor {
            type Value = DropOnDeath;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("false, true, 0, 1, or 2")
            }

            fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Self::Value, E> {
                Ok(match v {
                    false => DropOnDeath::Nothing,
                    true => DropOnDeath::All,
                })
            }

            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Self::Value, E> {
                if v >= 0 {
                    self.visit_u64(v as u64)
                } else {
                    Err(E::invalid_value(
                        serde::de::Unexpected::Signed(v),
                        &"0, 1, or 2",
                    ))
                }
            }

            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Self::Value, E> {
                match v {
                    0 => Ok(DropOnDeath::Nothing),
                    1 => Ok(DropOnDeath::All),
                    2 => Ok(DropOnDeath::Backpack),
                    _ => Err(E::invalid_value(
                        serde::de::Unexpected::Unsigned(v),
                        &"0, 1, or 2",
                    )),
                }
            }
        }

        deserializer.deserialize_any(DropOnDeathVisitor)
    }
}

#[inline(always)]
const fn default_offline_chars_in_world() -> bool {
    true
}
