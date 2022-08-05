use std::cell::RefCell;
use std::rc::Rc;

use anyhow::Result;
use fltk::app;
use fltk::button::CheckButton;
use fltk::enums::{Align, CallbackTrigger, Event};
use fltk::frame::Frame;
use fltk::group::{Group, Tile};
use fltk::input::Input;
use fltk::misc::InputChoice;
use fltk::prelude::*;
use fltk::table::TableContext;
use fltk_table::{SmartTable, TableOpts};
use strum::IntoEnumIterator;

use crate::servers::{Filter, Mode, Region, Server, ServerList, SortCriteria, SortKey};

use super::prelude::*;
use super::{widget_auto_height, widget_col_width, CleanupFn, Handler};

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

    fn populate_servers(&mut self, all_servers: ServerList) {
        self.all_servers = all_servers;
        self.filtered_servers = self.all_servers.filtered(&self.filter);
        self.sorted_servers = self.filtered_servers.sorted(self.sort_criteria);
    }

    fn filter(&self) -> &Filter {
        &self.filter
    }

    fn change_filter(&mut self, mut mutator: impl FnMut(&mut Filter)) {
        mutator(&mut self.filter);
        self.filtered_servers = self.all_servers.filtered(&self.filter);
        self.sorted_servers = self.filtered_servers.sorted(self.sort_criteria);
    }

    fn sort_criteria(&self) -> &SortCriteria {
        &self.sort_criteria
    }

    fn set_sort_criteria(&mut self, sort_criteria: SortCriteria) {
        self.sort_criteria = sort_criteria;
        self.sorted_servers = self.filtered_servers.sorted(self.sort_criteria);
    }

    fn servers(&self) -> ServerList {
        self.sorted_servers.clone()
    }
}

pub(super) struct ServerBrowser {
    pub(super) group: Group,
    on_action: Box<dyn Handler<ServerBrowserAction>>,
    server_list: SmartTable,
    server_details: SmartTable,
    state: ServerBrowserData,
}

