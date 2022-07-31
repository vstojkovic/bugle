use std::cell::RefCell;
use std::rc::Rc;

use anyhow::Result;
use fltk::app;
use fltk::enums::Event;
use fltk::group::Group;
use fltk::prelude::*;
use fltk::table::TableContext;
use fltk_table::{SmartTable, TableOpts};

use crate::servers::{Mode, Region, Server, ServerList, SortCriteria, SortKey};

use super::{CleanupFn, Handler};

pub enum ServerBrowserAction {
    LoadServers,
}

pub enum ServerBrowserUpdate {
    PopulateServers(Result<ServerList>),
}

struct ServerBrowserData {
    all_servers: ServerList,
    sort_criteria: SortCriteria,
    sorted_servers: ServerList,
}

impl ServerBrowserData {
    fn new(all_servers: ServerList, sort_criteria: SortCriteria) -> Self {
        let sorted_servers = all_servers.sorted(sort_criteria);
        Self {
            all_servers,
            sort_criteria,
            sorted_servers,
        }
    }

    fn sort_criteria(&self) -> &SortCriteria {
        &self.sort_criteria
    }

    fn set_sort_criteria(&mut self, sort_criteria: SortCriteria) {
        self.sort_criteria = sort_criteria;
        self.sorted_servers = self.all_servers.sorted(self.sort_criteria);
    }

    fn servers(&self) -> ServerList {
        self.sorted_servers.clone()
    }
}

pub(super) struct ServerBrowser {
    pub(super) group: Group,
    on_action: Box<dyn Handler<ServerBrowserAction>>,
    server_list: SmartTable,
    state: ServerBrowserData,
    col_down: i32,
}

impl ServerBrowser {
    pub(super) fn new(on_action: impl Handler<ServerBrowserAction> + 'static) -> Rc<RefCell<Self>> {
        let state = ServerBrowserData::new(
            ServerList::empty(),
            SortCriteria {
                key: SortKey::Name,
                ascending: true,
            },
        );

        let mut group = Group::default_fill();

        let mut server_list = make_table(TABLE_COLUMNS, state.sort_criteria());

        group.end();
        group.hide();

        let browser = Rc::new(RefCell::new(Self {
            group,
            on_action: Box::new(on_action),
            server_list: server_list.clone(),
            state,
            col_down: -1,
        }));

        {
            let browser = browser.clone();
            server_list.set_callback(move |_| {
                let mut browser = browser.borrow_mut();
                match app::event() {
                    Event::Push => browser.server_list_push(),
                    Event::Released => browser.server_list_release(),
                    _ => (),
                }
            });
        }

        browser
    }

    pub(super) fn show(&mut self) -> CleanupFn {
        self.group.show();

        (self.on_action)(ServerBrowserAction::LoadServers).unwrap();

        let mut group = self.group.clone();
        Box::new(move || {
            group.hide();
        })
    }

    pub(super) fn handle_update(&mut self, update: ServerBrowserUpdate) {
        match update {
            ServerBrowserUpdate::PopulateServers(payload) => match payload {
                Ok(all_servers) => {
                    self.state = ServerBrowserData::new(all_servers, *self.state.sort_criteria());
                    self.populate_servers();
                }
                Err(err) => super::alert_error(ERR_LOADING_SERVERS, &err),
            },
        }
    }

    fn server_list_push(&mut self) {
        self.col_down = if self.server_list.callback_context() == TableContext::ColHeader {
            self.server_list.callback_col()
        } else {
            -1
        };
    }

