use std::rc::Rc;

use fltk::group::Group;
use fltk::{dialog, prelude::*};
use fltk_table::{SmartTable, TableOpts};

use super::{Action, ActionHandler, CleanupFn};

pub enum ServerBrowserAction {
    LoadServers,
}

pub(super) struct ServerBrowser {
    pub(super) group: Group,
    on_action: Rc<dyn ActionHandler>,
}

impl ServerBrowser {
    pub(super) fn new(on_action: Rc<dyn ActionHandler>) -> Self {
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

        Self { group, on_action }
    }

    pub(super) fn show(&mut self) -> CleanupFn {
        self.group.show();

        if let Err(err) = self.action(ServerBrowserAction::LoadServers) {
            dialog::alert_default(&format!("Error while loading server list:\n{}", err));
        }

        let mut group = self.group.clone();
        Box::new(move || {
            group.hide();
            None
        })
    }

    fn action(&self, action: ServerBrowserAction) -> anyhow::Result<()> {
        (self.on_action)(Action::ServerBrowser(action))
    }
}

fn make_table(cols: &[(&str, i32)]) -> SmartTable {
    let mut table = SmartTable::default_fill().with_opts(TableOpts {
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
