use fltk::group::{Group};
use fltk::prelude::*;
use fltk_table::{SmartTable, TableOpts};

pub(super) struct ServerBrowser {
    pub(super) group: Group,
}

impl ServerBrowser {
    pub(super) fn new() -> Self {
        let mut group = Group::default_fill();

        let _server_list = make_table(&[
            ("\u{1f512}", 20),
            ("Server Name", 420),
            ("Map", 160),
            ("Mode", 80),
            ("Region", 80),
            ("Players", 60),
            ("Age", 60),
            ("Ping", 60),
            ("BattlEye", 60), // 2714 / 2716
            ("Level", 60),
        ]);

        group.end();
        group.hide();

        Self { group }
    }
}

fn make_table(cols: &[(&str, i32)]) -> SmartTable {
    let mut table = SmartTable::default_fill()
        .with_opts(TableOpts {
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
