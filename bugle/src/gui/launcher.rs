use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::AtomicUsize;
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

use crate::auth_manager::AuthManager;
use crate::bus::AppBus;
use crate::config::ConfigManager;
use crate::game::Game;
use crate::launcher::Launcher;
use crate::mod_manager::ModManager;
use crate::saved_games_manager::SavedGamesManager;
use crate::server_manager::ServerManager;

use super::home::HomeTab;
use super::main_menu::MainMenu;
use super::mod_manager::ModManagerTab;
use super::server_browser::ServerBrowserTab;
use super::single_player::SinglePlayerTab;
use super::wrapper_factory;

pub struct LauncherWindow {
    window: Window,
}

impl LauncherWindow {
    pub fn new(
        logger: &Logger,
        bus: Rc<RefCell<AppBus>>,
        game: Arc<Game>,
        config: Rc<ConfigManager>,
        log_level: Option<Arc<AtomicUsize>>,
        auth: Rc<AuthManager>,
        launcher: Rc<Launcher>,
        servers: Rc<ServerManager>,
        saves: Rc<SavedGamesManager>,
        mod_manager: Rc<ModManager>,
        can_switch_branch: bool,
    ) -> Self {
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

        let home_tab = HomeTab::new(
            logger,
            Rc::clone(&bus),
            Arc::clone(&game),
            Rc::clone(&config),
            log_level,
            Rc::clone(&auth),
            Rc::clone(&launcher),
            can_switch_branch,
        );
        content_overlay.add_shared(Rc::<HomeTab>::clone(&home_tab));

        let server_browser_tab = ServerBrowserTab::new(
            logger,
            Rc::clone(&bus),
            Arc::clone(&game),
            Rc::clone(&config),
            Rc::clone(&launcher),
            Rc::clone(&servers),
            Rc::clone(&mod_manager),
        );
        content_overlay.add_shared(Rc::<ServerBrowserTab>::clone(&server_browser_tab));

        let single_player_tab = {
            SinglePlayerTab::new(
                logger,
                Rc::clone(&bus),
                Arc::clone(&game),
                Rc::clone(&launcher),
                Rc::clone(&saves),
            )
        };
        content_overlay.add_shared(Rc::<SinglePlayerTab>::clone(&single_player_tab));

        let mod_manager_tab =
            ModManagerTab::new(logger, Arc::clone(&game), Rc::clone(&mod_manager));
        content_overlay.add_shared(Rc::<ModManagerTab>::clone(&mod_manager_tab));

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

        content_group.set_current_widget(home_tab.root());

        {
            let mut content_group = content_group.clone();
            let home_tab = Rc::clone(&home_tab);
            main_menu.set_on_home(move || content_group.set_current_widget(home_tab.root()));
        }

        {
            let mut content_group = content_group.clone();
            let server_browser_tab = Rc::clone(&server_browser_tab);
            main_menu
                .set_on_online(move || content_group.set_current_widget(server_browser_tab.root()));
        }

        {
            let mut content_group = content_group.clone();
            let single_player_tab = Rc::clone(&single_player_tab);
            main_menu.set_on_single_player(move || {
                content_group.set_current_widget(single_player_tab.root())
            });
        }

        {
            let mut content_group = content_group.clone();
            let mod_manager_tab = Rc::clone(&mod_manager_tab);
            main_menu.set_on_mods(move || content_group.set_current_widget(mod_manager_tab.root()));
        }

        Self { window }
    }

    pub fn show(&self) {
        self.window.clone().show();
    }

    pub fn window(&self) -> &Window {
        &self.window
    }
}
