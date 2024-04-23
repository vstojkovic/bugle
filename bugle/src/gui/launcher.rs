use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use fltk::app;
use fltk::enums::{Event, FrameType};
use fltk::group::Wizard;
use fltk::prelude::*;
use fltk::window::Window;
use fltk_float::grid::{CellAlign, Grid};
use fltk_float::overlay::OverlayBuilder;
use fltk_float::LayoutElement;
use slog::Logger;

use crate::bus::AppBus;
use crate::config::Config;
use crate::game::platform::ModDirectory;
use crate::game::Game;

use super::home::HomeTab;
use super::main_menu::MainMenu;
use super::mod_manager::ModManagerTab;
use super::server_browser::ServerBrowserTab;
use super::single_player::SinglePlayerTab;
use super::wrapper_factory;
use super::{Action, Handler};

pub struct LauncherWindow {
    window: Window,
    on_action: Rc<RefCell<Box<dyn Handler<Action>>>>,
}

impl LauncherWindow {
    pub fn new(
        logger: Logger,
        bus: &mut AppBus,
        game: Arc<Game>,
        config: &Config,
        mod_resolver: Rc<dyn ModDirectory>,
        log_level_overridden: bool,
        can_switch_branch: bool,
        can_save_servers: bool,
    ) -> Self {
        let on_action: Rc<RefCell<Box<dyn Handler<Action>>>> =
            Rc::new(RefCell::new(Box::new(|_| {
                panic!("Action handler not yet assigned");
            })));

        let mut window = Window::default().with_size(1280, 760).with_label("BUGLE");

        let root = Grid::builder_with_factory(wrapper_factory());
        let mut root = root
            .with_col_spacing(10)
            .with_row_spacing(10)
            .with_padding(10, 10, 10, 10);
        root.row()
            .with_stretch(1)
            .with_default_align(CellAlign::Stretch)
            .add();

        root.col().with_min_size(140).add();
        let (mut main_menu, main_menu_grid) = MainMenu::new();
        root.cell().unwrap().add(main_menu_grid);

        root.col().with_stretch(1).add();

        let mut content_overlay = OverlayBuilder::new(Wizard::default_fill());

        let home = {
            let on_action = Rc::clone(&on_action);
            HomeTab::new(
                logger.clone(),
                bus,
                Arc::clone(&game),
                config,
                log_level_overridden,
                can_switch_branch,
                move |home_action| on_action.borrow()(Action::HomeAction(home_action)),
            )
        };
        content_overlay.add_shared(Rc::<HomeTab>::clone(&home));

        let server_browser = {
            let on_action = Rc::clone(&on_action);
            ServerBrowserTab::new(
                logger.clone(),
                bus,
                Arc::clone(&game),
                &config.server_browser,
                mod_resolver,
                can_save_servers,
                move |browser_action| on_action.borrow()(Action::ServerBrowser(browser_action)),
            )
        };
        content_overlay.add_shared(Rc::<ServerBrowserTab>::clone(&server_browser));

        let single_player = {
            let on_action = Rc::clone(&on_action);
            SinglePlayerTab::new(
                logger.clone(),
                bus,
                Arc::clone(game.maps()),
                move |sp_action| on_action.borrow()(Action::SinglePlayer(sp_action)),
            )
        };
        content_overlay.add_shared(Rc::<SinglePlayerTab>::clone(&single_player));

        let mod_manager = {
            let on_action = Rc::clone(&on_action);
            ModManagerTab::new(
                logger.clone(),
                bus,
                Arc::clone(game.installed_mods()),
                game.branch(),
                move |mod_mgr_action| on_action.borrow()(Action::ModManager(mod_mgr_action)),
            )
        };
        content_overlay.add_shared(Rc::<ModManagerTab>::clone(&mod_manager));

        let content_overlay = content_overlay.end();
        let mut content_group = content_overlay.group();
        content_group.set_frame(FrameType::NoBox);
        root.cell().unwrap().add(content_overlay);

        let root = root.end();
        root.layout_children();

        window.set_callback(|_| {
            if app::event() == Event::Close {
                app::quit();
            }
        });
        let min_size = root.min_size();
        window.size_range(min_size.width, min_size.height, 0, 0);
        window.make_resizable(true);
        window.resize_callback(move |_, _, _, _, _| root.layout_children());

        content_group.set_current_widget(home.root());

        {
            let mut content_group = content_group.clone();
            let home = Rc::clone(&home);
            main_menu.set_on_home(move || content_group.set_current_widget(home.root()));
        }

        {
            let mut content_group = content_group.clone();
            let server_browser = Rc::clone(&server_browser);
            main_menu
                .set_on_online(move || content_group.set_current_widget(server_browser.root()));
        }

        {
            let mut content_group = content_group.clone();
            let single_player = Rc::clone(&single_player);
            main_menu.set_on_single_player(move || {
                content_group.set_current_widget(single_player.root())
            });
        }

        {
            let mut content_group = content_group.clone();
            let mod_manager = Rc::clone(&mod_manager);
            main_menu.set_on_mods(move || content_group.set_current_widget(mod_manager.root()));
        }

        Self { window, on_action }
    }

    pub fn set_on_action(&self, on_action: impl Handler<Action> + 'static) {
        *self.on_action.borrow_mut() = Box::new(on_action);
    }

    pub fn show(&self) {
        self.window.clone().show();
    }

    pub fn window(&self) -> &Window {
        &self.window
    }
}
