use std::borrow::Cow;

use fltk::prelude::*;
use nom::character::complete::{char, digit1};
use nom::sequence::separated_pair;
use nom::IResult;
use strum::IntoEnumIterator;

use crate::gui::widgets::{
    make_readonly_cell_widget, DataTable, DataTableProperties, DataTableUpdate, ReadOnlyText,
};
use crate::servers::{DropOnDeath, Server, Validity, Weekday};

use super::{community_name, mode_name, region_name, weekday_name};

type DetailRow = [Cow<'static, str>; 2];

pub(super) struct DetailsPane {
    table: DataTable<DetailRow>,
    cell: ReadOnlyText,
}

impl DetailsPane {
    pub fn new() -> Self {
        let table_props = DataTableProperties {
            columns: vec!["Server Details".into()],
            cell_selection_color: fltk::enums::Color::Free,
            header_font_color: fltk::enums::Color::Gray0,
            ..Default::default()
        };
        let width_padding = table_props.cell_padding * 2 + fltk::app::scrollbar_size();

        let mut table = DataTable::<[Cow<'static, str>; 2]>::default().with_properties(table_props);
        table.set_row_header(true);
        table.set_col_header(true);
        table.set_col_resize(true);

        let mut header_width = 0i32;
        fltk::draw::set_font(table.label_font(), table.label_size());
        let mut consumer = |row: DetailRow| {
            let (w, _) = fltk::draw::measure(row[0].as_ref(), true);
            header_width = std::cmp::max(header_width, w);
        };
        for inspector in SERVER_DETAILS_ROWS.iter() {
            inspector(None, &mut consumer, true);
        }
        header_width += width_padding;
        table.set_row_header_width(header_width);

        let w = table.w();
        table.set_col_width(0, w - header_width - width_padding);

        table.end();

        let cell = make_readonly_cell_widget(&table);

        let pane = Self { table, cell };
        pane.populate(None);

        pane
    }

    pub fn populate(&self, server: Option<&Server>) {
        self.cell.clone().hide();
        {
            let data = self.table.data();
            let mut data = data.borrow_mut();
            data.clear();
            let mut consumer = |row| data.push(row);
            for inspector in SERVER_DETAILS_ROWS.iter() {
                inspector(server, &mut consumer, false);
            }
        }
        self.table.updated(DataTableUpdate::DATA);
    }
}

type Inspector = fn(Option<&Server>, &mut dyn FnMut(DetailRow), bool);

macro_rules! inspect_attr {
    ($header:literal, $lambda:expr) => {
        |server: Option<&Server>, row_consumer: &mut dyn FnMut(DetailRow), _include_empty: bool| {
            row_consumer([$header.into(), server.map($lambda).unwrap_or_default()]);
        }
    };
}

macro_rules! inspect_opt_attr {
    ($header:literal, $lambda:expr) => {
        |server: Option<&Server>, row_consumer: &mut dyn FnMut(DetailRow), include_empty: bool| {
            let detail = server.and_then($lambda);
            if detail.is_some() || include_empty {
                row_consumer([$header.into(), detail.unwrap_or_default()]);
            }
        }
    };
}

const SERVER_DETAILS_ROWS: &[Inspector] = &[
    inspect_attr!("ID", |server| server.id.clone().into()),
    inspect_attr!("Server Name", |server| server.name.clone().into()),
    inspect_attr!("Host", |server| server.host().into()),
    inspect_attr!("Map Name", |server| server.map.clone().into()),
    inspect_attr!("Mode", |server| mode_name(server.mode()).into()),
    inspect_attr!("Region", |server| region_name(server.region).into()),
    inspect_attr!("Max Clan Size", |server| {
        server
            .max_clan_size
            .map(|size| size.to_string())
            .unwrap_or_default()
            .into()
    }),
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
        server.xp_rate_mult.to_string().into()
    }),
    inspect_attr!("Crafting Time Multiplier", |server| {
        server.crafting.crafting_time_mult.to_string().into()
    }),
    inspect_attr!("Thrall Crafting Time Multiplier", |server| {
        server.crafting.thrall_crafting_time_mult.to_string().into()
    }),
    raid_hours_inspector,
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
        format!("{} secs", server.combat.thrall_wakeup_time_secs()).into()
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
        community_name(server.community).into()
    }),
    inspect_attr!("Max Ping", |server| {
        server
            .max_ping
            .map(|ping| ping.to_string())
            .unwrap_or_default()
            .into()
    }),
    inspect_opt_attr!("Mods", mods_cell_value),
    inspect_opt_attr!("Problems", problems_cell_value),
];

fn raid_hours_inspector(
    server: Option<&Server>,
    row_consumer: &mut dyn FnMut(DetailRow),
    include_empty: bool,
) {
    let mut header = "Raid Hours";
    let mut consumer_called = false;

    if let Some(server) = server {
        for weekday in Weekday::iter() {
            if let Some((start, end)) = server.raid_hours.get(weekday) {
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

fn mods_cell_value(server: &Server) -> Option<Cow<'static, str>> {
    let mods = server.mods.as_ref()?;
    if let Ok((_, (steam_mods, non_steam_mods))) = parse_mod_counts(mods) {
        Some(format!("Steam: {}, Non-Steam: {}", steam_mods, non_steam_mods).into())
    } else {
        Some("????".into())
    }
}

fn parse_mod_counts(input: &str) -> IResult<&str, (&str, &str), ()> {
    separated_pair(digit1, char(':'), digit1)(input)
}

fn problems_cell_value(server: &Server) -> Option<Cow<'static, str>> {
    if server.is_valid() {
        return None;
    }
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
    Some(problems.join(", ").into())
}
