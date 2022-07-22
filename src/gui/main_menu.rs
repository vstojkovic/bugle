use fltk::app;
use fltk::button::{Button, RadioButton};
use fltk::dialog;
use fltk::enums::FrameType;
use fltk::group::Column;
use fltk::prelude::*;

use super::alert_not_implemented;

pub(super) struct MainMenu {
    pub(super) group: Column,
    continue_btn: Button,
    online_btn: RadioButton,
}

impl MainMenu {
    pub(super) fn new() -> Self {
        let mut group = Column::default_fill();
        group.set_margin(10);
        group.set_pad(10);

        let continue_btn = make_button(Button::default, "Continue");
        let online_btn = make_button(RadioButton::default, "Online");
        let mut singleplayer_btn = make_button(Button::default, "Singleplayer");
        let mut coop_btn = make_button(Button::default, "Co-op");
        let mut mods_btn = make_button(Button::default, "Mods");
        let mut settings_btn = make_button(Button::default, "Settings");
        let mut exit_btn = make_button(Button::default, "Exit");

        group.end();

        singleplayer_btn.set_callback(alert_not_implemented);
        coop_btn.set_callback(alert_not_implemented);
        mods_btn.set_callback(alert_not_implemented);
        settings_btn.set_callback(alert_not_implemented);
        exit_btn.set_callback(|_| app::quit());

        Self {
            group,
            continue_btn,
            online_btn,
        }
    }

    pub(super) fn set_on_continue(
        &mut self,
        on_continue: impl Fn() -> std::io::Result<()> + 'static,
    ) {
        self.continue_btn.set_callback(move |_| {
            if let Err(err) = on_continue() {
                dialog::alert_default(&format!("Failed to launch Conan Exiles:\n{}", err))
            }
        });
    }

    pub(super) fn set_on_online(&mut self, mut on_online: impl FnMut() + 'static) {
        self.online_btn.set_callback(move |_| on_online());
    }
}

fn make_button<B, C>(ctor: C, text: &str) -> B
where
    B: ButtonExt,
    C: FnOnce() -> B,
{
    let mut button = ctor().with_label(text);
    button.set_frame(FrameType::PlasticThinUpBox);
    button.set_down_frame(FrameType::PlasticThinDownBox);
    button.clear_visible_focus();
    button
}
