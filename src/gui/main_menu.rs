use fltk::app;
use fltk::button::{Button, RadioButton};
use fltk::dialog;
use fltk::enums::FrameType;
use fltk::group::Group;
use fltk::prelude::*;

use super::not_implemented_callback;
use super::prelude::LayoutExt;

pub(super) struct MainMenu {
    continue_btn: Button,
    online_btn: RadioButton,
}

impl MainMenu {
    pub(super) fn new() -> Self {
        let group = Group::default_fill();

        let continue_btn = make_button(Button::default_fill, "Continue");
        let online_btn = make_button(RadioButton::default_fill, "Online");
        let mut singleplayer_btn = make_button(Button::default_fill, "Singleplayer");
        let mut coop_btn = make_button(Button::default_fill, "Co-op");
        let mut mods_btn = make_button(Button::default_fill, "Mods");
        let mut settings_btn = make_button(Button::default_fill, "Settings");
        let mut exit_btn = make_button(Button::default_fill, "Exit");

        let btn_count = group.children();
        let btn_height = (group.h() - (btn_count - 1) * 10) / btn_count;
        group
            .child(0)
            .unwrap()
            .with_size_flex(0, btn_height)
            .inside_parent(0, 0);
        for idx in 1..btn_count {
            let prev = group.child(idx - 1).unwrap();
            group.child(idx).unwrap().size_of(&prev).below_of(&prev, 10);
        }

        group.end();

        singleplayer_btn.set_callback(not_implemented_callback);
        coop_btn.set_callback(not_implemented_callback);
        mods_btn.set_callback(not_implemented_callback);
        settings_btn.set_callback(not_implemented_callback);
        exit_btn.set_callback(|_| app::quit());

        Self {
            continue_btn,
            online_btn,
        }
    }

    pub(super) fn set_on_continue(
        &mut self,
        on_continue: impl Fn() -> anyhow::Result<()> + 'static,
    ) {
        self.continue_btn.set_callback(move |_| {
            // FIXME: Change this to alert_error
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
