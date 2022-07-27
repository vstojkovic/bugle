use std::rc::Rc;

use anyhow::Result;
use fltk::group::Group;
use fltk::{prelude::*};
use fltk_table::{SmartTable, TableOpts};

use crate::servers::{Server, Region};

use super::{Action, ActionHandler, CleanupFn};

pub enum ServerBrowserAction {
    LoadServers,
}

pub enum ServerBrowserUpdate {
    PopulateServers(Result<Vec<Server>>),
}

pub(super) struct ServerBrowser {
    pub(super) group: Group,
    on_action: Rc<dyn ActionHandler>,
    server_list: SmartTable,
}

impl ServerBrowser {
    pub(super) fn new(on_action: Rc<dyn ActionHandler>) -> Self {
        let mut group = Group::default_fill();

        let server_list = make_table(&[
            (GLYPH_LOCK, 20),
            ("Server Name", 420),
            ("Map", 160),
            ("Mode", 80),
            ("Region", 80),
            ("Players", 60),
            ("Age", 60),
            ("Ping", 60),
            ("BattlEye", 60),
            ("Level", 50),
        ]);

        group.end();
        group.hide();

        Self { group, on_action, server_list }
    }

    pub(super) fn show(&mut self) -> CleanupFn {
        self.group.show();

        if let Err(err) = self.action(ServerBrowserAction::LoadServers) {
            super::alert_error(ERR_LOADING_SERVERS, &err);
        }

        let mut group = self.group.clone();
        Box::new(move || {
            group.hide();
        })
    }

    pub(super) fn handle_update(&mut self, update: ServerBrowserUpdate) {
        match update {
            ServerBrowserUpdate::PopulateServers(servers) => {
                match servers {
                    Ok(server_list) => self.populate_servers(server_list),
                    Err(err) => super::alert_error(ERR_LOADING_SERVERS, &err),
                }
            }
        }
    }

    fn action(&self, action: ServerBrowserAction) -> anyhow::Result<()> {
        (self.on_action)(Action::ServerBrowser(action))
    }

    fn populate_servers(&mut self, server_list: Vec<Server>) {
        let row_count = server_list.len();
        {
            let data_ref = self.server_list.data_ref();
            *data_ref.lock().unwrap() = server_list.into_iter().map(make_server_row).collect();
        }
        self.server_list.set_rows(row_count as _);
    }
}

const ERR_LOADING_SERVERS: &str = "Error while loading the server list.";
const GLYPH_LOCK: &str = "\u{1f512}";
const GLYPH_YES: &str = "\u{2714}";
const GLYPH_NO: &str = "\u{2716}";

fn make_table(cols: &[(&str, i32)]) -> SmartTable {
    let mut table = SmartTable::default_fill().with_opts(TableOpts {
        rows: 0,
        cols: cols.len() as _,
        editable: false,
        ..Default::default()
    });
    table.set_row_header(false);
    table.set_col_resize(true);

    for (idx, (header, width)) in cols.iter().enumerate() {
        let idx = idx as _;
        table.set_col_header_value(idx, header);
        table.set_col_width(idx, *width);
    }

    table
}

fn make_server_row(server: Server) -> Vec<String> {
    let mode_name = mode_name(&server).to_string();
    vec![
        (if server.password_protected { GLYPH_LOCK } else { "" }).to_string(),
        server.name.unwrap_or("".to_string()),
        server.map,
        mode_name,
        region_name(&server.region).to_string(),
        format!("??/{}", server.max_players), // TODO: Current players
        "????".to_string(), // TODO: Age
        "????".to_string(), // TODO: Ping
        (if server.battleye_required { GLYPH_YES } else { GLYPH_NO }).to_string(),
        "??".to_string(), // TODO: Level
    ]
}

fn mode_name(server: &Server) -> &str {
    match server.pvp_enabled {
        false => "PVE",
        true => match server.is_conflict() {
            false => "PVP",
            true => "PVE-C",
        }
    }
}

fn region_name(region: &Region) -> &str {
    match region {
        Region::EU => "EU",
        Region::America => "America",
        Region::Asia => "Asia",
        Region::Oceania => "Oceania",
        Region::LATAM => "LATAM",
        Region::Japan => "Japan",
    }
}
