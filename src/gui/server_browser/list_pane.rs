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
    on_server_selected: RefCell<Box<dyn Fn(Option<&Server>)>>,
    selection: RefCell<Selection>,
}

struct Selection {
    index: Option<usize>,
    scroll_lock: bool,
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

        for (idx, col) in SERVER_LIST_COLS.iter().enumerate() {
            let idx = idx as _;
            let ascending = if idx == sorted_col { Some(initial_sort.ascending) } else { None };
            table.set_col_header_value(idx, &col.header(ascending));
            table.set_col_width(idx, col.width);
        }

        table.end();

        let list_pane = Rc::new(Self {
            table: table.clone(),
            sort_criteria: RefCell::new(*initial_sort),
            server_list: RefCell::new(Rc::new(RefCell::new(Vec::new()))),
            on_sort_changed: RefCell::new(Box::new(|_| ())),
            on_server_selected: RefCell::new(Box::new(|_| ())),
            selection: RefCell::new(Selection {
                index: None,
                scroll_lock: true,
            }),
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

        let selection = self.selection.borrow();
        let mut reselect = false;

        let data_ref = table.data_ref();
        {
            let mut data = data_ref.lock().unwrap();
            for idx in indices.into_iter() {
                data[idx] = make_server_row(&servers[idx]);
                if Some(idx) == selection.index {
                    reselect = true;
                }
            }
        }
        table.redraw();

        if reselect {
            self.on_server_selected.borrow()(Some(&servers[selection.index.unwrap()]));
        }
    }

    pub fn set_on_sort_changed(&self, on_sort_changed: impl Fn(SortCriteria) + 'static) {
        *self.on_sort_changed.borrow_mut() = Box::new(on_sort_changed);
    }

    pub fn set_on_server_selected(&self, on_server_selected: impl Fn(Option<&Server>) + 'static) {
        *self.on_server_selected.borrow_mut() = Box::new(on_server_selected);
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.selection.borrow().index
    }

    pub fn set_selected_index(&self, index: Option<usize>, override_scroll_lock: bool) {
        {
            let mut selection = self.selection.borrow_mut();
            if index == selection.index {
                return;
            }
            selection.index = index;
            let mut table = self.table.clone();
            if let Some(index) = index {
                let row = index as _;
                table.set_selection(row, 0, row, (SERVER_LIST_COLS.len() - 1) as _);
            } else {
                table.unset_selection();
            }
        }
        if override_scroll_lock || self.selection.borrow().scroll_lock {
            self.ensure_selection_visible();
        }
        if let Some(index) = index {
            let server_list = self.server_list.borrow();
            let server = &server_list.borrow()[index];
            self.on_server_selected.borrow()(Some(server));
        } else {
            self.on_server_selected.borrow()(None);
        }
    }

    pub fn set_scroll_lock(&self, scroll_lock: bool) {
        self.selection.borrow_mut().scroll_lock = scroll_lock;
        if scroll_lock {
            self.ensure_selection_visible();
        }
    }

    fn clicked(&self) {
        match self.table.callback_context() {
            TableContext::ColHeader => self.header_clicked(),
            TableContext::Cell => {
                let _ = self.table.clone().take_focus();

                let selected_idx = self.table.callback_row() as _;
                self.selection.borrow_mut().index = Some(selected_idx);
                let server_list = self.server_list.borrow();
                let server = &server_list.borrow()[selected_idx];
                self.on_server_selected.borrow()(Some(server));
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
            table.set_col_header_value(old_col, &SERVER_LIST_COLS[old_col as usize].header(None));
        }
        table.set_col_header_value(
            col,
            &SERVER_LIST_COLS[col as usize].header(Some(new_criteria.ascending)),
        );
        *self.sort_criteria.borrow_mut() = new_criteria;
        self.on_sort_changed.borrow()(new_criteria);
    }

    fn ensure_selection_visible(&self) {
        let row = match self.selection.borrow().index {
            Some(index) => index as i32,
            None => return,
        };
        let mut table = self.table.clone();
        if let Some((top, bottom, _, _)) = table.try_visible_cells() {
            let row_span = bottom - top;
            let new_top = i32::max(row - row_span / 2, 0);
            if top != new_top {
                table.set_row_position(new_top);
            }
        }
    }
}

struct Column {
    header: &'static str,
    width: i32,
    sort_key: Option<SortKey>,
    value_fn: fn(&Server) -> String,
}

impl Column {
    const fn new(
        header: &'static str,
        width: i32,
        sort_key: Option<SortKey>,
        value_fn: fn(&Server) -> String,
    ) -> Self {
        Self {
            header,
            width,
            sort_key,
            value_fn,
        }
    }

    fn value_for(&self, server: &Server) -> String {
        (self.value_fn)(server)
    }

    fn header(&self, ascending: Option<bool>) -> String {
        if self.sort_key.is_none() {
            return self.header.to_string();
        }

        format!(
            "{} {}",
            self.header,
            match ascending {
                None => glyph::UNSORTED,
                Some(false) => glyph::DESC,
                Some(true) => glyph::ASC,
            }
        )
    }
}

macro_rules! col {
    ($header:expr, $width:expr, $sort_key:expr, $value_fn:expr) => {
        Column::new($header, $width, $sort_key, $value_fn)
    };
}

#[rustfmt::skip]
const SERVER_LIST_COLS: &[Column] = &[
    col!(glyph::WARNING, 20, None, |server| str_if(!server.is_valid(), glyph::WARNING)),
    col!(glyph::LOCK, 20, None, |server| str_if(server.password_protected, glyph::LOCK)),
    col!(glyph::TOOLS, 20, None, |server| str_if(server.is_modded(), glyph::TOOLS)),
    col!(glyph::FLAG, 20, None, |server| str_if(server.is_official(), glyph::FLAG)),
    col!(glyph::EYE, 20, None, |server| str_if(server.battleye_required, glyph::EYE)),
    col!(glyph::HEART, 20, None, |server| str_if(server.favorite, glyph::HEART)),
    col!("Server Name", 380, Some(SortKey::Name), |server| server.name.clone()),
    col!("Map", 150, Some(SortKey::Map), |server| server.map.clone()),
    col!("Mode", 80, Some(SortKey::Mode), |server| mode_name(server.mode()).to_string()),
    col!("Region", 80, Some(SortKey::Region), |server| region_name(server.region).to_string()),
    col!("Players", 70, Some(SortKey::Players), |server| players_col_value(server)),
    col!("Age", 60, Some(SortKey::Age), |server| age_col_value(server)),
    col!("Ping", 60, Some(SortKey::Ping), |server| ping_col_value(server)),
];

lazy_static! {
    static ref SORT_KEY_TO_COLUMN: HashMap<SortKey, i32> = {
        let mut map = HashMap::new();
        for col in 0..SERVER_LIST_COLS.len() {
            let col = col as _;
            if let Some(sort_key) = column_to_sort_key(col) {
                map.insert(sort_key, col);
            }
        }
        map
    };
}

fn sort_key_to_column(sort_key: SortKey) -> i32 {
    *SORT_KEY_TO_COLUMN.get(&sort_key).unwrap()
}

fn column_to_sort_key(col: i32) -> Option<SortKey> {
    SERVER_LIST_COLS[col as usize].sort_key
}

fn str_if(condition: bool, str_true: &str) -> String {
    (if condition { str_true } else { "" }).to_string()
}

fn players_col_value(server: &Server) -> String {
    match server.connected_players {
        Some(players) => format!("{}/{}", players, server.max_players),
        None => format!("?/{}", server.max_players),
    }
}

fn age_col_value(server: &Server) -> String {
    match server.age {
        Some(age) => format!("{}", age.as_secs() / 86400),
        None => "????".to_string(),
    }
}

fn ping_col_value(server: &Server) -> String {
    match server.ping {
        Some(ping) => format!("{}", ping.as_millis()),
        None => "????".to_string(),
    }
}

fn make_server_row(server: &Server) -> Vec<String> {
    SERVER_LIST_COLS
        .iter()
        .map(|col| col.value_for(server))
        .collect()
}
