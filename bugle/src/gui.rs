use std::rc::Rc;

use chrono::Weekday;
use fltk::app;
use fltk::button::{Button, CheckButton, LightButton, RadioButton, ReturnButton, ToggleButton};
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
mod task_progress_monitor;
pub mod theme;
mod widgets;

pub use self::dialog::Dialog;
pub use self::home::{UpdateAuthState, UpdateLastSession};
pub use self::launcher::LauncherWindow;
pub use self::mod_update::{ModUpdateProgressDialog, ModUpdateSelectionDialog};
pub use self::server_browser::{PopulateServers, ProcessPongs, RefreshServerDetails, UpdateServer};
pub use self::single_player::PopulateSinglePlayerGames;
pub use self::task_progress_monitor::{TaskProgressMonitor, TaskProgressUpdate};

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
        factory.set_wrapper::<ToggleButton, ButtonElement<ToggleButton>>();
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

fn min_input_width(samples: &[&str]) -> i32 {
    fltk::draw::set_font(fltk::enums::Font::Helvetica, fltk::app::font_size());
    samples
        .into_iter()
        .map(|text| fltk::draw::measure(&format!("#{}#", text), false).0)
        .max()
        .unwrap_or_default()
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
