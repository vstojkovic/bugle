use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use fltk::app;
use fltk::enums::Event;
use fltk::prelude::*;
use fltk::table::TableContext;
use fltk_table::{SmartTable, TableOpts};
use lazy_static::lazy_static;

use crate::gui::glyph;
use crate::servers::{Server, ServerList, SortCriteria, SortKey};

use super::{mode_name, region_name};

pub(super) struct ListPane {
    table: SmartTable,
    sort_criteria: RefCell<SortCriteria>,
    server_list: RefCell<Rc<RefCell<dyn ServerList>>>,
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
            server_list: RefCell::new(Rc::new(RefCell::new(Vec::new()))),
            on_sort_changed: RefCell::new(Box::new(|_| ())),
            on_server_selected: RefCell::new(Box::new(|_| ())),
        });

        {
            let list_pane = Rc::downgrade(&list_pane);
            table.set_callback(move |_| {
                if let Some(list_pane) = list_pane.upgrade() {
                    match app::event() {
                        Event::Released => {
                            if app::event_is_click() {
                                list_pane.clicked()
                            }
                        }
                        _ => (),
                    }
                }
            });
        }

        list_pane
    }

    pub fn populate(&self, server_list: Rc<RefCell<dyn ServerList>>) {
        let mut table = self.table.clone();
        {
            let servers = server_list.borrow();
            {
                let data_ref = table.data_ref();
                *data_ref.lock().unwrap() = servers.into_iter().map(make_server_row).collect();
            }
            table.set_rows(servers.len() as _);
            table.redraw();
        }
        *self.server_list.borrow_mut() = server_list;
    }

    pub fn update(&self, indices: impl IntoIterator<Item = usize>) {
        let mut table = self.table.clone();
        let servers_ref = self.server_list.borrow();
        let servers = servers_ref.borrow();
        let data_ref = table.data_ref();
        let mut data = data_ref.lock().unwrap();
        for idx in indices.into_iter() {
            data[idx] = make_server_row(&servers[idx]);
        }
        table.redraw();
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
                let _ = self.table.clone().take_focus();

                let server_list = self.server_list.borrow();
                let server = &server_list.borrow()[self.table.callback_row() as _];
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

const SERVER_LIST_COLS: &[(&str, i32)] = &[
    (glyph::WARNING, 20),
    (glyph::LOCK, 20),
    ("Server Name", 380),
    ("Map", 150),
    ("Mode", 80),
    ("Region", 80),
    ("Players", 70),
    ("Age", 60),
    ("Ping", 60),
    ("BattlEye", 60),
    ("Level", 50),
];

lazy_static! {
    static ref COLUMN_TO_SORT_KEY: HashMap<i32, SortKey> = {
        use strum::IntoEnumIterator;
        let mut map = HashMap::new();
        for sort_key in SortKey::iter() {
            map.insert(sort_key_to_column(sort_key), sort_key);
        }
        map
    };
}

fn sort_key_to_column(sort_key: SortKey) -> i32 {
    match sort_key {
        SortKey::Name => 2,
        SortKey::Map => 3,
        SortKey::Mode => 4,
        SortKey::Region => 5,
        SortKey::Players => 6,
        SortKey::Age => 7,
        SortKey::Ping => 8,
    }
}

fn column_to_sort_key(col: i32) -> Option<SortKey> {
    COLUMN_TO_SORT_KEY.get(&col).copied()
}

fn sortable_column_header(col: i32, ascending: Option<bool>) -> String {
    format!(
        "{} {}",
        SERVER_LIST_COLS[col as usize].0,
        match ascending {
            None => glyph::UNSORTED,
            Some(false) => glyph::DESC,
            Some(true) => glyph::ASC,
        }
    )
}

fn make_server_row(server: &Server) -> Vec<String> {
    let players = match server.connected_players {
        Some(players) => format!("{}/{}", players, server.max_players),
        None => format!("?/{}", server.max_players),
    };
    let age = match server.age {
        Some(age) => format!("{}", age.as_secs() / 86400),
        None => "????".to_string(),
    };
    let ping = match server.ping {
        Some(ping) => format!("{}", ping.as_millis()),
        None => "????".to_string(),
    };
    vec![
        (if server.is_valid() { "" } else { glyph::WARNING }).to_string(),
        (if server.password_protected { glyph::LOCK } else { "" }).to_string(),
        server.name.clone(),
        server.map.clone(),
        mode_name(server.mode()).to_string(),
        region_name(server.region).to_string(),
        players,
        age,
        ping,
        (if server.battleye_required { glyph::YES } else { glyph::NO }).to_string(),
        "??".to_string(), // TODO: Level
    ]
}
