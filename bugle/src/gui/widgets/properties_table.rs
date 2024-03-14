use std::borrow::Cow;
use std::ops::{Deref, DerefMut};

use fltk::prelude::*;
use fltk::table::TableRow;

use super::{
    make_readonly_cell_widget, DataTable, DataTableProperties, DataTableUpdate, ReadOnlyText,
};

pub type PropertyRow = [Cow<'static, str>; 2];
pub type Inspector<S, C> = fn(&C, Option<&S>, &mut dyn FnMut(PropertyRow), bool);

pub struct PropertiesTable<S: 'static, C: 'static> {
    ctx: C,
    inspectors: &'static [Inspector<S, C>],
    table: DataTable<PropertyRow>,
    cell: ReadOnlyText,
}

impl<S: 'static, C: 'static> PropertiesTable<S, C> {
    pub fn new(ctx: C, inspectors: &'static [Inspector<S, C>], title: &'static str) -> Self {
        let table_props = DataTableProperties {
            columns: vec![title.into()],
            cell_selection_color: fltk::enums::Color::Free,
            header_font_color: fltk::enums::Color::Gray0,
            ..Default::default()
        };
        let width_padding = table_props.cell_padding * 2 + fltk::app::scrollbar_size();

        let mut table = DataTable::<PropertyRow>::default().with_properties(table_props);
        table.set_row_header(true);
        table.set_col_header(true);
        table.set_col_resize(true);

        table.end();

        let cell = make_readonly_cell_widget(&table);

        let this = Self {
            ctx,
            inspectors,
            table,
            cell,
        };
        this.populate(None);

        let mut table = this.table.clone();
        let mut header_width = 0i32;
        fltk::draw::set_font(table.label_font(), table.label_size());
        let mut consumer = |row: PropertyRow| {
            let (w, _) = fltk::draw::measure(row[0].as_ref(), true);
            header_width = std::cmp::max(header_width, w);
        };
        for inspector in this.inspectors.iter() {
            inspector(&this.ctx, None, &mut consumer, true);
        }
        header_width += width_padding;
        table.set_row_header_width(header_width);

        table.set_flex_col(1);

        this
    }

    pub fn populate(&self, subject: Option<&S>) {
        self.cell.clone().hide();
        {
            let data = self.table.data();
            let mut data = data.borrow_mut();
            data.clear();
            let mut consumer = |row| data.push(row);
            for inspector in self.inspectors.iter() {
                inspector(&self.ctx, subject, &mut consumer, false);
            }
        }
        self.table.updated(DataTableUpdate::DATA);
    }
}

impl<S: 'static, C: 'static> Deref for PropertiesTable<S, C> {
    type Target = TableRow;
    fn deref(&self) -> &Self::Target {
        &self.table
    }
}

impl<S: 'static, C: 'static> DerefMut for PropertiesTable<S, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.table
    }
}

macro_rules! use_inspector_macros {
    (@ $subject:ty, $ctx:ty, $d:tt) => {
        #[allow(unused_macros)]
        macro_rules! inspect_attr {
            ($d header:literal, $d lambda:expr) => {
                |_: &$ctx,
                 subject: Option<&$subject>,
                 row_consumer: &mut dyn FnMut($crate::gui::widgets::PropertyRow),
                 _include_empty: bool| {
                    row_consumer([$d header.into(), subject.map($d lambda).unwrap_or_default()]);
                }
            };
        }
        #[allow(unused_macros)]
        macro_rules! inspect_opt_attr {
            ($d header:literal, $d lambda:expr) => {
                |_: &$ctx,
                 subject: Option<&$subject>,
                 row_consumer: &mut dyn FnMut($crate::gui::widgets::PropertyRow),
                 include_empty: bool| {
                    let value = subject.and_then($d lambda);
                    if value.is_some() || include_empty {
                        row_consumer([$d header.into(), value.unwrap_or_default()]);
                    }
                }
            };
        }
    };
    ($subject:ty, $ctx:ty) => {
        use_inspector_macros!(@ $subject, $ctx, $);
    }
}
pub(crate) use use_inspector_macros;
