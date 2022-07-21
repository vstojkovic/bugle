use fltk::dialog;
use fltk::prelude::*;
use fltk::window::Window;

mod main_menu;

use main_menu::MainMenu;

pub struct LauncherWindow {
    window: Window,
}

impl LauncherWindow {
    pub fn new(on_continue: impl Fn() -> std::io::Result<()> + 'static) -> Self {
        let mut window = Window::default().with_size(400, 300);
        window.set_label("BUGLE");

        let _main_menu = MainMenu::new(on_continue);

        window.end();

        Self { window }
    }

    pub fn show(&mut self) {
        self.window.show();
    }
}

fn alert_not_implemented(_: &mut impl WidgetExt) {
    dialog::alert_default("This feature is not yet implemented in the current release.");
}
