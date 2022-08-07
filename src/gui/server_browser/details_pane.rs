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

        table.end();

        Self { table }
    }

    pub fn populate(&self, server: &Server) {
        let mut table = self.table.clone();
        table.set_cell_value(0, 0, &server.id);
        table.set_cell_value(1, 0, &server.name);
        table.set_cell_value(2, 0, &format!("{}:{}", server.ip, server.port));
        table.set_cell_value(3, 0, &server.map);
        table.set_cell_value(4, 0, mode_name(server.mode()));
        table.set_cell_value(5, 0, region_name(server.region));
        table.redraw();
    }
}

const SERVER_DETAILS_ROWS: &[&str] = &["ID", "Server Name", "Host", "Map Name", "Mode", "Region"];
