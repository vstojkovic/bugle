use std::cell::RefCell;
use std::rc::Rc;

use fltk::dialog;
use fltk::group::Group;
use fltk::prelude::*;
use fltk::text::TextDisplay;
use fltk::window::Window;

mod main_menu;
mod prelude;
mod server_browser;

use self::main_menu::MainMenu;
use self::prelude::*;
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

        let root_group = Group::default_fill();

        let main_menu_group = Group::default()
            .inside_parent(10, 10)
            .size_of_parent()
            .with_size_flex(200, -20);
        let mut main_menu = MainMenu::new();
        main_menu_group.end();

        let content_group = Group::default_fill()
            .right_of(&main_menu_group, 10)
            .stretch_to_parent(10, 10);

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

        root_group.end();

        window.end();

        let active_content_cleanup_fn: Rc<RefCell<CleanupFn>> =
            Rc::new(RefCell::new(Box::new(move || {
                welcome_group.hide();
            })));

        {
            let on_action = on_action.clone();
            main_menu.set_on_continue(move || on_action(Action::Continue));
        }

        {
            let old_cleanup = active_content_cleanup_fn.clone();
            let server_browser = server_browser.clone();
            main_menu.set_on_online(move || {
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

pub fn alert_not_implemented() {
    dialog::alert_default("This feature is not yet implemented in the current release.");
}

pub fn alert_error(message: &str, err: &anyhow::Error) {
    dialog::alert_default(&format!("{}\n{}", message, err));
}

fn not_implemented_callback(_: &mut impl WidgetExt) {
    alert_not_implemented()
}

fn switch_content(
    old_cleanup_fn: &Rc<RefCell<CleanupFn>>,
    mut show_new_content_fn: impl FnMut() -> CleanupFn,
) {
    old_cleanup_fn.borrow_mut()();
    let _ = old_cleanup_fn.replace(show_new_content_fn());
}
