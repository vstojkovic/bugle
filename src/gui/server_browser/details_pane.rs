use std::borrow::Cow;

use fltk::prelude::*;
use fltk_table::{SmartTable, TableOpts};
use nom::character::complete::{char, digit1};
use nom::sequence::separated_pair;
use nom::IResult;

use crate::gui::{make_readonly_cell_widget, ReadOnlyText};
use crate::servers::{Server, Validity, Weekday};

use super::{community_name, mode_name, region_name};

pub(super) struct DetailsPane {
    table: SmartTable,
    cell: ReadOnlyText,
}

impl DetailsPane {
    pub fn new() -> Self {
        let mut table = SmartTable::default_fill().with_opts(TableOpts {
            rows: SERVER_DETAILS_ROWS.len() as _,
            cols: 1,
            editable: false,
            ..Default::default()
        });
        table.set_col_resize(true);

        let mut header_width = 0i32;
        fltk::draw::set_font(table.label_font(), table.label_size());
        for (idx, (header, _)) in SERVER_DETAILS_ROWS.iter().enumerate() {
            let idx = idx as _;
            table.set_row_header_value(idx, header);
            let (w, _) = fltk::draw::measure(header, true);
            header_width = std::cmp::max(header_width, w);
        }
        header_width += 20;
        table.set_row_header_width(header_width);

        let w = table.w();
        table.set_col_header_value(0, "Server Details");
        table.set_col_width(0, w - header_width - 20);

        table.end();

        let cell = make_readonly_cell_widget(&table);

        Self { table, cell }
    }

    pub fn populate(&self, server: Option<&Server>) {
        self.cell.clone().hide();
        let mut table = self.table.clone();
        if let Some(server) = server {
            for (idx, (_, cell_value)) in SERVER_DETAILS_ROWS.iter().enumerate() {
                table.set_cell_value(idx as _, 0, cell_value(server).as_ref());
            }
        } else {
            table.clear();
        }
        table.redraw();
    }
}

