use std::cell::RefCell;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

use fltk::app;
use fltk::dialog as fltk_dialog;
use fltk::enums::Event;
use fltk::prelude::*;
use fltk::table::TableContext;
use fltk::text::{Cursor, TextBuffer, TextEditor};
use fltk_table::SmartTable;

mod data;
mod dialog;
pub mod glyph;
mod home;
mod launcher;
mod main_menu;
mod mod_manager;
mod prelude;
mod server_browser;
mod single_player;

pub use self::dialog::Dialog;
pub use self::home::{HomeAction, HomeUpdate};
pub use self::launcher::LauncherWindow;
pub use self::mod_manager::{ModManagerAction, ModManagerUpdate};
pub use self::server_browser::{ServerBrowserAction, ServerBrowserUpdate};
pub use self::single_player::{SinglePlayerAction, SinglePlayerUpdate};

pub enum Action {
    HomeAction(HomeAction),
    ServerBrowser(ServerBrowserAction),
    SinglePlayer(SinglePlayerAction),
    ModManager(ModManagerAction),
}

pub enum Update {
    HomeUpdate(HomeUpdate),
    ServerBrowser(ServerBrowserUpdate),
    SinglePlayer(SinglePlayerUpdate),
    ModManager(ModManagerUpdate),
}

impl Update {
    pub fn try_consolidate(self, other: Self) -> Result<Update, (Update, Update)> {
        match (self, other) {
            (Self::ServerBrowser(this), Self::ServerBrowser(other)) => {
                Self::consolidation_result(this.try_consolidate(other))
            }
            (this, other) => Err((this, other)),
        }
    }

    fn consolidation_result<U: Into<Update>>(
        result: Result<U, (U, U)>,
    ) -> Result<Update, (Update, Update)> {
        match result {
            Ok(consolidated) => Ok(consolidated.into()),
            Err((this, other)) => Err((this.into(), other.into())),
        }
    }
}

pub trait Handler<A>: Fn(A) -> anyhow::Result<()> {}
impl<A, F: Fn(A) -> anyhow::Result<()>> Handler<A> for F {}

type CleanupFn = Box<dyn FnMut()>;

#[derive(Clone)]
pub struct ReadOnlyText {
    editor: TextEditor,
    value: Rc<RefCell<String>>,
}

impl ReadOnlyText {
    pub fn new(initial_value: String) -> Self {
        let mut buffer = TextBuffer::default();
        buffer.set_text(&initial_value);

        let mut editor = TextEditor::default();
        editor.set_buffer(buffer.clone());
        editor.show_cursor(true);
        editor.set_cursor_style(Cursor::Simple);

        let value = Rc::new(RefCell::new(initial_value));
        {
            let mut editor = editor.clone();
            let mut buffer = buffer.clone();
            let value = Rc::clone(&value);
            buffer
                .clone()
                .add_modify_callback(move |pos, ins, del, _, _| {
                    if (ins > 0) || (del > 0) {
                        if let Ok(value) = value.try_borrow_mut() {
                            buffer.set_text(&value);
                            editor.set_insert_position(pos);
                        }
                    }
                });
        }

        Self { editor, value }
    }

    pub fn widget(&self) -> &TextEditor {
        &self.editor
    }

    pub fn set_value(&self, value: String) {
        let mut value_ref = self.value.borrow_mut();
        self.editor.buffer().unwrap().set_text(&value);
        *value_ref = value;
    }
}

impl Default for ReadOnlyText {
    fn default() -> Self {
        Self::new(String::new())
    }
}

impl Deref for ReadOnlyText {
    type Target = TextEditor;
    fn deref(&self) -> &Self::Target {
        &self.editor
    }
}

impl DerefMut for ReadOnlyText {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.editor
    }
}

pub fn alert_error(message: &str, err: &anyhow::Error) {
    fltk_dialog::alert_default(&format!("{}\n{}", message, err));
}

pub fn prompt_confirm(prompt: &str) -> bool {
    fltk_dialog::choice2_default(prompt, "No", "Yes", "")
        .map(|choice| choice == 1)
        .unwrap_or_default()
}

fn widget_auto_width<W: WidgetExt + ?Sized>(widget: &W) -> i32 {
    let (w, _) = widget.measure_label();
    w + 20
}

fn widget_auto_height<W: WidgetExt + ?Sized>(widget: &W) -> i32 {
    let (_, h) = widget.measure_label();
    h * 3 / 2
}

fn button_auto_height<B: ButtonExt + ?Sized>(button: &B) -> i32 {
    let (_, h) = button.measure_label();
    h * 14 / 8
}

fn widget_col_width(widgets: &[&dyn WidgetExt]) -> i32 {
    widgets
        .into_iter()
        .map(|widget| widget_auto_width(*widget))
        .max()
        .unwrap()
}

fn button_row_height(buttons: &[&dyn ButtonExt]) -> i32 {
    buttons
        .into_iter()
        .map(|button| button_auto_height(*button))
        .max()
        .unwrap()
}

fn is_table_nav_event() -> bool {
    match app::event() {
        Event::KeyDown => true,
        Event::Released => app::event_is_click(),
        _ => false,
    }
}

fn make_readonly_cell_widget(table: &SmartTable) -> ReadOnlyText {
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
                        let cell_value = table.cell_value(row, col);
                        let cell_value_len = cell_value.len();
                        cell.set_value(cell_value);
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
