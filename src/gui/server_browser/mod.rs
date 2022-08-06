use std::cell::RefCell;
use std::rc::Rc;

use anyhow::Result;
use fltk::app;
use fltk::enums::Event;
use fltk::group::{Group, Tile};
use fltk::prelude::*;
use fltk::table::TableContext;
use fltk_table::{SmartTable, TableOpts};

use crate::servers::{Filter, Mode, Region, Server, ServerList, SortCriteria, SortKey};

use self::filter_pane::{FilterHolder, FilterPane};

use super::prelude::*;
use super::{CleanupFn, Handler};

mod filter_pane;

pub enum ServerBrowserAction {
    LoadServers,
}

pub enum ServerBrowserUpdate {
    PopulateServers(Result<ServerList>),
}

struct ServerBrowserData {
    all_servers: ServerList,
    filter: Filter,
    filtered_servers: ServerList,
    sort_criteria: SortCriteria,
    sorted_servers: ServerList,
}

impl ServerBrowserData {
    fn new(all_servers: ServerList, filter: Filter, sort_criteria: SortCriteria) -> Self {
        let filtered_servers = all_servers.filtered(&filter);
        let sorted_servers = filtered_servers.sorted(sort_criteria);
        Self {
            all_servers,
            filter,
            filtered_servers,
            sort_criteria,
            sorted_servers,
        }
    }

    fn set_servers(&mut self, all_servers: ServerList) {
        self.all_servers = all_servers;
        self.update_filtered_servers();
    }

    fn filter(&self) -> &Filter {
        &self.filter
    }

    fn change_filter(&mut self, mut mutator: impl FnMut(&mut Filter)) {
        mutator(&mut self.filter);
        self.update_filtered_servers();
    }

    fn sort_criteria(&self) -> &SortCriteria {
        &self.sort_criteria
    }

    fn set_sort_criteria(&mut self, sort_criteria: SortCriteria) {
        self.sort_criteria = sort_criteria;
        self.update_sorted_servers();
    }

    fn servers(&self) -> ServerList {
        self.sorted_servers.clone()
    }

    fn update_filtered_servers(&mut self) {
        self.filtered_servers = self.all_servers.filtered(&self.filter);
        self.update_sorted_servers()
    }

    fn update_sorted_servers(&mut self) {
        self.sorted_servers = self.filtered_servers.sorted(self.sort_criteria);
    }
}

pub(super) struct ServerBrowser {
    pub(super) group: Group,
    on_action: Box<dyn Handler<ServerBrowserAction>>,
    server_list: SmartTable,
    server_details: SmartTable,
    state: Rc<RefCell<ServerBrowserData>>,
}

impl ServerBrowser {
    pub(super) fn new(
        build_id: u32,
        on_action: impl Handler<ServerBrowserAction> + 'static,
    ) -> Rc<Self> {
        let mut filter: Filter = Default::default();
        filter.set_build_id(build_id);
        let state = Rc::new(RefCell::new(ServerBrowserData::new(
            ServerList::empty(),
            filter,
            SortCriteria {
                key: SortKey::Name,
                ascending: true,
            },
        )));

        let mut group = Group::default_fill();

        let filter_pane = FilterPane::new(build_id);

        let tiles = Tile::default_fill()
            .below_of(filter_pane.root(), 10)
            .stretch_to_parent(0, 0);

        let upper_tile = Group::default_fill()
            .inside_parent(0, 0)
            .with_size_flex(0, tiles.height() * 3 / 4);

        let mut server_list = make_server_list(state.borrow().sort_criteria());
        server_list.end();

        upper_tile.end();

        let lower_tile = Group::default_fill()
            .below_of(&upper_tile, 0)
            .stretch_to_parent(0, 0);

        let server_details = make_server_details();
        server_details.end();

        lower_tile.end();

        tiles.end();

        group.end();
        group.hide();

        let browser = Rc::new(Self {
            group,
            on_action: Box::new(on_action),
            server_list: server_list.clone(),
            server_details,
            state: state.clone(),
        });

        filter_pane.set_filter_holder(browser.clone());
        {
            let browser = browser.clone();
            server_list.set_callback(move |_| match app::event() {
                Event::Released => {
                    if app::event_is_click() {
                        browser.server_list_click()
                    }
                }
                _ => (),
            });
        }

        browser
    }

    pub(super) fn show(&self) -> CleanupFn {
        self.group.clone().show();

        (self.on_action)(ServerBrowserAction::LoadServers).unwrap();

        let mut group = self.group.clone();
        Box::new(move || {
            group.hide();
        })
    }

    pub(super) fn handle_update(&self, update: ServerBrowserUpdate) {
        match update {
            ServerBrowserUpdate::PopulateServers(payload) => match payload {
                Ok(all_servers) => {
                    self.state.borrow_mut().set_servers(all_servers);
                    self.populate_servers();
                }
                Err(err) => super::alert_error(ERR_LOADING_SERVERS, &err),
            },
        }
    }