const SERVER_DETAILS_ROWS: &[(&str, fn(&Server) -> Cow<str>)] = &[
    ("ID", |server| Cow::from(&server.id)),
    ("Server Name", |server| Cow::from(&server.name)),
    ("Host", |server| Cow::from(server.host())),
    ("Map Name", |server| Cow::from(&server.map)),
    ("Mode", |server| Cow::from(mode_name(server.mode()))),
    ("Region", |server| Cow::from(region_name(server.region))),
    ("Max Clan Size", |server| {
        Cow::from(
            server
                .max_clan_size
                .map(|size| size.to_string())
                .unwrap_or_default(),
        )
    }),
    ("On Death", |server| {
        Cow::from(if server.survival.drop_items_on_death { "drop items" } else { "keep items" })
    }),
    ("Player Corpse", |server| {
        Cow::from(if server.survival.anyone_can_loot_corpse {
            "can be looted by anyone"
        } else {
            "can only be looted by owner"
        })
    }),
    ("Offline Characters", |server| {
        Cow::from(if server.survival.offline_chars_in_world {
            "stay in the world"
        } else {
            "disappear from the world"
        })
    }),
    ("Harvest Amount Multiplier", |server| {
        Cow::from(server.harvesting.harvest_amount_mult.to_string())
    }),
    ("XP Rate Multiplier", |server| {
        Cow::from(server.xp_rate_mult.to_string())
    }),
    ("Crafting Time Multiplier", |server| {
        Cow::from(server.crafting.crafting_time_mult.to_string())
    }),
    ("Thrall Crafting Time Multiplier", |server| {
        Cow::from(server.crafting.thrall_crafting_time_mult.to_string())
    }),
    ("Raid Hours (Mon)", |server| {
        Cow::from(
            server
                .raid_hours
                .get(Weekday::Mon)
                .map(|(start, end)| format!("{} - {}", start.to_string(), end.to_string()))
                .unwrap_or_default(),
        )
    }),
    ("Raid Hours (Tue)", |server| {
        Cow::from(
            server
                .raid_hours
                .get(Weekday::Tue)
                .map(|(start, end)| format!("{} - {}", start.to_string(), end.to_string()))
                .unwrap_or_default(),
        )
    }),
    ("Raid Hours (Wed)", |server| {
        Cow::from(
            server
                .raid_hours
                .get(Weekday::Wed)
                .map(|(start, end)| format!("{} - {}", start.to_string(), end.to_string()))
                .unwrap_or_default(),
        )
    }),
    ("Raid Hours (Thu)", |server| {
        Cow::from(
            server
                .raid_hours
                .get(Weekday::Thu)
                .map(|(start, end)| format!("{} - {}", start.to_string(), end.to_string()))
                .unwrap_or_default(),
        )
    }),
    ("Raid Hours (Fri)", |server| {
        Cow::from(
            server
                .raid_hours
                .get(Weekday::Fri)
                .map(|(start, end)| format!("{} - {}", start.to_string(), end.to_string()))
                .unwrap_or_default(),
        )
    }),
    ("Raid Hours (Sat)", |server| {
        Cow::from(
            server
                .raid_hours
                .get(Weekday::Sat)
                .map(|(start, end)| format!("{} - {}", start.to_string(), end.to_string()))
                .unwrap_or_default(),
        )
    }),
    ("Raid Hours (Sun)", |server| {
        Cow::from(
            server
                .raid_hours
                .get(Weekday::Sun)
                .map(|(start, end)| format!("{} - {}", start.to_string(), end.to_string()))
                .unwrap_or_default(),
        )
    }),
    ("Stamina Cost Multiplier", |server| {
        Cow::from(server.survival.stamina_cost_mult.to_string())
    }),
    ("Item Spoil Rate Scale", |server| {
        Cow::from(server.harvesting.item_spoil_rate_mult.to_string())
    }),
    ("Resource Respawn Speed Multiplier", |server| {
        Cow::from(server.harvesting.rsrc_respawn_speed_mult.to_string())
    }),
    ("Idle Thirst Multiplier", |server| {
        Cow::from(server.survival.idle_thirst_mult.to_string())
    }),
    ("Active Thirst Multiplier", |server| {
        Cow::from(server.survival.active_thirst_mult.to_string())
    }),
    ("Idle Hunger Multiplier", |server| {
        Cow::from(server.survival.idle_hunger_mult.to_string())
    }),
    ("Active Hunger Multiplier", |server| {
        Cow::from(server.survival.active_hunger_mult.to_string())
    }),
    ("Durability Multiplier", |server| {
        Cow::from(server.combat.durability_mult.to_string())
    }),
    ("Thrall Wakeup Time", |server| {
        Cow::from(format!("{} secs", server.combat.thrall_wakeup_time_secs()))
    }),
    ("Day Cycle Speed", |server| {
        Cow::from(server.daylight.day_cycle_speed_mult.to_string())
    }),
    ("Dawn/Dusk Time Speed", |server| {
        Cow::from(server.daylight.dawn_dusk_speed_mult.to_string())
    }),
    ("Use Catch Up Time", |server| {
        Cow::from(if server.daylight.use_catch_up_time { "Yes" } else { "No" })
    }),
    ("Community", |server| {
        Cow::from(community_name(server.community))
    }),
    ("Max Ping", |server| {
        Cow::from(
            server
                .max_ping
                .map(|ping| ping.to_string())
                .unwrap_or_default(),
        )
    }),
    ("Mods", mods_cell_value),
    ("Problems", problems_cell_value),
];

fn mods_cell_value(server: &Server) -> Cow<str> {
    if let Some(mods) = &server.mods {
        if let Ok((_, (steam_mods, non_steam_mods))) = parse_mod_counts(mods) {
            format!("Steam: {}, Non-Steam: {}", steam_mods, non_steam_mods).into()
        } else {
            "????".into()
        }
    } else {
        "".into()
    }
}

fn parse_mod_counts(input: &str) -> IResult<&str, (&str, &str), ()> {
    separated_pair(digit1, char(':'), digit1)(input)
}

fn problems_cell_value(server: &Server) -> Cow<str> {
    if server.is_valid() {
        "".into()
    } else {
        let mut problems = Vec::new();
        if server.validity.contains(Validity::INVALID_BUILD) {
            problems.push("version mismatch");
        }
        if server.validity.contains(Validity::INVALID_ADDR) {
            problems.push("invalid IP address");
        }
        if server.validity.contains(Validity::INVALID_PORT) {
            problems.push("version port");
        }
        problems.join(", ").into()
    }
}
