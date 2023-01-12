use std::cell::RefCell;
use std::rc::Rc;

use fltk::dialog;
use fltk::group::Group;
use fltk::prelude::*;
use fltk::text::TextDisplay;
use fltk::window::Window;

pub mod glyph;
mod main_menu;
mod mod_manager;
mod prelude;
mod server_browser;

use self::main_menu::MainMenu;
use self::mod_manager::ModManager;
use self::prelude::*;
use self::server_browser::ServerBrowser;

pub use self::mod_manager::{ModManagerAction, ModManagerUpdate};
pub use self::server_browser::{ServerBrowserAction, ServerBrowserUpdate};

pub enum Action {
    Launch,
    Continue,
    ServerBrowser(ServerBrowserAction),
    ModManager(ModManagerAction),
}

pub enum Update {
    ServerBrowser(ServerBrowserUpdate),
    ModManager(ModManagerUpdate),
}

impl Update {
    pub fn try_consolidate(self, other: Self) -> Result<Update, (Update, Update)> {
        match (self, other) {
            (Self::ServerBrowser(this), Self::ServerBrowser(other)) => {
                Self::consolidation_result(this.try_consolidate(other))
            }
            (this, other) => Err((this, other)),
        }
    }

    fn consolidation_result<U: Into<Update>>(
        result: Result<U, (U, U)>,
    ) -> Result<Update, (Update, Update)> {
        match result {
            Ok(consolidated) => Ok(consolidated.into()),
            Err((this, other)) => Err((this.into(), other.into())),
        }
    }
}

pub struct LauncherWindow {
    window: Window,
    server_browser: Rc<ServerBrowser>,
    mod_manager: Rc<ModManager>,
}

pub trait Handler<A>: Fn(A) -> anyhow::Result<()> {}
impl<A, F: Fn(A) -> anyhow::Result<()>> Handler<A> for F {}

type CleanupFn = Box<dyn FnMut()>;

impl LauncherWindow {
    pub fn new(on_action: impl Handler<Action> + 'static) -> Self {
        let on_action: Rc<dyn Handler<Action>> = Rc::new(on_action);

        let mut window = Window::default().with_size(1280, 760);
        window.set_label("BUGLE");

        let root = Group::default_fill();

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
            let on_action = Rc::clone(&on_action);
            ServerBrowser::new(move |browser_action| {
                on_action(Action::ServerBrowser(browser_action))
            })
        };

        let mod_manager = {
            let on_action = Rc::clone(&on_action);
            ModManager::new(move |mod_mgr_action| on_action(Action::ModManager(mod_mgr_action)))
        };

        content_group.end();

        root.end();

        window.end();

        let active_content_cleanup_fn: Rc<RefCell<CleanupFn>> =
            Rc::new(RefCell::new(Box::new(move || {
                welcome_group.hide();
            })));

        {
            let on_action = Rc::clone(&on_action);
            main_menu.set_on_launch(move || on_action(Action::Launch));
        }

        {
            let on_action = Rc::clone(&on_action);
            main_menu.set_on_continue(move || on_action(Action::Continue));
        }

        {
            let old_cleanup = Rc::clone(&active_content_cleanup_fn);
            let server_browser = Rc::clone(&server_browser);
            main_menu.set_on_online(move || {
                switch_content(&old_cleanup, || server_browser.show());
            });
        }

        {
            let old_cleanup = Rc::clone(&active_content_cleanup_fn);
            let mod_manager = Rc::clone(&mod_manager);
            main_menu.set_on_mods(move || {
                switch_content(&old_cleanup, || mod_manager.show());
            });
        }

        Self {
            window,
            server_browser,
            mod_manager,
        }
    }

    pub fn show(&mut self) {
        self.window.show();
    }

    pub fn handle_update(&mut self, update: Update) {
        match update {
            Update::ServerBrowser(update) => self.server_browser.handle_update(update),
            Update::ModManager(update) => self.mod_manager.handle_update(update),
        }
    }
}

pub fn alert_not_implemented() {
    dialog::alert_default("This feature is not yet implemented in the current release.");
}

pub fn alert_error(message: &str, err: &anyhow::Error) {
    dialog::alert_default(&format!("{}\n{}", message, err));
}

fn widget_auto_width<W: WidgetExt + ?Sized>(widget: &W) -> i32 {
    let (w, _) = widget.measure_label();
    w + 20
}

fn widget_auto_height<W: WidgetExt + ?Sized>(widget: &W) -> i32 {
    let (_, h) = widget.measure_label();
    h * 3 / 2
}

fn button_auto_height<B: ButtonExt + ?Sized>(button: &B) -> i32 {
    let (_, h) = button.measure_label();
    h * 14 / 8
}

fn widget_col_width(widgets: &[&dyn WidgetExt]) -> i32 {
    widgets
        .into_iter()
        .map(|widget| widget_auto_width(*widget))
        .max()
        .unwrap()
}

fn button_row_height(buttons: &[&dyn ButtonExt]) -> i32 {
    buttons
        .into_iter()
        .map(|button| button_auto_height(*button))
        .max()
        .unwrap()
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
