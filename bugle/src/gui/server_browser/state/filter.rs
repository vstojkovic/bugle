use std::ops::{Deref, DerefMut};
use std::str::FromStr;

use ini_persist::load::ParseProperty;
use ini_persist::save::DisplayProperty;
use regex::{Regex, RegexBuilder};

use crate::config::ServerBrowserConfig;
use crate::gui::data::RowFilter;
use crate::servers::{EnumFilter, RangeFilter, Server};

#[derive(Clone, Debug)]
pub struct Filter {
    values: crate::servers::Filter,
    name_re: Regex,
    map_re: Regex,
}

impl Filter {
    pub fn from_config(config: &ServerBrowserConfig) -> Self {
        Self {
            values: config.filter.clone(),
            name_re: Self::regex(&config.filter.name),
            map_re: Self::regex(&config.filter.map),
        }
    }

    pub fn name(&self) -> &str {
        &self.values.name
    }

    pub fn set_name(&mut self, name: String) {
        self.name_re = Self::regex(&name);
        self.values.name = name;
    }

    pub fn map(&self) -> &str {
        &self.values.map
    }

    pub fn set_map(&mut self, map: String) {
        self.map_re = Self::regex(&map);
        self.values.map = map;
    }

    fn regex(text: &str) -> Regex {
        RegexBuilder::new(&regex::escape(&text))
            .case_insensitive(true)
            .build()
            .unwrap()
    }
}

impl Deref for Filter {
    type Target = crate::servers::Filter;
    fn deref(&self) -> &Self::Target {
        &self.values
    }
}

impl DerefMut for Filter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.values
    }
}

impl AsRef<crate::servers::Filter> for Filter {
    fn as_ref(&self) -> &crate::servers::Filter {
        &self.values
    }
}

trait PropertyFilter<T> {
    fn matches(&self, getter: impl FnOnce() -> T) -> bool;
}

impl<T, F: PropertyFilter<T>> PropertyFilter<T> for Option<F> {
    fn matches(&self, getter: impl FnOnce() -> T) -> bool {
        self.as_ref().map_or(true, |filter| filter.matches(getter))
    }
}

impl PropertyFilter<bool> for bool {
    fn matches(&self, getter: impl FnOnce() -> bool) -> bool {
        *self == getter()
    }
}

impl<T: ParseProperty + DisplayProperty + Copy + PartialOrd> PropertyFilter<T> for RangeFilter<T> {
    fn matches(&self, getter: impl FnOnce() -> T) -> bool {
        self.matches(getter())
    }
}

impl<T: FromStr + Into<&'static str> + Copy + Eq> PropertyFilter<T> for EnumFilter<T> {
    fn matches(&self, getter: impl FnOnce() -> T) -> bool {
        self.matches(getter())
    }
}

impl RowFilter<Server> for Filter {
    fn matches(&self, server: &Server) -> bool {
        !server.tombstone
            && self.name_re.is_match(&server.name)
            && self.map_re.is_match(&server.map)
            && self.values.type_filter.matches(server)
            && self.values.mode.map_or(true, |mode| server.mode() == mode)
            && self
                .values
                .region
                .map_or(true, |region| server.region == region)
            && self.values.battleye_required.map_or(true, |required| {
                server.general.battleye_required == required
            })
            && self.values.include_invalid >= !server.is_valid()
            && (self.values.include_password_protected || !server.password_protected)
            && self
                .values
                .mods
                .map_or(true, |mods| server.is_modded() == mods)
            && self.values.community.matches(|| server.general.community)
            && self
                .values
                .max_clan_size
                .matches(|| server.general.max_clan_size)
            && self
                .values
                .raid_enabled
                .matches(|| server.general.raid_enabled)
            && self
                .values
                .raid_restricted
                .matches(|| server.general.raid_restricted)
            && self
                .values
                .xp_rate_mult
                .matches(|| server.progression.xp_rate_mult)
            && self
                .values
                .day_cycle_speed_mult
                .matches(|| server.daylight.day_cycle_speed_mult)
            && self
                .values
                .dawn_dusk_speed_mult
                .matches(|| server.daylight.dawn_dusk_speed_mult)
            && self
                .values
                .use_catch_up_time
                .matches(|| server.daylight.use_catch_up_time)
            && self
                .values
                .stamina_cost_mult
                .matches(|| server.survival.stamina_cost_mult)
            && self
                .values
                .idle_thirst_mult
                .matches(|| server.survival.idle_thirst_mult)
            && self
                .values
                .active_thirst_mult
                .matches(|| server.survival.active_thirst_mult)
            && self
                .values
                .idle_hunger_mult
                .matches(|| server.survival.idle_hunger_mult)
            && self
                .values
                .active_hunger_mult
                .matches(|| server.survival.active_hunger_mult)
            && self
                .values
                .drop_items_on_death
                .matches(|| server.survival.drop_items_on_death)
            && self
                .values
                .anyone_can_loot_corpse
                .matches(|| server.survival.anyone_can_loot_corpse)
            && self
                .values
                .offline_chars_in_world
                .matches(|| server.survival.offline_chars_in_world)
            && self
                .values
                .durability_mult
                .matches(|| server.combat.durability_mult)
            && self
                .values
                .thrall_wakeup_time_secs
                .matches(|| server.combat.thrall_wakeup_time.num_seconds())
            && self
                .values
                .harvest_amount_mult
                .matches(|| server.harvesting.harvest_amount_mult)
            && self
                .values
                .item_spoil_rate_mult
                .matches(|| server.harvesting.item_spoil_rate_mult)
            && self
                .values
                .rsrc_respawn_speed_mult
                .matches(|| server.harvesting.rsrc_respawn_speed_mult)
            && self
                .values
                .crafting_time_mult
                .matches(|| server.crafting.crafting_time_mult)
            && self
                .values
                .thrall_crafting_time_mult
                .matches(|| server.crafting.thrall_crafting_time_mult)
    }
}