impl ServerBrowser {
    pub(super) fn new(
        build_id: u32,
        on_action: impl Handler<ServerBrowserAction> + 'static,
    ) -> Rc<RefCell<Self>> {
        let mut filter: Filter = Default::default();
        filter.set_build_id(build_id);
        let state = ServerBrowserData::new(
            ServerList::empty(),
            filter,
            SortCriteria {
                key: SortKey::Name,
                ascending: true,
            },
        );

        let mut group = Group::default_fill();

        let mut filter_pane = Group::default_fill();

        let label_align = Align::Right | Align::Inside;
        let name_label = Frame::default()
            .with_label("Server Name:")
            .with_align(label_align);
        let map_label = Frame::default().with_label("Map:").with_align(label_align);
        let invalid_check = CheckButton::default().with_label("Show invalid servers");
        let mode_label = Frame::default().with_label("Mode:").with_align(label_align);
        let region_label = Frame::default()
            .with_label("Region:")
            .with_align(label_align);
        let pwd_prot_check = CheckButton::default().with_label("Show password protected servers");
        let left_width = widget_col_width(&[&name_label, &mode_label]);
        let mid_width = widget_col_width(&[&mode_label, &region_label]);
        let right_width = widget_col_width(&[&invalid_check, &pwd_prot_check]);
        let height = widget_auto_height(&name_label);
        let input_width = (filter_pane.w() - left_width - mid_width - right_width - 40) / 2;

        filter_pane.set_size(filter_pane.w(), height * 2 + 10);

        let name_label = name_label.with_size(left_width, height).inside_parent(0, 0);
        let mut name_input = Input::default()
            .with_size(input_width, height)
            .right_of(&name_label, 10);
        let map_label = map_label
            .with_size(mid_width, height)
            .right_of(&name_input, 10);
        let mut map_input = Input::default()
            .with_size(input_width, height)
            .right_of(&map_label, 10);
        let mut invalid_check = invalid_check
            .with_size(right_width, height)
            .right_of(&map_input, 10);
        let mode_label = mode_label
            .with_size(left_width, height)
            .below_of(&name_label, 10);
        let mut mode_input = InputChoice::default()
            .with_size(input_width, height)
            .right_of(&mode_label, 10);
        mode_input.input().set_readonly(true);
        mode_input.input().clear_visible_focus();
        mode_input.add("All");
        for mode in Mode::iter() {
            mode_input.add(mode_name(mode));
        }
        mode_input.set_value_index(0);
        let region_label = region_label
            .with_size(mid_width, height)
            .right_of(&mode_input, 10);
        let mut region_input = InputChoice::default()
            .with_size(input_width, height)
            .right_of(&region_label, 10);
        region_input.input().set_readonly(true);
        region_input.input().clear_visible_focus();
        region_input.add("All");
        for region in Region::iter() {
            region_input.add(region_name(region));
        }
        region_input.set_value_index(0);
        let mut pwd_prot_check = pwd_prot_check
            .with_size(right_width, height)
            .right_of(&region_input, 10);

        filter_pane.end();

        let tiles = Tile::default_fill()
            .below_of(&filter_pane, 10)
            .stretch_to_parent(0, 0);

        let upper_tile = Group::default_fill()
            .inside_parent(0, 0)
            .with_size_flex(0, tiles.height() * 3 / 4);

        let mut server_list = make_server_list(state.sort_criteria());
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

        let browser = Rc::new(RefCell::new(Self {
            group,
            on_action: Box::new(on_action),
            server_list: server_list.clone(),
            server_details,
            state,
        }));

        {
            let browser = browser.clone();
            server_list.set_callback(move |_| {
                let mut browser = browser.borrow_mut();
                match app::event() {
                    Event::Released => {
                        if app::event_is_click() {
                            browser.server_list_click()
                        }
                    }
                    _ => (),
                }
            });
        }
        {
            let browser = browser.clone();
            name_input.set_trigger(CallbackTrigger::Changed);
            name_input.set_callback(move |input| {
                let mut browser = browser.borrow_mut();
                browser
                    .state
                    .change_filter(|filter| filter.set_name(input.value()));
                browser.populate_servers();
            });
        }
        {
            let browser = browser.clone();
            map_input.set_trigger(CallbackTrigger::Changed);
            map_input.set_callback(move |input| {
                let mut browser = browser.borrow_mut();
                browser
                    .state
                    .change_filter(|filter| filter.set_map(input.value()));
                browser.populate_servers();
            });
        }
        {
            let browser = browser.clone();
            mode_input.set_trigger(CallbackTrigger::Changed);
            mode_input.set_callback(move |input| {
                let mut browser = browser.borrow_mut();
                let mode = {
                    let repr = input.menu_button().value() - 1;
                    if repr < 0 {
                        None
                    } else {
                        Mode::from_repr(repr as _)
                    }
                };
                browser.state.change_filter(|filter| filter.set_mode(mode));
                browser.populate_servers();
            });
        }
        {
            let browser = browser.clone();
            region_input.set_trigger(CallbackTrigger::Changed);
            region_input.set_callback(move |input| {
                let mut browser = browser.borrow_mut();
                let region = {
                    let repr = input.menu_button().value() - 1;
                    if repr < 0 {
                        None
                    } else {
                        Region::from_repr(repr as _)
                    }
                };
                browser
                    .state
                    .change_filter(|filter| filter.set_region(region));
                browser.populate_servers();
            });
        }
        {
            let browser = browser.clone();
            invalid_check.set_trigger(CallbackTrigger::Changed);
            invalid_check.set_callback(move |input| {
                let mut browser = browser.borrow_mut();
                let build_id = if input.is_checked() { None } else { Some(build_id) };
                browser
                    .state
                    .change_filter(|filter| filter.set_build_id(build_id));
                browser.populate_servers();
            })
        }
        {
            let browser = browser.clone();
            pwd_prot_check.set_trigger(CallbackTrigger::Changed);
            pwd_prot_check.set_callback(move |input| {
                let mut browser = browser.borrow_mut();
                browser
                    .state
                    .change_filter(|filter| filter.set_password_protected(input.is_checked()));
                browser.populate_servers();
            })
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
                    self.state.populate_servers(all_servers);
                    self.populate_servers();
                }
                Err(err) => super::alert_error(ERR_LOADING_SERVERS, &err),
            },
        }
    }

    fn server_list_click(&mut self) {
        match self.server_list.callback_context() {
            TableContext::ColHeader => (),
            TableContext::Cell => return self.populate_details(self.server_list.callback_row()),
            _ => return,
        };

        let col = self.server_list.callback_col();
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

    fn populate_details(&mut self, row: i32) {
        let server = &self.state.servers()[row as _];
        self.set_detail_field(0, &server.id);
        self.set_detail_field(1, &server.name);
        self.set_detail_field(2, &format!("{}:{}", server.ip, server.port));
        self.set_detail_field(3, &server.map);
        self.set_detail_field(4, mode_name(server.mode()));
        self.set_detail_field(5, region_name(server.region));
        self.server_details.redraw();
    }

    fn set_detail_field(&mut self, idx: i32, value: &str) {
        self.server_details.set_cell_value(idx, 0, value);
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
