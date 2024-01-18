use std::rc::Rc;
use std::sync::Arc;

use anyhow::Result;
use fltk::button::Button;
use fltk::enums::{Align, Color};
use fltk::frame::Frame;
use fltk::prelude::*;
use fltk::window::Window;
use fltk_float::grid::{CellAlign, GridBuilder};
use fltk_float::SimpleWrapper;
use humansize::SizeFormatter;

use crate::game::platform::{ModDirectory, ModUpdate};
use crate::game::{ModEntry, ModRef, Mods};
use crate::gui::widgets::{draw_table_cell, DataTable, DataTableProperties, DataTableUpdate};
use crate::gui::wrapper_factory;
use crate::workers::TaskState;

pub struct ModUpdateProgressDialog {
    window: Window,
    progress_table: DataTable<ProgressRow>,
}

enum ProgressStatus {
    Pending,
    InProgress,
    Done,
    Error,
}

impl ProgressStatus {
    fn is_final(&self) -> bool {
        match self {
            Self::Done | Self::Error => true,
            _ => false,
        }
    }
}

struct ProgressRow {
    name: String,
    update: Result<Rc<dyn ModUpdate>>,
    status: ProgressStatus,
    progress: Option<(u64, u64)>,
    display_text: String,
}

impl ProgressRow {
    fn new(mod_entry: &ModEntry, mod_directory: Rc<dyn ModDirectory>) -> Self {
        let name = mod_entry.info.as_ref().unwrap().name.clone();
        let update = mod_directory.start_update(mod_entry);
        let (status, display_text) = match &update {
            Ok(_) => (ProgressStatus::Pending, name.clone()),
            Err(_) => (ProgressStatus::Error, format!("{} [error]", &name)),
        };
        let progress = None;
        Self {
            name,
            update,
            status,
            progress,
            display_text,
        }
    }

    fn update(&mut self) {
        if self.status.is_final() {
            return;
        }
        let update = self.update.as_ref().unwrap();
        self.status = match update.state() {
            TaskState::Pending => match update.progress() {
                Some(progress) => {
                    self.progress = Some(progress);
                    ProgressStatus::InProgress
                }
                None => ProgressStatus::Error,
            },
            TaskState::Ready(Ok(())) => ProgressStatus::Done,
            TaskState::Ready(Err(_)) => ProgressStatus::Error,
        };
        self.display_text = format!("{} [{}]", &self.name, self.format_progress().as_ref());
    }

    fn format_progress(&self) -> std::borrow::Cow<str> {
        match self.status {
            ProgressStatus::Pending => unreachable!(),
            ProgressStatus::InProgress => {
                let (done, total) = self.progress.unwrap();
                let done_fmt = SizeFormatter::new(done, humansize::BINARY);
                let total_fmt = SizeFormatter::new(total, humansize::BINARY);
                format!("{} / {}", done_fmt, total_fmt).into()
            }
            ProgressStatus::Done => "finished".into(),
            ProgressStatus::Error => "error".into(),
        }
    }
}

impl ModUpdateProgressDialog {
    pub fn new(
        parent: &Window,
        mods: &Arc<Mods>,
        mods_to_update: Vec<ModRef>,
        mod_directory: Rc<dyn ModDirectory>,
    ) -> Self {
        let mut window = Window::default()
            .with_size(480, 480)
            .with_label("Updating Mods");

        let mut grid = GridBuilder::with_factory(window.clone(), wrapper_factory())
            .with_col_spacing(10)
            .with_row_spacing(10)
            .with_padding(10, 10, 10, 10);
        grid.col().with_stretch(1).add();

        grid.row().add();
        grid.cell()
            .unwrap()
            .wrap(Frame::default_fill())
            .with_label(MSG_UPDATING_MODS);

        grid.row()
            .with_stretch(1)
            .with_default_align(CellAlign::Stretch)
            .add();
        let progress_table = DataTable::<ProgressRow>::new(|row, _| &row.display_text);
        let progress_table = progress_table.with_draw_fn(draw_progress_cell);
        let mut progress_table = progress_table.with_properties(DataTableProperties {
            columns: vec!["".into()],
            cell_padding: 4,
            cell_selection_color: fltk::enums::Color::Free,
            header_font_color: fltk::enums::Color::Gray0,
            ..Default::default()
        });
        progress_table.set_row_header(false);
        progress_table.set_col_header(true);
        progress_table.set_col_resize(true);
        progress_table.end();
        let data = progress_table.data();
        let mut data = data.borrow_mut();
        for mod_ref in mods_to_update.iter() {
            let mod_info = mods.get(mod_ref).unwrap();
            data.push(ProgressRow::new(mod_info, Rc::clone(&mod_directory)));
        }
        drop(data);
        progress_table.updated(DataTableUpdate::DATA);
        grid.cell().unwrap().add(SimpleWrapper::new(
            progress_table.as_base_widget(),
            Default::default(),
        ));

        grid.row().add();
        let mut btn_skip = grid
            .cell()
            .unwrap()
            .with_horz_align(CellAlign::Center)
            .wrap(Button::default())
            .with_label("Skip");

        grid.end().layout_children();

        let scrollbar_width = progress_table.scrollbar_size();
        let scrollbar_width =
            if scrollbar_width > 0 { scrollbar_width } else { fltk::app::scrollbar_size() };
        let col_width = progress_table.width() - scrollbar_width - 2;
        progress_table.set_col_width(0, col_width);

        btn_skip.set_callback({
            let mut window = window.clone();
            move |_| window.hide()
        });

        window.set_pos(
            parent.x() + (parent.w() - window.w()) / 2,
            parent.y() + (parent.h() - window.h()) / 2,
        );

        Self {
            window,
            progress_table,
        }
    }

