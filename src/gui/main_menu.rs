use fltk::app;
use fltk::button::Button;
use fltk::dialog;
use fltk::enums::FrameType;
use fltk::group::Column;
use fltk::prelude::*;

use super::alert_not_implemented;

pub(super) struct MainMenu;

impl MainMenu {
    pub(super) fn new(on_continue: impl Fn() -> std::io::Result<()> + 'static) -> Self {
        let mut group = Column::default_fill();
        group.set_margin(10);
        group.set_pad(10);

        let _continue_btn = make_button("Continue", move |_| {
            if let Err(err) = on_continue() {
                dialog::alert_default(&format!("Failed to launch Conan Exiles:\n{}", err))
            }
        });
        let _online_btn = make_button("Online", alert_not_implemented);
        let _singleplayer_btn = make_button("Singleplayer", alert_not_implemented);
        let _coop_btn = make_button("Co-op", alert_not_implemented);
        let _mods_btn = make_button("Mods", alert_not_implemented);
        let _settings_btn = make_button("Settings", alert_not_implemented);
        let _exit_btn = make_button("Exit", |_| app::quit());

        group.end();

        Self
    }
}

fn make_button<F: FnMut(&mut Button) + 'static>(text: &str, callback: F) -> Button {
    let mut button = Button::default().with_label(text);
    button.set_frame(FrameType::RoundUpBox);
    button.set_down_frame(FrameType::RoundDownBox);
    button.clear_visible_focus();
    button.set_callback(callback);
    button
}
