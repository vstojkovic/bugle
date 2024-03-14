use std::cell::{Ref, RefCell};
use std::ops::{Deref, DerefMut, Index};
use std::rc::Rc;

use bitflags::bitflags;
use fltk::enums::{Align, Color, Font, FrameType};
use fltk::prelude::{TableExt, WidgetBase, WidgetExt};
use fltk::table::{TableContext, TableRow};

#[derive(Debug, Clone)]
pub struct DataColumn {
    pub header: String,
    pub align: Align,
    pub width: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct DataTableProperties {
    pub columns: Vec<DataColumn>,
    pub cell_color: Color,
    pub cell_font: Font,
    pub cell_font_color: Color,
    pub cell_font_size: i32,
    pub cell_selection_color: Color,
    pub cell_border_color: Color,
    pub cell_padding: i32,
    pub header_font: Font,
    pub header_frame: FrameType,
    pub header_color: Color,
    pub header_font_color: Color,
    pub header_font_size: i32,
    pub row_header_align: Align,
}

impl Default for DataTableProperties {
    fn default() -> Self {
        Self {
            columns: Vec::new(),
            cell_color: Color::BackGround2,
            cell_font: Font::Helvetica,
            cell_font_color: Color::Gray0,
            cell_font_size: 14,
            cell_selection_color: Color::from_u32(0x00D3D3D3),
            cell_border_color: Color::Gray0,
            cell_padding: 1,
            header_font: Font::Helvetica,
            header_frame: FrameType::ThinUpBox,
            header_color: Color::FrameDefault,
            header_font_color: Color::Black,
            header_font_size: 14,
            row_header_align: Align::Center,
        }
    }
}

bitflags! {
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct DataTableUpdate: u8 {
        const NONE = 0b00000000;
        const PROPERTIES = 0b00000001;
        const DATA = 0b00000010;
    }
}

pub struct DataTable<T: 'static> {
    inner: TableRow,
    renderer: Rc<dyn Fn(&T, usize) -> &str>,
    props: Rc<RefCell<DataTableProperties>>,
    data: Rc<RefCell<Vec<T>>>,
}

impl<T: 'static> Clone for DataTable<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            renderer: Rc::clone(&self.renderer),
            props: Rc::clone(&self.props),
            data: Rc::clone(&self.data),
        }
    }
}

impl<R: AsRef<str> + 'static, T: Index<usize, Output = R> + 'static> DataTable<T> {
    pub fn default() -> Self {
        Self::new(|row, col| row[col].as_ref())
    }
}