    fn server_list_click(&self) {
        match self.server_list.callback_context() {
            TableContext::ColHeader => self.server_list_header_click(),
            TableContext::Cell => self.populate_details(self.server_list.callback_row()),
            _ => (),
        }
    }

    fn server_list_header_click(&self) {
        let mut table = self.server_list.clone();
        let col = table.callback_col();
        let new_key = match column_to_sort_key(col) {
            Some(key) => key,
            None => return,
        };
        let old_criteria = *self.state.borrow().sort_criteria();
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
            table.set_col_header_value(old_col, &sortable_column_header(old_col, None));
        }
        self.state.borrow_mut().set_sort_criteria(new_criteria);
        table.set_col_header_value(
            col,
            &sortable_column_header(col, Some(new_criteria.ascending)),
        );
        self.populate_servers();
    }

    fn populate_servers(&self) {
        let server_list = self.state.borrow().servers().clone();
        let mut table = self.server_list.clone();
        let row_count = server_list.len();
        {
            let data_ref = table.data_ref();
            *data_ref.lock().unwrap() = server_list.into_iter().map(make_server_row).collect();
        }
        table.set_rows(row_count as _);
        table.redraw();
    }

    fn populate_details(&self, row: i32) {
        let server = &self.state.borrow().servers()[row as _];
        let mut table = self.server_details.clone();
        table.set_cell_value(0, 0, &server.id);
        table.set_cell_value(1, 0, &server.name);
        table.set_cell_value(2, 0, &format!("{}:{}", server.ip, server.port));
        table.set_cell_value(3, 0, &server.map);
        table.set_cell_value(4, 0, mode_name(server.mode()));
        table.set_cell_value(5, 0, region_name(server.region));
        table.redraw();
    }
}

impl FilterHolder for Rc<ServerBrowser> {
    fn access_filter(&self, accessor: impl FnOnce(&Filter)) {
        accessor(self.state.borrow().filter());
    }

    fn mutate_filter(&self, mutator: impl FnMut(&mut Filter)) {
        self.state.borrow_mut().change_filter(mutator);
        self.populate_servers();
    }
}

const ERR_LOADING_SERVERS: &str = "Error while loading the server list.";
const GLYPH_LOCK: &str = "\u{1f512}";
const GLYPH_YES: &str = "\u{2714}";
const GLYPH_NO: &str = "\u{2716}";
const GLYPH_UNSORTED: &str = "\u{25bd}";
const GLYPH_ASC: &str = "\u{25b2}";
const GLYPH_DESC: &str = "\u{25bc}";

const SERVER_LIST_COLS: &[(&str, i32)] = &[
    (GLYPH_LOCK, 20),
    ("Server Name", 400),
    ("Map", 160),
    ("Mode", 80),
    ("Region", 80),
    ("Players", 60),
    ("Age", 60),
    ("Ping", 60),
    ("BattlEye", 60),
    ("Level", 50),
];

const SERVER_DETAILS_ROWS: &[&str] = &["ID", "Server Name", "Host", "Map Name", "Mode", "Region"];

fn make_server_list(initial_sort: &SortCriteria) -> SmartTable {
    let mut table = SmartTable::default_fill().with_opts(TableOpts {
        rows: 0,
        cols: SERVER_LIST_COLS.len() as _,
        editable: false,
        ..Default::default()
    });
    table.make_resizable(true);
    table.set_row_header(false);
    table.set_col_resize(true);

    let sorted_col = sort_key_to_column(initial_sort.key);

    for (idx, (header, width)) in SERVER_LIST_COLS.iter().enumerate() {
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

fn make_server_details() -> SmartTable {
    let mut table = SmartTable::default_fill().with_opts(TableOpts {
        rows: SERVER_DETAILS_ROWS.len() as _,
        cols: 1,
        editable: false,
        ..Default::default()
    });
    table.set_col_resize(true);

    let mut header_width = 0i32;
    fltk::draw::set_font(table.label_font(), table.label_size());
    for (idx, header) in SERVER_DETAILS_ROWS.iter().enumerate() {
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
    table
}

fn make_server_row(server: &Server) -> Vec<String> {
    vec![
        (if server.password_protected { GLYPH_LOCK } else { "" }).to_string(),
        server.name.clone(),
        server.map.clone(),
        mode_name(server.mode()).to_string(),
        region_name(server.region).to_string(),
        format!("??/{}", server.max_players), // TODO: Current players
        "????".to_string(),                   // TODO: Age
        "????".to_string(),                   // TODO: Ping
        (if server.battleye_required { GLYPH_YES } else { GLYPH_NO }).to_string(),
        "??".to_string(), // TODO: Level
    ]
}

fn mode_name(mode: Mode) -> &'static str {
    match mode {
        Mode::PVE => "PVE",
        Mode::PVEC => "PVE-C",
        Mode::PVP => "PVP",
    }
}

fn region_name(region: Region) -> &'static str {
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
        SERVER_LIST_COLS[col as usize].0,
        match ascending {
            None => GLYPH_UNSORTED,
            Some(false) => GLYPH_DESC,
            Some(true) => GLYPH_ASC,
        }
    )
}
