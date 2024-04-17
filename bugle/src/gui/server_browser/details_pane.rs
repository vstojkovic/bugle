use std::borrow::Cow;
use std::rc::Rc;

use nom::character::complete::{char, digit1};
use nom::combinator::map_res;
use nom::sequence::{separated_pair, terminated};
use nom::IResult;

use crate::game::platform::ModDirectory;
use crate::game::settings::server::DropOnDeath;
use crate::game::settings::Hours;
use crate::gui::weekday_name;
use crate::gui::widgets::{use_inspector_macros, Inspector, PropertiesTable, PropertyRow};
use crate::servers::{Server, Validity};
use crate::util::weekday_iter;

use super::{community_name, mode_name, region_name};

pub(super) struct DetailsPane {
    table: PropertiesTable<Server, InspectorCtx>,
}

struct InspectorCtx {
    mod_resolver: Rc<dyn ModDirectory>,
}

impl DetailsPane {
    pub fn new(mod_resolver: Rc<dyn ModDirectory>) -> Self {
        let ctx = InspectorCtx { mod_resolver };
        Self {
            table: PropertiesTable::new(ctx, SERVER_DETAILS_ROWS, "Server Details"),
        }
    }

    pub fn populate(&self, server: Option<&Server>) {
        self.table.populate(server);
    }
}

impl InspectorCtx {
    fn inspect_raid_hours(
        &self,
        server: Option<&Server>,
        row_consumer: &mut dyn FnMut(PropertyRow),
        include_empty: bool,
    ) {
        let mut header = "Raid Hours";
        let mut consumer_called = false;

        if let Some(server) = server {
            for weekday in weekday_iter() {
                if let Some(Hours { start, end }) = server.general.raid_hours.get(&weekday) {
                    row_consumer([
                        header.into(),
                        format!(
                            "{}: {} - {}",
                            weekday_name(weekday),
                            start.to_string(),
                            end.to_string(),
                        )
                        .into(),
                    ]);
                    header = "";
                    consumer_called = true;
                }
            }
        }

        if !consumer_called && include_empty {
            row_consumer([header.into(), "".into()]);
        }
    }

    fn inspect_mods(
        &self,
        server: Option<&Server>,
        row_consumer: &mut dyn FnMut(PropertyRow),
        include_empty: bool,
    ) {
        let mut header = "Mods";

        let mods = match server.and_then(|server| server.mods.as_ref()) {
            Some(mods) => mods,
            None => {
                if include_empty {
                    row_consumer([header.into(), "".into()]);
                }
                return;
            }
        };

        let (mut mod_ids, steam_mods, non_steam_mods) = match parse_mod_counts(mods) {
            Ok((mod_ids, (steam_mods, non_steam_mods))) => (mod_ids, steam_mods, non_steam_mods),
            Err(_) => {
                row_consumer([header.into(), "????".into()]);
                return;
            }
        };

        let mut resolution: [(u64, Option<String>); 10] = std::array::from_fn(|_| (0u64, None));
        let mut resolve_count = std::cmp::min(steam_mods, 10);
        for idx in 0..resolve_count {
            let (remaining, id) = match parse_mod_id(mod_ids) {
                Ok((remaining, id)) => (remaining, id),
                Err(_) => {
                    resolve_count = idx;
                    break;
                }
            };
            mod_ids = remaining;
            resolution[idx].0 = id;
        }

        Rc::clone(&self.mod_resolver).resolve(&mut resolution[0..resolve_count]);
        for (id, name) in &resolution[0..resolve_count] {
            match name {
                Some(name) => row_consumer([header.into(), name.clone().into()]),
                None => row_consumer([header.into(), format!("???? ({})", id).into()]),
            };
            header = "";
        }
        if steam_mods > resolve_count {
            row_consumer([
                header.into(),
                format!("+{} more Steam mods", steam_mods - resolve_count).into(),
            ]);
            header = "";
        }
        if non_steam_mods > 0 {
            row_consumer([
                header.into(),
                format!("+{} non-Steam mods", non_steam_mods).into(),
            ]);
        }
    }
}

use_inspector_macros!(Server, InspectorCtx);

