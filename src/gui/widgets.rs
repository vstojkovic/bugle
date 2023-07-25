use fltk::app;
use fltk::enums::Event;
use fltk::prelude::*;
use fltk::table::TableContext;

mod data_table;
mod read_only_text;

pub use self::data_table::{
    draw_table_cell, DataColumn, DataTable, DataTableProperties, DataTableUpdate,
};
pub use self::read_only_text::{ReadOnlyText, ReadOnlyTextElement};

use super::is_table_nav_event;

pub fn make_readonly_cell_widget<T: 'static>(table: &DataTable<T>) -> ReadOnlyText {
    let mut cell = ReadOnlyText::new(String::new());
    cell.set_scrollbar_size(-1);
    cell.hide();

    cell.handle(move |cell, event| {
        if let Event::Unfocus = event {
            cell.hide();
        }
        false
    });

    {
        let mut cell = cell.clone();
        let table = table.clone();
        table.clone().handle(move |_, event| {
            if (event == Event::Push) || (event == Event::MouseWheel) {
                cell.hide();
            }
            if is_table_nav_event() && app::event_clicks() {
                if let TableContext::Cell = table.callback_context() {
                    let row = table.callback_row();
                    let col = table.callback_col();
                    if let Some((x, y, w, h)) = table.find_cell(TableContext::Cell, row, col) {
                        cell.resize(x, y, w, h);
                        let cell_value = table.cell_text(row, col);
                        let cell_value_len = cell_value.len();
                        cell.set_value(cell_value.to_string());
                        cell.buffer().unwrap().select(0, cell_value_len as _);
                        cell.show();
                        let _ = cell.take_focus();
                    }
                }
            }
            false
        });
    }

    cell
}
