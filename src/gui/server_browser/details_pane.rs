use std::borrow::Cow;

use fltk::prelude::*;
use fltk_table::{SmartTable, TableOpts};

use crate::servers::Server;

use super::{mode_name, region_name};

pub(super) struct DetailsPane {
    table: SmartTable,
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

        Self { table }
    }

    pub fn populate(&self, server: &Server) {
        let mut table = self.table.clone();
        for (idx, (_, cell_value)) in SERVER_DETAILS_ROWS.iter().enumerate() {
            table.set_cell_value(idx as _, 0, cell_value(server).as_ref());
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
];
