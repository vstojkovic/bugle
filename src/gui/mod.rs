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
use self::server_browser::ServerBrowser;

pub use self::server_browser::{ServerBrowserAction, ServerBrowserUpdate};

pub enum Action {
    Continue,
    ServerBrowser(ServerBrowserAction),
}

pub enum Update {
    ServerBrowser(ServerBrowserUpdate),
}

pub struct LauncherWindow {
    window: Window,
    server_browser: Rc<RefCell<ServerBrowser>>,
}

pub trait Handler<A>: Fn(A) -> anyhow::Result<()> {}
impl<A, F: Fn(A) -> anyhow::Result<()>> Handler<A> for F {}

type CleanupFn = Box<dyn FnMut()>;

impl LauncherWindow {
    pub fn new(on_action: impl Handler<Action> + 'static) -> Self {
        let on_action: Rc<dyn Handler<Action>> = Rc::new(on_action);

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

        let server_browser = {
            let on_action = on_action.clone();
            ServerBrowser::new(move |browser_action| {
                on_action(Action::ServerBrowser(browser_action))
            })
        };

        content_group.end();

        main_group.set_size(&_main_menu.group.as_group().unwrap(), 200);
        main_group.end();

        window.end();

        let active_content_cleanup_fn: Rc<RefCell<CleanupFn>> =
            Rc::new(RefCell::new(Box::new(move || {
                welcome_group.hide();
            })));

        {
            let on_action = on_action.clone();
            _main_menu.set_on_continue(move || on_action(Action::Continue));
        }

        {
            let old_cleanup = active_content_cleanup_fn.clone();
            let server_browser = server_browser.clone();
            _main_menu.set_on_online(move || {
                switch_content(&old_cleanup, || server_browser.borrow_mut().show());
            });
        }

        Self {
            window,
            server_browser,
        }
    }

    pub fn show(&mut self) {
        self.window.show();
    }

    pub fn handle_update(&mut self, update: Update) {
        match update {
            Update::ServerBrowser(update) => self.server_browser.borrow_mut().handle_update(update),
        }
    }
}

fn alert_not_implemented(_: &mut impl WidgetExt) {
    dialog::alert_default("This feature is not yet implemented in the current release.");
}

fn alert_error(message: &str, err: &anyhow::Error) {
    dialog::alert_default(&format!("{}\n{}", message, err));
}

fn switch_content(
    old_cleanup_fn: &Rc<RefCell<CleanupFn>>,
    mut show_new_content_fn: impl FnMut() -> CleanupFn,
) {
    old_cleanup_fn.borrow_mut()();
    let _ = old_cleanup_fn.replace(show_new_content_fn());
}
