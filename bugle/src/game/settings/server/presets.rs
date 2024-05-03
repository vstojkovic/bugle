use crate::game::settings::Multiplier;

use super::ServerSettings;

// Note: All presets set the friendly_fire_dmg_mult to 0.25, because that's what the game does,
// even though the GUI only supports increments of 0.1 and the default value, when not specified in
// the .ini file, is 0.2

pub fn civilized() -> ServerSettings {
    let mut result = ServerSettings::default();
    result.progression.xp_rate_mult = Multiplier(1.25);
    result.survival.idle_thirst_mult = Multiplier(0.5);
    result.survival.active_thirst_mult = Multiplier(0.5);
    result.survival.idle_hunger_mult = Multiplier(0.5);
    result.survival.active_hunger_mult = Multiplier(0.5);
    result.combat.player_dmg_mult = Multiplier(1.2);
    result.combat.npc_dmg_mult = Multiplier(0.8);
    result.combat.friendly_fire_dmg_mult = Multiplier(0.25);
    result.harvesting.harvest_amount_mult = Multiplier(1.2);
    result.harvesting.item_spoil_rate_mult = Multiplier(0.5);
    result.crafting.crafting_time_mult = Multiplier(0.5);
    result.crafting.thrall_crafting_time_mult = Multiplier(0.5);
    result.building.thrall_decay_disabled = true;
    result.maelstrom.storm_endurance_drain_mult = Multiplier(0.1);
    result
}

pub fn decadent() -> ServerSettings {
    let mut result = ServerSettings::default();
    result.combat.friendly_fire_dmg_mult = Multiplier(0.25);
    result.building.thrall_decay_disabled = true;
    result.maelstrom.storm_endurance_drain_mult = Multiplier(0.1);
    result
}

pub fn barbaric() -> ServerSettings {
    let mut result = ServerSettings::default();
    result.progression.xp_time_mult = Multiplier(0.0);
    result.survival.idle_thirst_mult = Multiplier(1.1);
    result.survival.active_thirst_mult = Multiplier(1.1);
    result.survival.idle_hunger_mult = Multiplier(1.1);
    result.survival.active_hunger_mult = Multiplier(1.1);
    result.combat.player_dmg_mult = Multiplier(0.8);
    result.combat.npc_dmg_mult = Multiplier(1.2);
    result.combat.friendly_fire_dmg_mult = Multiplier(0.25);
    result.harvesting.harvest_amount_mult = Multiplier(0.8);
    result.harvesting.item_spoil_rate_mult = Multiplier(1.1);
    result.building.abandonment_disabled = false;
    result.building.thrall_decay_disabled = true;
    result.maelstrom.storm_endurance_drain_mult = Multiplier(0.1);
    result
}
