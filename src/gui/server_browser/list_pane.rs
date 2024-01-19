use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::CString;
use std::rc::Rc;

use fltk::enums::{Align, Event};
use fltk::frame::Frame;
use fltk::misc::Tooltip;
use fltk::prelude::*;
use fltk::table::TableContext;
use lazy_static::lazy_static;

use crate::gui::data::{IterableTableSource, TableSource};
use crate::gui::widgets::{DataColumn, DataTable, DataTableProperties, DataTableUpdate};
use crate::gui::{glyph, is_table_nav_event};
use crate::servers::{Server, SortCriteria, SortKey};

use super::{mode_name, region_name};

type ServerRow = [Cow<'static, str>; NUM_COLS];

pub(super) struct ListPane {
    table: DataTable<ServerRow>,
    loading_label: Frame,
    sort_criteria: RefCell<SortCriteria>,
    server_list: RefCell<Rc<RefCell<dyn TableSource<Output = Server>>>>,
    on_sort_changed: RefCell<Box<dyn Fn(SortCriteria)>>,
    on_server_selected: RefCell<Box<dyn Fn(Option<&Server>)>>,
    selection: RefCell<Selection>,
}

struct Selection {
    index: Option<usize>,
    scroll_lock: bool,
}

impl ListPane {
    pub fn new(initial_sort: &SortCriteria, scroll_lock: bool) -> Rc<Self> {
        let sorted_col = sort_key_to_column(initial_sort.key);
        let columns = SERVER_LIST_COLS
            .iter()
            .enumerate()
            .map(|(idx, col)| {
                let ascending = if idx == sorted_col { Some(initial_sort.ascending) } else { None };
                col.to_data_column(ascending)
            })
            .collect();
        let mut table = DataTable::default().with_properties(DataTableProperties {
            columns,
            cell_padding: 4,
            cell_selection_color: fltk::enums::Color::Free,
            header_font_color: fltk::enums::Color::Gray0,
            ..Default::default()
        });
        table.make_resizable(true);
        table.set_row_header(false);
        table.set_col_header(true);
        table.set_col_resize(true);

        table.end();
        table.hide();

        let loading_label = Frame::default_fill()
            .with_label("Fetching server list...")
            .with_align(Align::Center);

        let list_pane = Rc::new(Self {
            table: table.clone(),
            loading_label,
            sort_criteria: RefCell::new(*initial_sort),
            server_list: RefCell::new(Rc::new(RefCell::new(Vec::new()))),
            on_sort_changed: RefCell::new(Box::new(|_| ())),
            on_server_selected: RefCell::new(Box::new(|_| ())),
            selection: RefCell::new(Selection {
                index: None,
                scroll_lock,
            }),
        });

        {
            let list_pane = Rc::downgrade(&list_pane);
            table.set_callback(move |_| {
                if let Some(list_pane) = list_pane.upgrade() {
                    if is_table_nav_event() {
                        list_pane.clicked();
                    }
                }
            });
        }
        {
            let list_pane = Rc::downgrade(&list_pane);
            let mut tooltip_pos = None;
            table.handle(move |_, event| {
                if let Some(list_pane) = list_pane.upgrade() {
                    list_pane.update_tooltip(event, &mut tooltip_pos);
                }
                false
            });
        }

        list_pane
    }

    pub fn populate(&self, server_list: Rc<RefCell<dyn TableSource<Output = Server>>>) {
        self.clear_refreshing();
        self.set_server_list(server_list);
    }

    pub fn mark_refreshing(&self) {
        self.set_server_list(Rc::new(RefCell::new(Vec::new())));
        self.table.clone().hide();
        self.loading_label.clone().show();
    }

    pub fn clear_refreshing(&self) {
        self.loading_label.clone().hide();
        self.table.clone().show();
    }

    pub fn update(&self, indices: impl IntoIterator<Item = usize>) {
        let servers_ref = self.server_list.borrow();
        let servers = servers_ref.borrow();

        let selection = self.selection.borrow();
        let mut reselect = false;

        {
            let data = self.table.data();
            let mut data = data.borrow_mut();
            for idx in indices.into_iter() {
                data[idx] = make_server_row(&servers[idx]);
                if Some(idx) == selection.index {
                    reselect = true;
                }
            }
        }
        self.table.updated(DataTableUpdate::DATA);

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

    pub fn scroll_lock(&self) -> bool {
        self.selection.borrow().scroll_lock
    }

    pub fn set_scroll_lock(&self, scroll_lock: bool) {
        self.selection.borrow_mut().scroll_lock = scroll_lock;
        if scroll_lock {
            self.ensure_selection_visible();
        }
    }

    fn set_server_list(&self, server_list: Rc<RefCell<dyn TableSource<Output = Server>>>) {
        {
            let servers = server_list.borrow();
            {
                *self.table.data().borrow_mut() = servers.iter().map(make_server_row).collect();
            }
            self.table.updated(DataTableUpdate::DATA);
        }
        *self.server_list.borrow_mut() = server_list;
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
        let col = self.table.callback_col() as usize;
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
        {
            let props = self.table.properties();
            let mut props = props.borrow_mut();
            if old_criteria.key != new_criteria.key {
                let old_col = sort_key_to_column(old_criteria.key);
                props.columns[old_col].header = SERVER_LIST_COLS[old_col].header(None);
            }
            props.columns[col].header = SERVER_LIST_COLS[col].header(Some(new_criteria.ascending));
        }
        self.table.updated(DataTableUpdate::PROPERTIES);
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

    fn update_tooltip(&self, event: Event, tooltip_pos: &mut Option<(TableContext, i32, i32)>) {
        let table_widget: &fltk::table::TableRow = &self.table;
        match event {
            Event::Move => {
                let mut new_pos = match self.table.cursor2rowcol() {
                    Some((TableContext::ColHeader, row, col, _)) if col < 6 => {
                        Some((TableContext::ColHeader, row, col))
                    }
                    Some((TableContext::Cell, row, col, _)) if col < 6 => {
                        Some((TableContext::Cell, row, col))
                    }
                    _ => None,
                };
                if *tooltip_pos != new_pos {
                    if let Some((TableContext::Cell, row, col)) = &new_pos {
                        let data = self.table.data();
                        let data = data.borrow();
                        if data[*row as usize][*col as usize].is_empty() {
                            new_pos = None;
                        }
                    }
                }
                if *tooltip_pos != new_pos {
                    *tooltip_pos = new_pos;
                    Tooltip::current(&self.table.parent().unwrap());
                    if let Some((ctx, row, col)) = &tooltip_pos {
                        let (x, y, w, h) = self.table.find_cell(*ctx, *row, *col).unwrap();
                        Tooltip::enter_area(
                            table_widget,
                            x - &self.table.x(),
                            y - &self.table.y(),
                            w,
                            h,
                            COL_TOOLTIPS[*col as usize].as_c_str(),
                        );
                    }
                }
            }
            Event::Leave => {
                if tooltip_pos.is_some() {
                    *tooltip_pos = None;
                }
            }
            _ => (),
        }
    }
}

struct Column {
    header: &'static str,
    width: i32,
    align: Align,
    sort_key: Option<SortKey>,
    value_fn: fn(&Server) -> Cow<'static, str>,
}

impl Column {
    const fn new(
        header: &'static str,
        width: i32,
        align: Align,
        sort_key: Option<SortKey>,
        value_fn: fn(&Server) -> Cow<'static, str>,
    ) -> Self {
        Self {
            header,
            width,
            align,
            sort_key,
            value_fn,
        }
    }

    fn value_for(&self, server: &Server) -> Cow<'static, str> {
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

    fn to_data_column(&self, ascending: Option<bool>) -> DataColumn {
        DataColumn::default()
            .with_header(self.header(ascending))
            .with_align(self.align)
            .with_width(self.width)
    }
}

macro_rules! col {
    ($header:expr, $width:expr, $align:ident, $sort_key:expr, $value_fn:expr) => {
        Column::new($header, $width, Align::$align, $sort_key, $value_fn)
    };
}

#[rustfmt::skip]
const SERVER_LIST_COLS: &[Column] = &[
    col!(glyph::WARNING, 20, Center, None, |server| str_if(!server.is_valid(), glyph::WARNING)),
    col!(glyph::LOCK, 20, Center, None, |server| str_if(server.password_protected, glyph::LOCK)),
    col!(glyph::TOOLS, 20, Center, None, |server| str_if(server.is_modded(), glyph::TOOLS)),
    col!(glyph::FLAG, 20, Center, None, |server| str_if(server.is_official(), glyph::FLAG)),
    col!(glyph::EYE, 20, Center, None, |server| str_if(server.battleye_required, glyph::EYE)),
    col!(glyph::HEART, 20, Center, None, |server| str_if(server.favorite, glyph::HEART)),
    col!("Server Name", 470, Left, Some(SortKey::Name), |server| server.name.clone().into()),
    col!("Map", 150, Center, Some(SortKey::Map), |server| server.map.clone().into()),
    col!("Mode", 80, Center, Some(SortKey::Mode), |server| mode_name(server.mode()).into()),
    col!("Region", 80, Center, Some(SortKey::Region), |server| region_name(server.region).into()),
    col!("Players", 70, Center, Some(SortKey::Players), |server| players_col_value(server).into()),
    col!("Age", 60, Center, Some(SortKey::Age), |server| age_col_value(server).into()),
    col!("Ping", 60, Center, Some(SortKey::Ping), |server| ping_col_value(server).into()),
];
const NUM_COLS: usize = SERVER_LIST_COLS.len();

lazy_static! {
    static ref SORT_KEY_TO_COLUMN: HashMap<SortKey, usize> = {
        let mut map = HashMap::new();
        for col in 0..SERVER_LIST_COLS.len() {
            if let Some(sort_key) = column_to_sort_key(col) {
                map.insert(sort_key, col);
            }
        }
        map
    };
    static ref COL_TOOLTIPS: [CString; 6] = [
        CString::new("Invalid").unwrap(),
        CString::new("Password protected").unwrap(),
        CString::new("Modded").unwrap(),
        CString::new("Official").unwrap(),
        CString::new("BattlEye required").unwrap(),
        CString::new("Favorite").unwrap(),
    ];
}

fn sort_key_to_column(sort_key: SortKey) -> usize {
    *SORT_KEY_TO_COLUMN.get(&sort_key).unwrap()
}

fn column_to_sort_key(col: usize) -> Option<SortKey> {
    SERVER_LIST_COLS[col].sort_key
}

fn str_if(condition: bool, str_true: &'static str) -> Cow<'static, str> {
    (if condition { str_true } else { "" }).into()
}

fn players_col_value(server: &Server) -> String {
    match server.connected_players {
        Some(players) => format!("{}/{}{}", players, server.max_players, pong_suffix(server)),
        None => format!("?/{}{}", server.max_players, pong_suffix(server)),
    }
}

fn age_col_value(server: &Server) -> String {
    match server.age {
        Some(age) => format!("{}{}", age.as_secs() / 86400, pong_suffix(server)),
        None => format!("????{}", pong_suffix(server)),
    }
}

fn ping_col_value(server: &Server) -> String {
    match server.ping {
        Some(ping) => format!("{}{}", ping.as_millis(), pong_suffix(server)),
        None => format!("????{}", pong_suffix(server)),
    }
}

fn pong_suffix(server: &Server) -> &str {
    if server.waiting_for_pong {
        " @-1reload"
    } else {
        ""
    }
}

fn make_server_row(server: &Server) -> ServerRow {
    std::array::from_fn(|idx| SERVER_LIST_COLS[idx].value_for(server))
}
