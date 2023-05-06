use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use fltk::app;
use fltk::enums::Event;
use fltk::group::Group;
use fltk::prelude::*;
use fltk::window::Window;
use slog::Logger;

use crate::config::Config;
use crate::game::Game;

use super::home::Home;
use super::main_menu::MainMenu;
use super::mod_manager::ModManager;
use super::prelude::*;
use super::server_browser::ServerBrowser;
use super::single_player::SinglePlayer;
use super::{Action, CleanupFn, Handler, Update};

pub struct LauncherWindow {
    window: Window,
    on_action: Rc<RefCell<Box<dyn Handler<Action>>>>,
    cleanup_fn: Rc<RefCell<CleanupFn>>,
    home: Rc<Home>,
    server_browser: Rc<ServerBrowser>,
    single_player: Rc<SinglePlayer>,
    mod_manager: Rc<ModManager>,
}

impl LauncherWindow {
    pub fn new(logger: Logger, game: &Game, config: &Config, log_level_overridden: bool) -> Self {
        let on_action: Rc<RefCell<Box<dyn Handler<Action>>>> =
            Rc::new(RefCell::new(Box::new(|_| {
                panic!("Action handler not yet assigned");
            })));

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
            Home::new(
                logger.clone(),
                game,
                config,
                log_level_overridden,
                move |action| on_action.borrow()(action),
            )
        };

        let server_browser = {
            let on_action = Rc::clone(&on_action);
            ServerBrowser::new(
                logger.clone(),
                Arc::clone(game.maps()),
                &config.server_browser,
                move |browser_action| on_action.borrow()(Action::ServerBrowser(browser_action)),
            )
        };

        let single_player = {
            let on_action = Rc::clone(&on_action);
            SinglePlayer::new(logger.clone(), Arc::clone(game.maps()), move |sp_action| {
                on_action.borrow()(Action::SinglePlayer(sp_action))
            })
        };

        let mod_manager = {
            let on_action = Rc::clone(&on_action);
            ModManager::new(
                logger.clone(),
                Arc::clone(game.installed_mods()),
                move |mod_mgr_action| on_action.borrow()(Action::ModManager(mod_mgr_action)),
            )
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
            on_action,
            cleanup_fn,
            home,
            server_browser,
            single_player,
            mod_manager,
        }
    }

    pub fn set_on_action(&self, on_action: impl Handler<Action> + 'static) {
        *self.on_action.borrow_mut() = Box::new(on_action);
    }

    pub fn show(&self) {
        switch_content(&self.cleanup_fn, || self.home.show());
        self.window.clone().show();
    }

    pub fn handle_update(&self, update: Update) {
        match update {
            Update::ServerBrowser(update) => self.server_browser.handle_update(update),
            Update::SinglePlayer(update) => self.single_player.handle_update(update),
            Update::ModManager(update) => self.mod_manager.handle_update(update),
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }
}

fn switch_content(
    old_cleanup_fn: &Rc<RefCell<CleanupFn>>,
    mut show_new_content_fn: impl FnMut() -> CleanupFn,
) {
    old_cleanup_fn.borrow_mut()();
    let _ = old_cleanup_fn.replace(show_new_content_fn());
}