const SERVER_DETAILS_ROWS: &[Inspector<Server, InspectorCtx>] = &[
    inspect_attr!("ID", |server| server.id.clone().into()),
    inspect_attr!("Server Name", |server| server.name.clone().into()),
    inspect_attr!("Host", |server| server.host().into()),
    inspect_attr!("Map Name", |server| server.map.clone().into()),
    inspect_attr!("Mode", |server| mode_name(server.mode()).into()),
    inspect_attr!("Region", |server| region_name(server.region).into()),
    inspect_attr!("Max Clan Size", |server| server
        .general
        .max_clan_size
        .to_string()
        .into()),
    inspect_attr!("On Death", |server| {
        match server.survival.drop_items_on_death {
            DropOnDeath::Nothing => "keep all items",
            DropOnDeath::All => "drop all items",
            DropOnDeath::Backpack => "drop only backpack",
        }
        .into()
    }),
    inspect_attr!("Player Corpse", |server| {
        if server.survival.anyone_can_loot_corpse {
            "can be looted by anyone"
        } else {
            "can only be looted by owner"
        }
        .into()
    }),
    inspect_attr!("Offline Characters", |server| {
        if server.survival.offline_chars_in_world {
            "stay in the world"
        } else {
            "disappear from the world"
        }
        .into()
    }),
    inspect_attr!("Harvest Amount Multiplier", |server| {
        server.harvesting.harvest_amount_mult.to_string().into()
    }),
    inspect_attr!("XP Rate Multiplier", |server| {
        server.progression.xp_rate_mult.to_string().into()
    }),
    inspect_attr!("Crafting Time Multiplier", |server| {
        server.crafting.crafting_time_mult.to_string().into()
    }),
    inspect_attr!("Thrall Crafting Time Multiplier", |server| {
        server.crafting.thrall_crafting_time_mult.to_string().into()
    }),
    InspectorCtx::inspect_raid_hours,
    inspect_attr!("Stamina Cost Multiplier", |server| {
        server.survival.stamina_cost_mult.to_string().into()
    }),
    inspect_attr!("Item Spoil Rate Scale", |server| {
        server.harvesting.item_spoil_rate_mult.to_string().into()
    }),
    inspect_attr!("Resource Respawn Speed Multiplier", |server| {
        server.harvesting.rsrc_respawn_speed_mult.to_string().into()
    }),
    inspect_attr!("Idle Thirst Multiplier", |server| {
        server.survival.idle_thirst_mult.to_string().into()
    }),
    inspect_attr!("Active Thirst Multiplier", |server| {
        server.survival.active_thirst_mult.to_string().into()
    }),
    inspect_attr!("Idle Hunger Multiplier", |server| {
        server.survival.idle_hunger_mult.to_string().into()
    }),
    inspect_attr!("Active Hunger Multiplier", |server| {
        server.survival.active_hunger_mult.to_string().into()
    }),
    inspect_attr!("Durability Multiplier", |server| {
        server.combat.durability_mult.to_string().into()
    }),
    inspect_attr!("Thrall Wakeup Time", |server| {
        format!("{} secs", server.combat.thrall_wakeup_time.num_seconds()).into()
    }),
    inspect_attr!("Day Cycle Speed", |server| {
        server.daylight.day_cycle_speed_mult.to_string().into()
    }),
    inspect_attr!("Dawn/Dusk Time Speed", |server| {
        server.daylight.dawn_dusk_speed_mult.to_string().into()
    }),
    inspect_attr!("Use Catch Up Time", |server| {
        if server.daylight.use_catch_up_time { "Yes" } else { "No" }.into()
    }),
    inspect_attr!("Community", |server| {
        community_name(server.general.community).into()
    }),
    inspect_opt_attr!("Max Ping", |server| {
        server.general.max_ping.map(|ping| ping.to_string().into())
    }),
    InspectorCtx::inspect_mods,
    inspect_opt_attr!("Problems", problems_cell_value),
];

fn parse_mod_counts(input: &str) -> IResult<&str, (usize, usize), ()> {
    terminated(
        separated_pair(
            map_res(digit1, |count: &str| count.parse()),
            char(':'),
            map_res(digit1, |count: &str| count.parse()),
        ),
        char('\n'),
    )(input)
}

fn parse_mod_id(input: &str) -> IResult<&str, u64, ()> {
    terminated(map_res(digit1, |id: &str| id.parse()), char('\n'))(input)
}

fn problems_cell_value(server: &Server) -> Option<Cow<'static, str>> {
    if server.is_valid() {
        return None;
    }
    let mut problems = String::new();
    if server.validity.contains(Validity::INVALID_BUILD) {
        problems.push_str("version mismatch, ");
    }
    if server.validity.contains(Validity::INVALID_ADDR) {
        problems.push_str("invalid IP address, ");
    }
    if server.validity.contains(Validity::INVALID_PORT) {
        problems.push_str("invalid port, ");
    }
    problems.truncate(problems.len() - 2);
    Some(problems.into())
}
