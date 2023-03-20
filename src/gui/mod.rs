use fltk::app;
use fltk::dialog;
use fltk::enums::Event;
use fltk::prelude::*;

mod data;
pub mod glyph;
mod home;
mod launcher;
mod main_menu;
mod mod_manager;
mod prelude;
mod server_browser;
mod single_player;

use crate::config::BattlEyeUsage;

pub use self::launcher::LauncherWindow;
pub use self::mod_manager::{ModManagerAction, ModManagerUpdate};
pub use self::server_browser::{ServerBrowserAction, ServerBrowserUpdate};
pub use self::single_player::{SinglePlayerAction, SinglePlayerUpdate};

pub enum Action {
    Launch,
    Continue,
    ConfigureBattlEye(BattlEyeUsage),
    ServerBrowser(ServerBrowserAction),
    SinglePlayer(SinglePlayerAction),
    ModManager(ModManagerAction),
}

pub enum Update {
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

pub fn alert_error(message: &str, err: &anyhow::Error) {
    dialog::alert_default(&format!("{}\n{}", message, err));
}

pub fn prompt_confirm(prompt: &str) -> bool {
    dialog::choice2_default(prompt, "No", "Yes", "")
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
