use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use fltk::app;
use fltk::dialog;
use fltk::enums::Event;
use fltk::group::Group;
use fltk::prelude::*;
use fltk::window::Window;

mod data;
pub mod glyph;
mod home;
mod main_menu;
mod mod_manager;
mod prelude;
mod server_browser;
mod single_player;

use crate::game::Game;

use self::home::Home;
use self::main_menu::MainMenu;
use self::mod_manager::ModManager;
use self::prelude::*;
use self::server_browser::ServerBrowser;
use self::single_player::SinglePlayer;

pub use self::mod_manager::{ModManagerAction, ModManagerUpdate};
pub use self::server_browser::{ServerBrowserAction, ServerBrowserUpdate};
pub use self::single_player::{SinglePlayerAction, SinglePlayerUpdate};

pub enum Action {
    Launch,
    Continue,
    ServerBrowser(ServerBrowserAction),
    SinglePlayer(SinglePlayerAction),
    ModManager(ModManagerAction),
}

pub enum Update {
    ServerBrowser(ServerBrowserUpdate),
    SinglePlayer(SinglePlayerUpdate),
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
    cleanup_fn: Rc<RefCell<CleanupFn>>,
    home: Rc<Home>,
    server_browser: Rc<ServerBrowser>,
    single_player: Rc<SinglePlayer>,
    mod_manager: Rc<ModManager>,
}

pub trait Handler<A>: Fn(A) -> anyhow::Result<()> {}
impl<A, F: Fn(A) -> anyhow::Result<()>> Handler<A> for F {}

type CleanupFn = Box<dyn FnMut()>;

impl LauncherWindow {
    pub fn new(game: &Game, on_action: impl Handler<Action> + 'static) -> Self {
        let on_action: Rc<dyn Handler<Action>> = Rc::new(on_action);

        let mut window = Window::default().with_size(1280, 760);
        window.set_label("BUGLE");
        window.set_callback(|_| {
            if app::event() == Event::Close {
                app::quit();
            }
        });

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

        let home = {
            let on_action = Rc::clone(&on_action);
            Home::new(game, move |action| on_action(action))
        };

        let server_browser = {
            let on_action = Rc::clone(&on_action);
            ServerBrowser::new(Arc::clone(game.maps()), move |browser_action| {
                on_action(Action::ServerBrowser(browser_action))
            })
        };

        let single_player = {
            let on_action = Rc::clone(&on_action);
            SinglePlayer::new(Arc::clone(game.maps()), move |sp_action| {
                on_action(Action::SinglePlayer(sp_action))
            })
        };

        let mod_manager = {
            let on_action = Rc::clone(&on_action);
            ModManager::new(Arc::clone(game.installed_mods()), move |mod_mgr_action| {
                on_action(Action::ModManager(mod_mgr_action))
            })
        };

        content_group.end();

        root.end();

        window.end();

        let cleanup_fn: Rc<RefCell<CleanupFn>> = Rc::new(RefCell::new(Box::new(|| ())));

        {
            let old_cleanup = Rc::clone(&cleanup_fn);
            let home = Rc::clone(&home);
            main_menu.set_on_home(move || switch_content(&old_cleanup, || home.show()));
        }

        {
            let old_cleanup = Rc::clone(&cleanup_fn);
            let server_browser = Rc::clone(&server_browser);
            main_menu.set_on_online(move || {
                switch_content(&old_cleanup, || server_browser.show());
            });
        }

        {
            let old_cleanup = Rc::clone(&cleanup_fn);
            let single_player = Rc::clone(&single_player);
            main_menu.set_on_single_player(move || {
                switch_content(&old_cleanup, || single_player.show());
            });
        }

        {
            let old_cleanup = Rc::clone(&cleanup_fn);
            let mod_manager = Rc::clone(&mod_manager);
            main_menu.set_on_mods(move || {
                switch_content(&old_cleanup, || mod_manager.show());
            });
        }

        Self {
            window,
            cleanup_fn,
            home,
            server_browser,
            single_player,
            mod_manager,
        }
    }

    pub fn show(&mut self) {
        switch_content(&self.cleanup_fn, || self.home.show());
        self.window.show();
    }

    pub fn handle_update(&mut self, update: Update) {
        match update {
            Update::ServerBrowser(update) => self.server_browser.handle_update(update),
            Update::SinglePlayer(update) => self.single_player.handle_update(update),
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

pub fn prompt_confirm(prompt: &str) -> bool {
    dialog::choice2_default(prompt, "No", "Yes", "")
        .map(|choice| choice == 1)
        .unwrap_or_default()
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

fn is_table_nav_event() -> bool {
    match app::event() {
        Event::KeyDown => true,
        Event::Released => app::event_is_click(),
        _ => false,
    }
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
