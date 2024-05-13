use std::cell::{Cell, Ref, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use anyhow::Result;
use dynabus::Bus;
use fltk::enums::{Align, Event};
use fltk::frame::Frame;
use fltk::group::{Group, Tile};
use fltk::prelude::*;
use fltk_float::grid::{CellAlign, Grid, GridBuilder};
use fltk_float::overlay::Overlay;
use fltk_float::{LayoutElement, SimpleWrapper, WrapperFactory};
use slog::{error, warn, Logger};
use strum::IntoEnumIterator;

use crate::bus::AppBus;
use crate::config::{ConfigManager, ServerBrowserConfig};
use crate::game::settings::server::Community;
use crate::game::Game;
use crate::gui::data::TableSource;
use crate::launcher::{ConnectionInfo, Launcher};
use crate::mod_manager::ModManager;
use crate::server_manager::ServerManager;
use crate::servers::{
    FavoriteServer, Mode, PingRequest, PingResponse, PingResult, Region, Server, SortCriteria,
    SortKey, TypeFilter,
};
use crate::util::weak_cb;

use super::data::{IterableTableSource, Reindex, RowFilter};
use super::{alert_error, glyph, wrapper_factory};

mod actions_pane;
mod add_server_dialog;
mod advanced_filter_dialog;
mod connect_dialog;
mod details_pane;
mod filter_pane;
mod list_pane;
mod state;

use self::actions_pane::{Action, ActionsPane};
use self::add_server_dialog::AddServerDialog;
use self::connect_dialog::ConnectDialog;
use self::details_pane::DetailsPane;
use self::filter_pane::{FilterHolder, FilterPane};
use self::list_pane::ListPane;
use self::state::{Filter, ServerBrowserState, SortOrder};

#[derive(dynabus::Event)]
pub struct PopulateServers {
    pub payload: Result<Vec<Server>>,
    pub done: bool,
}

#[derive(dynabus::Event)]
pub enum ProcessPongs {
    One(PingResponse),
    Many(Vec<PingResponse>),
}

#[derive(dynabus::Event)]
pub struct UpdateServer {
    pub idx: Option<usize>,
    pub server: Server,
}

#[derive(dynabus::Event)]
pub struct RefreshServerDetails;

pub(super) struct ServerBrowserTab {
    logger: Logger,
    game: Arc<Game>,
    config: Rc<ConfigManager>,
    launcher: Rc<Launcher>,
    server_mgr: Rc<ServerManager>,
    grid: Grid,
    root: Group,
    list_pane: Rc<ListPane>,
    details_pane: DetailsPane,
    actions_pane: Rc<ActionsPane>,
    stats: BrowserStats,
    loading_label: Frame,
    state: Rc<RefCell<ServerBrowserState>>,
    deferred_action: Cell<Option<DeferredAction>>,
    filter_dirty: Cell<bool>,
    refreshing: Cell<bool>,
}

struct BrowserStats {
    group: Group,
    total_servers_text: Frame,
    total_players_text: Frame,
    matching_servers_text: Frame,
    matching_players_text: Frame,
    total_servers: Cell<usize>,
    total_players: Cell<usize>,
    matching_servers: Cell<usize>,
    matching_players: Cell<usize>,
}

enum DeferredAction {
    Refresh,
    AlertError(&'static str, anyhow::Error),
    PingServers,
}

impl ServerBrowserTab {
    pub fn new(
        logger: &Logger,
        bus: Rc<RefCell<AppBus>>,
        game: Arc<Game>,
        config: Rc<ConfigManager>,
        launcher: Rc<Launcher>,
        server_mgr: Rc<ServerManager>,
        mod_manager: Rc<ModManager>,
    ) -> Rc<Self> {
        let browser_cfg = Ref::map(config.get(), |config| &config.server_browser);
        let state = Rc::new(RefCell::new(ServerBrowserState::new(
            Vec::new(),
            Filter::from_config(&*browser_cfg),
            SortOrder::new(browser_cfg.sort_criteria, region_sort_order()),
        )));

        let mut grid = Grid::builder_with_factory(wrapper_factory())
            .with_col_spacing(10)
            .with_row_spacing(10);
        grid.col().with_stretch(1).add();

        grid.row().add();
        let filter_pane = FilterPane::new(game.maps());
        grid.cell()
            .unwrap()
            .add_shared(Rc::<FilterPane>::clone(&filter_pane));

        grid.row().add();
        let mut status_overlay = Overlay::builder_with_factory(wrapper_factory());
        let (stats, stats_grid) = BrowserStats::new();
        status_overlay.add(stats_grid);
        let mut loading_label = status_overlay
            .wrap(Frame::default())
            .with_label(&format!("Fetching server list... {}", glyph::RELOAD));
        loading_label.set_frame(fltk::enums::FrameType::EngravedBox);
        let status_overlay = status_overlay.end();
        grid.cell().unwrap().add(status_overlay);

        let mut tiles = GridBuilder::with_factory(Tile::default_fill(), wrapper_factory());
        tiles.col().with_stretch(1).add();

        let upper_tile = Group::default_fill();
        tiles.row().with_stretch(3).add();
        tiles
            .cell()
            .unwrap()
            .with_vert_align(CellAlign::Stretch)
            .add(SimpleWrapper::new(upper_tile.clone(), Default::default()));

        let list_pane = ListPane::new(&state.borrow().order().criteria, browser_cfg.scroll_lock);

        upper_tile.end();

        let lower_tile = Group::default_fill();
        tiles.row().with_stretch(1).add();
        tiles
            .cell()
            .unwrap()
            .with_vert_align(CellAlign::Stretch)
            .add(SimpleWrapper::new(lower_tile.clone(), Default::default()));

        let details_pane = DetailsPane::new(mod_manager);

        lower_tile.end();

        let tiles = tiles.end();
        tiles.layout_children(); // necessary for Tile

        grid.row().with_stretch(1).add();
        grid.cell()
            .unwrap()
            .with_vert_align(CellAlign::Stretch)
            .add(tiles);

        grid.row().add();
        let actions_pane = ActionsPane::new(browser_cfg.scroll_lock, server_mgr.can_save_servers());
        grid.cell().unwrap().add(actions_pane.element());

        let grid = grid.end();
        grid.layout_children();

        let mut root = grid.group();
        root.hide();

        drop(browser_cfg);

        let this = Rc::new(Self {
            logger: logger.clone(),
            game,
            config,
            launcher,
            server_mgr,
            grid,
            root: root.clone(),
            list_pane: Rc::clone(&list_pane),
            details_pane,
            actions_pane: Rc::clone(&actions_pane),
            stats,
            loading_label,
            state: Rc::clone(&state),
            deferred_action: Cell::new(Some(DeferredAction::Refresh)),
            filter_dirty: Cell::new(false),
            refreshing: Cell::new(true),
        });

        root.handle(weak_cb!([this] => |_, event| {
            if let Event::Show = event {
                this.on_show();
            }
        }; false));

        filter_pane.set_filter_holder(Rc::clone(&this));
        list_pane.set_on_sort_changed(weak_cb!(
            [this] => |sort_criteria| {
                let selected_idx = this.selected_server_index();
                this.state
                    .borrow_mut()
                    .update_order(|order| order.criteria = sort_criteria);
                this.list_pane.populate(this.state.clone());
                this.set_selected_server_index(selected_idx, true);
                this.update_config();
            }
        ));
        list_pane.set_on_server_selected(weak_cb!(
            [this] => |server| {
                this.details_pane.populate(server);
                this.actions_pane.server_selected(server);
            }
        ));
        actions_pane.set_on_action(weak_cb!(
            [this] => |action| {
                match action {
                    Action::Join => this.on_join(),
                    Action::DirectConnect => this.on_direct_connect(),
                    Action::Ping => this.on_ping(),
                    Action::Refresh => this.on_refresh(),
                    Action::ToggleFavorite => this.on_toggle_favorite(),
                    Action::ToggleSaved => this.on_toggle_saved(),
                    Action::AddSaved => this.on_add_saved(),
                    Action::ScrollLock(scroll_lock) => {
                        this.list_pane.set_scroll_lock(scroll_lock);
                        this.update_config();
                    }
                }
            }
        ));

        {
            let mut bus = bus.borrow_mut();
            bus.subscribe_consumer(weak_cb!(
                [this] => |PopulateServers { payload, done }| this.populate_servers(payload, done)
            ));
            bus.subscribe_consumer(weak_cb!(
                [this] => |pongs: ProcessPongs| this.update_pinged_servers(pongs)
            ));
            bus.subscribe_consumer(weak_cb!(
                [this] => |UpdateServer { idx, server }| this.update_server(idx, server)
            ));
            bus.subscribe_consumer(weak_cb!(
                [this] => |RefreshServerDetails| this.refresh_server_details()
            ));
        }

        this
    }

    pub fn root(&self) -> &impl WidgetExt {
        &self.root
    }

    fn on_show(&self) {
        match self.deferred_action.take() {
            None => (),
            Some(DeferredAction::Refresh) => {
                self.server_mgr.load_server_list();
            }
            Some(DeferredAction::AlertError(msg, err)) => {
                alert_error(msg, &err);
            }
            Some(DeferredAction::PingServers) => self.ping_servers(),
        }
    }

    fn on_join(&self) {
        if let Some(server_idx) = self.list_pane.selected_index() {
            let conn_info = {
                let state = self.state.borrow();
                let server = &state[server_idx];
                if server.password_protected {
                    let password = match self.game.load_server_password(&server.name) {
                        Ok(password) => password.unwrap_or_default(),
                        Err(err) => {
                            warn!(
                                self.logger,
                                "Error loading saved password for server";
                                "server" => &server.name,
                                "error" => %err,
                            );
                            "".to_string()
                        }
                    };

                    let dialog = ConnectDialog::server_password(&self.root, server, &password);

                    // The following line is necessary, otherwise the incoming
                    // server list updates panic because the state remains
                    // borrowed while the dialog is displayed. ¯\_(ツ)_/¯
                    drop(state);

                    let dlg_result = match dialog.run() {
                        Some(dlg_result) => dlg_result,
                        None => return,
                    };

                    if dlg_result.save_password {
                        let state = self.state.borrow();
                        let server = &state[server_idx];
                        if let Err(err) = self.game.save_server_password(
                            &server.name,
                            dlg_result.connection.password.as_ref().unwrap(),
                        ) {
                            warn!(
                                self.logger,
                                "Error loading saved password for server";
                                "server" => &server.name,
                                "error" => %err,
                            );
                        }
                    }
                    dlg_result.connection
                } else {
                    ConnectionInfo {
                        addr: server.game_addr().unwrap(),
                        password: None,
                        battleye_required: Some(server.general.battleye_required),
                    }
                }
            };
            if let Err(err) = self.launcher.join_server(conn_info) {
                error!(self.logger, "Error joining server"; "error" => %err);
                alert_error(ERR_JOINING_SERVER, &err);
            }
        }
    }

    fn on_direct_connect(&self) {
        let dialog = ConnectDialog::direct_connect(&self.root);
        let Some(dlg_result) = dialog.run() else {
            return;
        };
        if let Err(err) = self.launcher.join_server(dlg_result.connection) {
            error!(self.logger, "Error on direct connect"; "error" => %err);
            alert_error(ERR_JOINING_SERVER, &err);
        }
    }

    fn on_ping(&self) {
        if let Some(server_idx) = self.list_pane.selected_index() {
            let state = self.state.borrow();
            let server = &state[server_idx];
            let source_idx = state.to_source_index(server_idx);
            let request = PingRequest::for_server(source_idx, server).unwrap();
            drop(state);

            self.update_servers(1, |all_servers, updated_indices, _, _| {
                all_servers[source_idx].waiting_for_pong = true;
                updated_indices.push(source_idx);
                Reindex::Nothing
            });

            if let Err(err) = self.server_mgr.ping_server(request) {
                error!(self.logger, "Error pinging server"; "error" => %err);
                alert_error(ERR_PINGING_SERVERS, &err);
            }
        }
    }

    fn on_refresh(&self) {
        self.refreshing.set(true);
        {
            let mut state = self.state.borrow_mut();
            state.update_source(Vec::clear);
        }
        self.list_pane.mark_refreshing();
        self.list_pane.set_selected_index(None, false);
        self.loading_label.clone().show();
        self.stats.hide();
        self.server_mgr.load_server_list();
    }

    fn on_toggle_favorite(&self) {
        if let Some(server_idx) = self.list_pane.selected_index() {
            // TODO: Only update if action was performed without error
            let src_idx = self.state.borrow().to_source_index(server_idx);
            self.update_servers(1, |all_servers, updated_indices, filter, _| {
                all_servers[src_idx].favorite = !all_servers[src_idx].favorite;
                updated_indices.push(src_idx);
                Reindex::Order.filter_if(filter.type_filter == TypeFilter::Favorite)
            });
            let state = self.state.borrow_mut();
            let favorites = state.source().iter().filter_map(|server| {
                if server.favorite {
                    Some(FavoriteServer::from_server(server))
                } else {
                    None
                }
            });

            if let Err(err) = self.game.save_favorites(favorites) {
                error!(
                    self.logger,
                    "Error updating favorites";
                    "error" => %err,
                );
                alert_error(ERR_UPDATING_FAVORITES, &err);
            }
        }
    }

    fn on_toggle_saved(&self) {
        if let Some(server_idx) = self.list_pane.selected_index() {
            let state = self.state.borrow();
            let src_idx = state.to_source_index(server_idx);
            let mut server = state[server_idx].clone();
            if !server.is_saved() {
                server.merged = true;
            }
            let result = if server.is_saved() {
                self.server_mgr.unsave_server(server, Some(src_idx))
            } else {
                self.server_mgr.save_server(server, Some(src_idx))
            };
            if let Err(err) = result {
                error!(
                    self.logger,
                    "Error updating saved servers";
                    "error" => %err,
                );
                alert_error(ERR_UPDATING_SAVED_SERVERS, &err);
            }
        }
    }

    fn on_add_saved(&self) {
        let dialog = AddServerDialog::new(&self.root, Arc::clone(&self.game));
        let Some(server) = dialog.run() else {
            return;
        };
        if let Err(err) = self.server_mgr.save_server(server, None) {
            error!(self.logger, "Error on adding saved server"; "error" => %err);
            alert_error(ERR_UPDATING_SAVED_SERVERS, &err);
        }
    }

    fn populate_servers(&self, payload: Result<Vec<Server>>, done: bool) {
        self.deferred_action.set(None);

        if done {
            self.refreshing.set(false);
            self.loading_label.clone().hide();
            self.stats.show();
        }

        let all_servers = match payload {
            Ok(all_servers) => all_servers,
            Err(err) => {
                self.list_pane.clear_refreshing();
                if self.root.visible() {
                    alert_error(ERR_LOADING_SERVERS, &err);
                } else {
                    self.deferred_action
                        .set(Some(DeferredAction::AlertError(ERR_LOADING_SERVERS, err)));
                }
                return;
            }
        };
        self.stats.set_total_servers(all_servers.len());

        {
            let mut state = self.state.borrow_mut();
            state.update(|servers, _, _| {
                *servers = all_servers;
                Reindex::all()
            });
        }
        self.stats.set_matching_servers(self.state.borrow().len());

        let state = Rc::clone(&self.state);
        self.list_pane.populate(state);

        if done {
            if self.root.visible() {
                self.ping_servers();
            } else {
                self.deferred_action.set(Some(DeferredAction::PingServers));
            }
        }
    }

    fn ping_servers(&self) {
        let ping_requests = {
            let state = self.state.borrow();
            let mut requests = Vec::with_capacity(state.source().len());

            requests.extend(state.iter().enumerate().filter_map(|(idx, server)| {
                PingRequest::for_server(state.to_source_index(idx), server)
            }));

            requests.extend(
                state
                    .source()
                    .iter()
                    .enumerate()
                    .filter(|(idx, _)| state.from_source_index(*idx).is_none())
                    .filter_map(|(idx, server)| PingRequest::for_server(idx, server)),
            );

            requests
        };

        self.stats.set_total_players(0);
        self.stats.set_matching_players(0);

        if let Err(err) = self.server_mgr.ping_servers(ping_requests) {
            error!(self.logger, "Error pinging server list"; "error" => %err);
            alert_error(ERR_PINGING_SERVERS, &err);
        }
    }

    fn update_pinged_servers(&self, pongs: ProcessPongs) {
        if self.refreshing.get() {
            return;
        }

        let updates = match &pongs {
            ProcessPongs::One(pong) => std::slice::from_ref(pong),
            ProcessPongs::Many(pongs) => &pongs[..],
        };

        let mut total_players = self.stats.total_players();
        let mut matching_players = self.stats.matching_players();
        self.update_servers(
            updates.len(),
            |all_servers, updated_indices, filter, sort_criteria| {
                let mut reindex = Reindex::Nothing;
                for update in updates {
                    let Some(server) = all_servers.get_mut(update.server_idx) else {
                        continue;
                    };
                    updated_indices.push(update.server_idx);
                    if Self::update_pinged_server(
                        server,
                        update,
                        filter,
                        &mut total_players,
                        &mut matching_players,
                    ) {
                        reindex = Reindex::Filter;
                    }
                }
                reindex.order_if(
                    (sort_criteria.key == SortKey::Players)
                        || (sort_criteria.key == SortKey::Age)
                        || (sort_criteria.key == SortKey::Ping),
                )
            },
        );
        self.stats.set_total_players(total_players);
        self.stats.set_matching_players(matching_players);
    }

    fn update_server(&self, idx: Option<usize>, server: Server) {
        self.update_servers(
            1,
            move |all_servers, updated_indices, filter, _sort_criteria| {
                let matched_before = idx
                    .map(|idx| filter.matches(&all_servers[idx]))
                    .unwrap_or_default();
                let matches_after = filter.matches(&server);
                match idx {
                    Some(idx) => {
                        all_servers[idx] = server;
                        updated_indices.push(idx);
                    }
                    None => {
                        all_servers.push(server);
                        updated_indices.push(all_servers.len() - 1);
                    }
                }
                Reindex::Order.filter_if(matched_before != matches_after)
            },
        );
    }

    fn refresh_server_details(&self) {
        if let Some(selected_idx) = self.list_pane.selected_index() {
            let server = &self.state.borrow()[selected_idx];
            self.details_pane.populate(Some(server));
        }
    }

    fn update_servers(
        &self,
        count_hint: usize,
        mutator: impl FnOnce(&mut Vec<Server>, &mut Vec<usize>, &Filter, &SortCriteria) -> Reindex,
    ) {
        let selected_idx = self.selected_server_index();

        let mut updated_indices: Vec<usize> = Vec::with_capacity(count_hint);
        let repopulate = {
            let mut state = self.state.borrow_mut();
            state.update(|all_servers, filter, order| {
                mutator(
                    all_servers,
                    &mut updated_indices,
                    filter,
                    &mut order.criteria,
                )
            })
        };

        if repopulate != Reindex::Nothing {
            self.list_pane.populate(self.state.clone());
            self.set_selected_server_index(selected_idx, false);
        } else {
            let state = self.state.borrow();
            self.list_pane.update(
                updated_indices
                    .iter()
                    .filter_map(|&idx| state.from_source_index(idx)),
            );
        };
    }

    fn update_pinged_server(
        server: &mut Server,
        update: &PingResponse,
        filter: &Filter,
        total_players: &mut usize,
        matching_players: &mut usize,
    ) -> bool {
        let matched_before = filter.matches(server);
        *total_players -= server.connected_players.unwrap_or_default();
        if matched_before {
            *matching_players -= server.connected_players.unwrap_or_default();
        }

        match update.result {
            PingResult::Pong {
                connected_players,
                age,
                round_trip,
            } => {
                server.connected_players = Some(connected_players);
                server.age = Some(age);
                server.ping = Some(round_trip);
            }
            PingResult::Timeout => {
                server.connected_players = None;
                server.age = None;
                server.ping = None;
            }
        };
        server.waiting_for_pong = false;

        let matches_after = filter.matches(server);
        *total_players += server.connected_players.unwrap_or_default();
        if matched_before {
            *matching_players += server.connected_players.unwrap_or_default();
        }

        matches_after != matched_before
    }

    fn selected_server_index(&self) -> Option<usize> {
        self.list_pane
            .selected_index()
            .map(|index| self.state.borrow().to_source_index(index))
    }

    fn set_selected_server_index(&self, index: Option<usize>, override_scroll_lock: bool) {
        self.list_pane.set_selected_index(
            index.and_then(|index| self.state.borrow().from_source_index(index)),
            override_scroll_lock,
        );
    }

    fn update_config(&self) {
        let state = self.state.borrow();
        let filter = state.filter();
        let order = state.order();
        let browser_cfg = ServerBrowserConfig {
            filter: filter.as_ref().clone(),
            sort_criteria: order.criteria,
            scroll_lock: self.list_pane.scroll_lock(),
        };
        self.config
            .update(|config| config.server_browser = browser_cfg);
    }
}

impl LayoutElement for ServerBrowserTab {
    fn min_size(&self) -> fltk_float::Size {
        self.grid.min_size()
    }

    fn layout(&self, x: i32, y: i32, width: i32, height: i32) {
        self.grid.layout(x, y, width, height)
    }
}

impl FilterHolder for ServerBrowserTab {
    fn access_filter(&self, accessor: impl FnOnce(&Filter)) {
        accessor(self.state.borrow().filter())
    }

    fn mutate_filter(&self, mutator: impl FnOnce(&mut Filter)) {
        let selected_idx = self.selected_server_index();
        self.state.borrow_mut().update_filter(mutator);
        self.list_pane.populate(self.state.clone());
        self.set_selected_server_index(selected_idx, false);
        self.filter_dirty.set(true);

        let state = self.state.borrow();
        let matching_players = state
            .iter()
            .map(|server| server.connected_players.unwrap_or_default())
            .sum();
        self.stats.set_matching_servers(state.len());
        self.stats.set_matching_players(matching_players);
    }

    fn persist_filter(&self) {
        if self.filter_dirty.take() {
            self.update_config();
        }
    }
}

impl BrowserStats {
    fn new() -> (Self, Grid) {
        let mut grid = Grid::builder_with_factory(wrapper_factory())
            .with_col_spacing(10)
            .with_row_spacing(10)
            .with_padding(2, 2, 2, 2);
        grid.row().add();

        let total_servers_text = browser_stat(&mut grid, "Total Servers:");
        let total_players_text = browser_stat(&mut grid, "Total Players Online:");
        let matching_servers_text = browser_stat(&mut grid, "Matching Servers:");
        let matching_players_text = browser_stat(&mut grid, "Players on Matching Servers:");

        let grid = grid.end();
        let mut group = grid.group();
        group.set_frame(fltk::enums::FrameType::EngravedBox);
        group.hide();

        let this = Self {
            group,
            total_servers_text,
            total_players_text,
            matching_servers_text,
            matching_players_text,
            total_servers: Cell::new(0),
            total_players: Cell::new(0),
            matching_servers: Cell::new(0),
            matching_players: Cell::new(0),
        };

        (this, grid)
    }

    fn show(&self) {
        self.group.clone().show()
    }

    fn hide(&self) {
        let mut group = self.group.clone();
        group.hide();
        self.total_servers_text.clone().set_label("?");
        self.total_players_text.clone().set_label("?");
        self.matching_servers_text.clone().set_label("?");
        self.matching_players_text.clone().set_label("?");
        group.redraw();
    }

    fn total_players(&self) -> usize {
        self.total_players.get()
    }

    fn matching_players(&self) -> usize {
        self.matching_players.get()
    }

    fn set_total_servers(&self, count: usize) {
        self.total_servers.set(count);
        let mut total_servers_text = self.total_servers_text.clone();
        total_servers_text.set_label(&count.to_string());
        total_servers_text.redraw();
    }

    fn set_total_players(&self, count: usize) {
        self.total_players.set(count);
        let mut total_players_text = self.total_players_text.clone();
        total_players_text.set_label(&count.to_string());
        total_players_text.redraw();
    }

    fn set_matching_servers(&self, count: usize) {
        self.matching_servers.set(count);
        let mut matching_servers_text = self.matching_servers_text.clone();
        matching_servers_text.set_label(&count.to_string());
        matching_servers_text.redraw();
    }

    fn set_matching_players(&self, count: usize) {
        self.matching_players.set(count);
        let mut matching_players_text = self.matching_players_text.clone();
        matching_players_text.set_label(&count.to_string());
        matching_players_text.redraw();
    }
}

const ERR_LOADING_SERVERS: &str = "Error while loading the server list.";
const ERR_PINGING_SERVERS: &str = "Error while pinging servers.";
const ERR_JOINING_SERVER: &str = "Error while trying to launch the game to join the server.";
const ERR_UPDATING_FAVORITES: &str = "Error while updating favorites.";
const ERR_UPDATING_SAVED_SERVERS: &str = "Error while updating saved servers.";

fn browser_stat(grid: &mut GridBuilder<Group, Rc<WrapperFactory>>, label: &str) -> Frame {
    grid.col()
        .with_default_align(CellAlign::End)
        .with_stretch(1)
        .add();
    grid.cell()
        .unwrap()
        .wrap(Frame::default())
        .with_label(label);
    grid.col()
        .with_default_align(CellAlign::Stretch)
        .with_stretch(1)
        .add();
    grid.cell()
        .unwrap()
        .wrap(Frame::default())
        .with_label("?")
        .with_align(Align::Left | Align::Inside)
}

fn mode_name(mode: Mode) -> &'static str {
    match mode {
        Mode::PVE => "PVE",
        Mode::PVEC => "PVE-C",
        Mode::PVP => "PVP",
    }
}

fn region_name(region: Region) -> &'static str {
    match region {
        Region::EU => "EU",
        Region::America => "America",
        Region::Asia => "Asia",
        Region::Oceania => "Oceania",
        Region::LATAM => "LATAM",
        Region::Japan => "Japan",
    }
}

fn region_sort_order() -> HashMap<Region, usize> {
    let mut sorted_regions: Vec<_> = Region::iter().collect();
    sorted_regions.sort_by(|&lhs, &rhs| region_name(lhs).cmp(region_name(rhs)));

    let mut order = HashMap::new();
    for (idx, region) in sorted_regions.into_iter().enumerate() {
        order.insert(region, idx);
    }
    order
}

fn community_name(community: Community) -> &'static str {
    match community {
        Community::Unspecified => "Unspecified",
        Community::Purist => "Purist",
        Community::Relaxed => "Relaxed",
        Community::Hardcore => "Hardcore",
        Community::RolePlaying => "Role Playing",
        Community::Experimental => "Experimental",
    }
}
