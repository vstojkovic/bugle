use std::cell::RefCell;
use std::rc::Rc;

use fltk::dialog;
use fltk::group::{Group, Row};
use fltk::prelude::*;
use fltk::text::TextDisplay;
use fltk::window::Window;

mod main_menu;
mod server_browser;

use self::main_menu::MainMenu;
use self::server_browser::{ServerBrowser, ServerBrowserAction};

pub enum Action {
    Continue,
    ServerBrowser(ServerBrowserAction),
}

pub struct LauncherWindow {
    window: Window,
}

pub trait ActionHandler: Fn(Action) -> anyhow::Result<()> {}
impl<F: Fn(Action) -> anyhow::Result<()>> ActionHandler for F {}

type CleanupFn = Box<dyn FnMut() -> Option<Action>>;

impl LauncherWindow {
    pub fn new(on_action: impl ActionHandler + 'static) -> Self {
        let on_action: Rc<dyn ActionHandler> = Rc::new(on_action);

        let mut window = Window::default().with_size(1280, 760);
        window.set_label("BUGLE");

        let mut main_group = Row::default_fill();

        let mut _main_menu = MainMenu::new();

        let content_group = Group::default_fill();

        let mut welcome_group = Group::default_fill();
        let _welcome_text = TextDisplay::default()
            .with_label("Welcome to BUGLE: Butt-Ugly Game Launcher For Exiles")
            .center_of_parent();
        welcome_group.end();

        let mut server_browser = ServerBrowser::new(on_action.clone());

        content_group.end();

        main_group.set_size(&_main_menu.group.as_group().unwrap(), 200);
        main_group.end();

        window.end();

        let active_content_cleanup_fn: Rc<RefCell<CleanupFn>> =
            Rc::new(RefCell::new(Box::new(move || {
                welcome_group.hide();
                None
            })));

        {
            let on_action = on_action.clone();
            _main_menu.set_on_continue(move || on_action(Action::Continue));
        }

        {
            let old_cleanup = active_content_cleanup_fn.clone();
            let on_action = on_action.clone();
            _main_menu.set_on_online(move || {
                switch_content(&old_cleanup, &on_action, || server_browser.show());
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

fn switch_content(
    old_cleanup_fn: &Rc<RefCell<CleanupFn>>,
    on_action: &Rc<dyn ActionHandler>,
    mut show_new_content_fn: impl FnMut() -> CleanupFn,
) {
    if let Some(cleanup_action) = old_cleanup_fn.borrow_mut()() {
        if let Err(err) = on_action(cleanup_action) {
            dialog::alert_default(&format!("{}", err)); // FIXME
        }
    };
    let _ = old_cleanup_fn.replace(show_new_content_fn());
}
