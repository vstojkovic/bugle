use std::cell::RefCell;
use std::rc::Rc;

use fltk::dialog;
use fltk::group::{Group, Row};
use fltk::prelude::*;
use fltk::text::TextDisplay;
use fltk::window::Window;

mod main_menu;
mod server_browser;

use main_menu::MainMenu;
use server_browser::ServerBrowser;

pub struct LauncherWindow {
    window: Window,
}

impl LauncherWindow {
    pub fn new(on_continue: impl Fn() -> std::io::Result<()> + 'static) -> Self {
        let mut window = Window::default().with_size(1280, 760);
        window.set_label("BUGLE");

        let mut main_group = Row::default_fill();

        let mut _main_menu = MainMenu::new();

        let content_group = Group::default_fill();

        let welcome_group = Group::default_fill();
        let _welcome_text = TextDisplay::default()
            .with_label("Welcome to BUGLE: Butt-Ugly Game Launcher For Exiles")
            .center_of_parent();
        welcome_group.end();

        let server_browser = ServerBrowser::new();

        content_group.end();

        main_group.set_size(&_main_menu.group.as_group().unwrap(), 200);
        main_group.end();

        window.end();

        let active_content_group = Rc::new(RefCell::new(welcome_group));

        _main_menu.set_on_continue(on_continue);
        {
            let active_content_group = active_content_group.clone();
            let mut server_browser_group = server_browser.group.clone();
            _main_menu.set_on_online(move || {
                active_content_group.borrow_mut().hide();
                server_browser_group.show();
                active_content_group.replace(server_browser_group.clone());
            });
        }

        Self { window }
    }

    pub fn show(&mut self) {
        self.window.show();
    }
}

fn alert_not_implemented(_: &mut impl WidgetExt) {
    dialog::alert_default("This feature is not yet implemented in the current release.");
}
