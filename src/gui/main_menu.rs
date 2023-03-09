use fltk::app;
use fltk::button::{Button, RadioButton};
use fltk::enums::FrameType;
use fltk::group::Group;
use fltk::prelude::*;

use super::not_implemented_callback;
use super::prelude::LayoutExt;

pub(super) struct MainMenu {
    home_btn: RadioButton,
    online_btn: RadioButton,
    single_player_btn: RadioButton,
    mods_btn: RadioButton,
}

impl MainMenu {
    pub fn new() -> Self {
        let group = Group::default_fill();

        let mut home_btn = make_button(RadioButton::default_fill, "Launcher");
        let online_btn = make_button(RadioButton::default_fill, "Online");
        let single_player_btn = make_button(RadioButton::default_fill, "Singleplayer");
        let mut coop_btn = make_button(Button::default_fill, "Co-op");
        let mods_btn = make_button(RadioButton::default_fill, "Mods");
        let mut exit_btn = make_button(Button::default_fill, "Exit");

        home_btn.toggle(true);

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

        coop_btn.set_callback(not_implemented_callback);
        exit_btn.set_callback(|_| app::quit());

        Self {
            home_btn,
            online_btn,
            single_player_btn,
            mods_btn,
        }
    }

    pub fn set_on_home(&mut self, mut on_home: impl FnMut() + 'static) {
        self.home_btn.set_callback(move |_| on_home());
    }

    pub fn set_on_online(&mut self, mut on_online: impl FnMut() + 'static) {
        self.online_btn.set_callback(move |_| on_online());
    }

    pub fn set_on_single_player(&mut self, mut on_single_player: impl FnMut() + 'static) {
        self.single_player_btn
            .set_callback(move |_| on_single_player());
    }

    pub fn set_on_mods(&mut self, mut on_mods: impl FnMut() + 'static) {
        self.mods_btn.set_callback(move |_| on_mods());
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