impl<T: 'static> DataTable<T> {
    pub fn new(renderer: impl 'static + Fn(&T, usize) -> &str) -> Self {
        let result = Self {
            inner: TableRow::default_fill(),
            renderer: Rc::new(renderer),
            props: Rc::new(RefCell::new(Default::default())),
            data: Rc::new(RefCell::new(Vec::new())),
        };

        result.with_draw_fn(Self::default_draw_cell)
    }

    pub fn properties(&self) -> Rc<RefCell<DataTableProperties>> {
        Rc::clone(&self.props)
    }

    pub fn data(&self) -> Rc<RefCell<Vec<T>>> {
        Rc::clone(&self.data)
    }

    pub fn with_properties(self, properties: DataTableProperties) -> Self {
        *self.properties().borrow_mut() = properties;
        self.updated(DataTableUpdate::PROPERTIES);
        self
    }

    pub fn with_draw_fn(
        mut self,
        mut draw_fn: impl 'static + FnMut(&DataTable<T>, i32, i32, i32, i32, i32, i32),
    ) -> Self {
        let renderer = Rc::clone(&self.renderer);
        let props = Rc::clone(&self.props);
        let data = Rc::clone(&self.data);
        let this = self.clone();
        self.inner.draw_cell(move |_, ctx, row, col, x, y, w, h| {
            let props = props.borrow();
            match ctx {
                TableContext::ColHeader => Self::draw_header(
                    &props.columns[col as usize].header,
                    x,
                    y,
                    w,
                    h,
                    props.columns[col as usize].align,
                    &props,
                ),
                TableContext::RowHeader => {
                    let data = data.borrow();
                    let text = renderer(&data[row as usize], 0);
                    Self::draw_header(text, x, y, w, h, props.row_header_align, &props);
                }
                TableContext::Cell => {
                    fltk::draw::push_clip(x, y, w, h);
                    draw_fn(&this, row, col, x, y, w, h);
                    fltk::draw::pop_clip();
                }
                _ => (),
            }
        });

        self
    }

    pub fn cell_text(&self, row: i32, col: i32) -> Ref<str> {
        let col_offset = if self.inner.row_header() { 1 } else { 0 };
        let data = self.data.borrow();
        Ref::map(data, |data| {
            (self.renderer)(&data[row as usize], col as usize + col_offset)
        })
    }

    pub fn updated(&self, update: DataTableUpdate) {
        if update.is_empty() {
            return;
        }

        let mut inner = self.inner.clone();
        if update.contains(DataTableUpdate::PROPERTIES) {
            let props = self.props.borrow();
            inner.set_cols(props.columns.len() as _);
            for (idx, col) in props.columns.iter().enumerate() {
                if let Some(width) = col.width {
                    inner.set_col_width(idx as i32, width);
                }
            }
        }
        if update.contains(DataTableUpdate::DATA) {
            let data = self.data.borrow();
            inner.set_rows(data.len() as _);
        }
        inner.redraw();
    }

    pub fn set_flex_col(&mut self, mut flex_col: i32) {
        let mut flex_width = self.width();

        let frame = self.frame();
        flex_width -= frame.dx() + frame.dw();
        flex_width -= fltk::app::scrollbar_size();
        if self.row_header() {
            flex_col -= 1;
            flex_width -= self.row_header_width();
        }
        for col in 0..self.cols() {
            if col != flex_col {
                flex_width -= self.col_width(col);
            }
        }
        self.set_col_width(flex_col, flex_width);

        let mut this = self.inner.clone();
        let mut old_width = self.width();
        self.resize_callback(move |_, _, _, width, _| {
            let delta = width - old_width;
            old_width = width;
            let flex_width = this.col_width(flex_col) + delta;
            this.set_col_width(flex_col, flex_width);
        });
    }

    pub fn default_draw_cell(&self, row: i32, col: i32, x: i32, y: i32, w: i32, h: i32) {
        let text = self.cell_text(row, col);
        let props = self.props.borrow();
        let fill_color = if self.is_selected(row as i32, col as i32) {
            props.cell_selection_color
        } else {
            props.cell_color
        };
        draw_table_cell(
            &*text,
            x,
            y,
            w,
            h,
            props.columns[col as usize].align,
            props.cell_border_color,
            fill_color,
            props.cell_font_color,
            props.cell_font,
            props.cell_font_size,
            props.cell_padding,
        )
    }

    fn draw_header(
        text: &str,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        align: Align,
        props: &DataTableProperties,
    ) {
        fltk::draw::push_clip(x, y, w, h);
        fltk::draw::draw_box(props.header_frame, x, y, w, h, props.header_color);
        fltk::draw::set_draw_color(props.header_font_color);
        fltk::draw::set_font(props.header_font, props.header_font_size);
        fltk::draw::draw_text2(
            text,
            x + props.cell_padding,
            y,
            w - props.cell_padding * 2,
            h,
            align,
        );
        fltk::draw::pop_clip();
    }
}

impl<T: 'static> Deref for DataTable<T> {
    type Target = TableRow;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: 'static> DerefMut for DataTable<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl Default for DataColumn {
    fn default() -> Self {
        Self {
            header: String::new(),
            align: Align::Center,
            width: None,
        }
    }
}

impl From<String> for DataColumn {
    fn from(value: String) -> Self {
        Self {
            header: value,
            ..Default::default()
        }
    }
}

impl From<&str> for DataColumn {
    fn from(value: &str) -> Self {
        value.to_owned().into()
    }
}

impl From<Align> for DataColumn {
    fn from(value: Align) -> Self {
        Self {
            align: value,
            ..Default::default()
        }
    }
}

impl From<i32> for DataColumn {
    fn from(value: i32) -> Self {
        Self {
            width: Some(value),
            ..Default::default()
        }
    }
}

impl<S: Into<String>> From<(S, Align)> for DataColumn {
    fn from(value: (S, Align)) -> Self {
        Self {
            header: value.0.into(),
            align: value.1,
            ..Default::default()
        }
    }
}

impl<S: Into<String>> From<(S, i32)> for DataColumn {
    fn from(value: (S, i32)) -> Self {
        Self {
            header: value.0.into(),
            width: Some(value.1),
            ..Default::default()
        }
    }
}

impl DataColumn {
    pub fn with_header<S: Into<String>>(mut self, header: S) -> Self {
        self.header = header.into();
        self
    }

    pub fn with_align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    pub fn with_width<W: Into<Option<i32>>>(mut self, width: W) -> Self {
        self.width = width.into();
        self
    }
}

pub fn draw_table_cell(
    text: &str,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    align: Align,
    border_color: Color,
    fill_color: Color,
    text_color: Color,
    text_font: Font,
    text_size: i32,
    padding: i32,
) {
    fltk::draw::set_draw_color(fill_color);
    fltk::draw::draw_rectf(x, y, w, h);
    fltk::draw::set_draw_color(text_color);
    fltk::draw::set_font(text_font, text_size);
    fltk::draw::draw_text2(text, x + padding, y, w - padding * 2, h, align);
    fltk::draw::set_draw_color(border_color);
    fltk::draw::draw_rect(x, y, w, h);
}