    fn server_list_release(&mut self) {
        if self.server_list.callback_context() != TableContext::ColHeader {
            return;
        }

        let col = self.server_list.callback_col();
        if col == self.col_down {
            if let Some(new_key) = column_to_sort_key(col) {
                let old_criteria = *self.state.sort_criteria();
                let new_criteria = if new_key == old_criteria.key {
                    old_criteria.reversed()
                } else {
                    SortCriteria {
                        key: new_key,
                        ascending: true,
                    }
                };
                if old_criteria.key != new_criteria.key {
                    let old_col = sort_key_to_column(old_criteria.key);
                    self.server_list
                        .set_col_header_value(old_col, &sortable_column_header(old_col, None));
                }
                self.state.set_sort_criteria(new_criteria);
                self.server_list.set_col_header_value(
                    col,
                    &sortable_column_header(col, Some(new_criteria.ascending)),
                );
                self.populate_servers();
            }
        }
    }

    fn populate_servers(&mut self) {
        let server_list = self.state.servers().clone();
        let row_count = server_list.len();
        {
            let data_ref = self.server_list.data_ref();
            *data_ref.lock().unwrap() = server_list.into_iter().map(make_server_row).collect();
        }
        self.server_list.set_rows(row_count as _);
        self.server_list.redraw();
    }
}

const ERR_LOADING_SERVERS: &str = "Error while loading the server list.";
const GLYPH_LOCK: &str = "\u{1f512}";
const GLYPH_YES: &str = "\u{2714}";
const GLYPH_NO: &str = "\u{2716}";
const GLYPH_UNSORTED: &str = "\u{25bd}";
const GLYPH_ASC: &str = "\u{25b2}";
const GLYPH_DESC: &str = "\u{25bc}";

const TABLE_COLUMNS: &[(&str, i32)] = &[
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
];

fn make_table(cols: &[(&str, i32)], initial_sort: &SortCriteria) -> SmartTable {
    let mut table = SmartTable::default_fill().with_opts(TableOpts {
        rows: 0,
        cols: cols.len() as _,
        editable: false,
        ..Default::default()
    });
    table.set_row_header(false);
    table.set_col_resize(true);

    let sorted_col = sort_key_to_column(initial_sort.key);

    for (idx, (header, width)) in cols.iter().enumerate() {
        let idx = idx as _;
        if column_to_sort_key(idx).is_some() {
            table.set_col_header_value(
                idx,
                &sortable_column_header(
                    idx,
                    if idx == sorted_col { Some(initial_sort.ascending) } else { None },
                ),
            )
        } else {
            table.set_col_header_value(idx, header);
        }
        table.set_col_width(idx, *width);
    }

    table
}

fn make_server_row(server: &Server) -> Vec<String> {
    let mode_name = mode_name(&server).to_string();
    vec![
        (if server.password_protected { GLYPH_LOCK } else { "" }).to_string(),
        server
            .name
            .as_ref()
            .map(String::clone)
            .unwrap_or("".to_string()),
        server.map.to_string(),
        mode_name,
        region_name(&server.region).to_string(),
        format!("??/{}", server.max_players), // TODO: Current players
        "????".to_string(),                   // TODO: Age
        "????".to_string(),                   // TODO: Ping
        (if server.battleye_required { GLYPH_YES } else { GLYPH_NO }).to_string(),
        "??".to_string(), // TODO: Level
    ]
}

fn mode_name(server: &Server) -> &str {
    match server.mode() {
        Mode::PVE => "PVE",
        Mode::PVEC => "PVE-C",
        Mode::PVP => "PVP",
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

fn sort_key_to_column(sort_key: SortKey) -> i32 {
    match sort_key {
        SortKey::Name => 1,
        SortKey::Map => 2,
        SortKey::Mode => 3,
        SortKey::Region => 4,
    }
}

fn column_to_sort_key(col: i32) -> Option<SortKey> {
    match col {
        1 => Some(SortKey::Name),
        2 => Some(SortKey::Map),
        3 => Some(SortKey::Mode),
        4 => Some(SortKey::Region),
        _ => None,
    }
}

fn sortable_column_header(col: i32, ascending: Option<bool>) -> String {
    format!(
        "{} {}",
        TABLE_COLUMNS[col as usize].0,
        match ascending {
            None => GLYPH_UNSORTED,
            Some(false) => GLYPH_DESC,
            Some(true) => GLYPH_ASC,
        }
    )
}
