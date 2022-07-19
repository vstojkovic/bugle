use fltk::app::{self, App};
use fltk::button::Button;
use fltk::dialog;
use fltk::enums::FrameType;
use fltk::group::{Pack, PackType};
use fltk::prelude::*;
use fltk::window::Window;

fn make_button<F: FnMut(&mut Button) + 'static>(text: &str, callback: F) -> Button {
    let mut button = Button::default().with_label(text);
    button.set_frame(FrameType::RoundUpBox);
    button.set_down_frame(FrameType::RoundDownBox);
    button.clear_visible_focus();
    button.set_callback(callback);
    button
}

fn not_implemented(_: &mut Button) {
    dialog::alert_default("This feature is not yet implemented in the current release.");
}

fn main() {
    let launcher = App::default();

    let mut main_win = Window::default().with_size(400, 300);
    main_win.set_label("BUGLE");

    let mut vpack = Pack::default_fill();
    vpack.set_type(PackType::Vertical);
    vpack.set_spacing(10);

    let _continue_btn = make_button("Continue", not_implemented);
    let _online_btn = make_button("Online", not_implemented);
    let _sp_btn = make_button("Singleplayer", not_implemented);
    let _coop_btn = make_button("Co-op", not_implemented);
    let _mods_btn = make_button("Mods", not_implemented);
    let _settings_btn = make_button("Settings", not_implemented);
    let _exit_btn = make_button("Exit", |_| { app::quit(); });

    vpack.end();
    vpack.auto_layout();

    main_win.end();
    main_win.show();

    launcher.run().unwrap();
}
