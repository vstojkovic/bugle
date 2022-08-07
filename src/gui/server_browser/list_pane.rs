use std::cell::RefCell;
use std::rc::Rc;

use fltk::app;
use fltk::enums::Event;
use fltk::prelude::*;
use fltk::table::TableContext;
use fltk_table::{SmartTable, TableOpts};

use crate::servers::{Server, ServerList, SortCriteria, SortKey};

use super::{mode_name, region_name};

pub(super) struct ListPane {
    table: SmartTable,
    sort_criteria: RefCell<SortCriteria>,
    server_list: RefCell<ServerList>,
    on_sort_changed: RefCell<Box<dyn Fn(SortCriteria)>>,
    on_server_selected: RefCell<Box<dyn Fn(&Server)>>,
}

impl ListPane {
    pub fn new(initial_sort: &SortCriteria) -> Rc<Self> {
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

        table.end();

        let list_pane = Rc::new(Self {
            table: table.clone(),
            sort_criteria: RefCell::new(*initial_sort),
            server_list: RefCell::new(ServerList::empty()),
            on_sort_changed: RefCell::new(Box::new(|_| ())),
            on_server_selected: RefCell::new(Box::new(|_| ())),
        });

        {
            let list_pane = list_pane.clone(); // TODO: Make weak
            table.set_callback(move |_| match app::event() {
                Event::Released => {
                    if app::event_is_click() {
                        list_pane.clicked()
                    }
                }
                _ => (),
            });
        }

        list_pane
    }

    pub fn populate(&self, server_list: ServerList) {
        let mut table = self.table.clone();
        let row_count = server_list.len();
        {
            let data_ref = table.data_ref();
            *data_ref.lock().unwrap() = server_list.into_iter().map(make_server_row).collect();
        }
        table.set_rows(row_count as _);
        table.redraw();
        *self.server_list.borrow_mut() = server_list;
    }

    pub fn set_on_sort_changed(&self, on_sort_changed: impl Fn(SortCriteria) + 'static) {
        *self.on_sort_changed.borrow_mut() = Box::new(on_sort_changed);
    }

    pub fn set_on_server_selected(&self, on_server_selected: impl Fn(&Server) + 'static) {
        *self.on_server_selected.borrow_mut() = Box::new(on_server_selected);
    }

    fn clicked(&self) {
        match self.table.callback_context() {
            TableContext::ColHeader => self.header_clicked(),
            TableContext::Cell => {
                let server = &self.server_list.borrow()[self.table.callback_row() as _];
                self.on_server_selected.borrow()(server);
            }
            _ => (),
        }
    }

    fn header_clicked(&self) {
        let mut table = self.table.clone();
        let col = table.callback_col();
        let new_key = match column_to_sort_key(col) {
            Some(key) => key,
            None => return,
        };
        let old_criteria = *self.sort_criteria.borrow();
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
        table.set_col_header_value(
            col,
            &sortable_column_header(col, Some(new_criteria.ascending)),
        );
        *self.sort_criteria.borrow_mut() = new_criteria;
        self.on_sort_changed.borrow()(new_criteria);
    }
}

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
