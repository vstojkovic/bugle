use std::rc::Rc;

use chrono::Weekday;
use fltk::app;
use fltk::button::{Button, CheckButton, LightButton, RadioButton, ReturnButton};
use fltk::dialog as fltk_dialog;
use fltk::enums::{Color, Event};
use fltk::frame::Frame;
use fltk::input::{Input, SecretInput};
use fltk::menu::MenuButton;
use fltk::misc::InputChoice;
use fltk_float::button::{ButtonElement, FramelessButtonElement, MenuButtonElement};
use fltk_float::frame::FrameElement;
use fltk_float::input::InputElement;
use fltk_float::misc::InputChoiceElement;
use fltk_float::WrapperFactory;

mod assets;
mod data;
mod dialog;
pub mod glyph;
mod home;
mod launcher;
mod main_menu;
mod mod_manager;
mod mod_update;
mod prelude;
mod server_browser;
mod server_settings;
mod single_player;
mod svg_symbol;
pub mod theme;
mod widgets;

pub use self::dialog::Dialog;
pub use self::home::{HomeAction, HomeUpdate};
pub use self::launcher::LauncherWindow;
pub use self::mod_manager::{ModManagerAction, ModManagerUpdate};
pub use self::mod_update::{ModUpdateProgressDialog, ModUpdateSelectionDialog};
pub use self::server_browser::{ServerBrowserAction, ServerBrowserUpdate};
pub use self::server_settings::ServerSettingsDialog;
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

pub fn alert_error(message: &str, err: &anyhow::Error) {
    fltk_dialog::alert_default(&format!("{}\n{}", message, err));
}

pub fn prompt_confirm(prompt: &str) -> bool {
    fltk_dialog::choice2_default(prompt, "No", "Yes", "")
        .map(|choice| choice == 1)
        .unwrap_or_default()
}

thread_local! {
    static WRAPPER_FACTORY: Rc<WrapperFactory> = {
        use self::widgets::{DropDownList, DropDownListElement, ReadOnlyText, ReadOnlyTextElement};
        let mut factory = WrapperFactory::new();
        factory.set_wrapper::<Button, ButtonElement<Button>>();
        factory.set_wrapper::<CheckButton, FramelessButtonElement<CheckButton>>();
        factory.set_wrapper::<DropDownList, DropDownListElement>();
        factory.set_wrapper::<Frame, FrameElement>();
        factory.set_wrapper::<Input, InputElement<Input>>();
        factory.set_wrapper::<InputChoice, InputChoiceElement>();
        factory.set_wrapper::<LightButton, ButtonElement<LightButton>>();
        factory.set_wrapper::<MenuButton, MenuButtonElement>();
        factory.set_wrapper::<RadioButton, ButtonElement<RadioButton>>();
        factory.set_wrapper::<ReadOnlyText, ReadOnlyTextElement>();
        factory.set_wrapper::<ReturnButton, ButtonElement<ReturnButton>>();
        factory.set_wrapper::<SecretInput, InputElement<SecretInput>>();
        Rc::new(factory)
    }
}

fn wrapper_factory() -> Rc<WrapperFactory> {
    WRAPPER_FACTORY.with(|factory| Rc::clone(factory))
}

fn is_table_nav_event() -> bool {
    match app::event() {
        Event::KeyDown => true,
        Event::Released => app::event_is_click(),
        _ => false,
    }
}

fn color_rgb(color: Color) -> u32 {
    let (r, g, b) = color.to_rgb();
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

fn weekday_name(weekday: Weekday) -> &'static str {
    match weekday {
        Weekday::Mon => "Mon",
        Weekday::Tue => "Tue",
        Weekday::Wed => "Wed",
        Weekday::Thu => "Thu",
        Weekday::Fri => "Fri",
        Weekday::Sat => "Sat",
        Weekday::Sun => "Sun",
    }
}