    pub fn run(self) {
        let mut window = self.window.clone();
        window.make_modal(true);
        window.show();

        let table = self.progress_table.clone();
        fltk::app::add_timeout3(0.5, move |handle| {
            let progress = update_progress(&table);
            if let ProgressStatus::Done = progress {
                window.hide();
            }
            if window.shown() && !progress.is_final() {
                fltk::app::repeat_timeout3(0.5, handle);
            }
        });

        while self.window.shown() {
            if !fltk::app::wait() {
                return;
            }
        }
    }
}

const MSG_UPDATING_MODS: &str = "Please wait while the following mods are being updated:";

fn update_progress(table: &DataTable<ProgressRow>) -> ProgressStatus {
    let data = table.data();
    let mut data = data.borrow_mut();

    let mut finished = true;
    let mut error = false;
    for row in data.iter_mut() {
        row.update();
        if !row.status.is_final() {
            finished = false;
        }
        if let ProgressStatus::Error = row.status {
            error = true;
        }
    }

    drop(data);
    table.updated(DataTableUpdate::DATA);

    match (finished, error) {
        (false, _) => ProgressStatus::InProgress,
        (true, false) => ProgressStatus::Done,
        (true, true) => ProgressStatus::Error,
    }
}

fn draw_progress_cell(
    table: &DataTable<ProgressRow>,
    row: i32,
    _: i32,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) {
    let data = table.data();
    let data = data.borrow();
    let row = &data[row as usize];

    let props = table.properties();
    let props = props.borrow();
    let border_color = props.cell_border_color;
    let remaining_color = props.cell_color;
    let text_color = props.cell_font_color;
    let text_font = props.cell_font;
    let text_size = props.cell_font_size;
    let padding = props.cell_padding;

    let done_width = match row.status {
        ProgressStatus::Pending => 0,
        ProgressStatus::InProgress => match row.progress {
            Some((done, total)) if done > 0 => lerp(width, done, total),
            _ => 0,
        },
        ProgressStatus::Done => width,
        ProgressStatus::Error => match row.progress {
            Some((done, total)) if done > 0 => lerp(width, done, total),
            _ => width,
        },
    };
    let done_color = if let ProgressStatus::Error = row.status {
        Color::Red
    } else {
        props.cell_selection_color
    };

    if done_width > 0 {
        fltk::draw::push_clip(x, y, done_width, height);
        draw_table_cell(
            &row.display_text,
            x,
            y,
            width,
            height,
            Align::Left,
            border_color,
            done_color,
            text_color,
            text_font,
            text_size,
            padding,
        );
        fltk::draw::pop_clip();
    }
    fltk::draw::push_clip(x + done_width, y, width - done_width, height);
    draw_table_cell(
        &row.display_text,
        x,
        y,
        width,
        height,
        Align::Left,
        border_color,
        remaining_color,
        text_color,
        text_font,
        text_size,
        padding,
    );
    fltk::draw::pop_clip();
}

fn lerp(width: i32, done: u64, total: u64) -> i32 {
    ((width as f64) * (done as f64) / (total as f64)) as i32
}
