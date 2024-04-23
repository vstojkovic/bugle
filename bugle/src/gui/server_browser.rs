use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::net::SocketAddr;
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
use fltk_float::{LayoutElement, SimpleWrapper};
use slog::{error, Logger};
use strum::IntoEnumIterator;

use crate::bus::AppBus;
use crate::config::ServerBrowserConfig;
use crate::game::platform::ModDirectory;
use crate::game::settings::server::Community;
use crate::game::Game;
use crate::servers::{
    FavoriteServer, Mode, PingRequest, PingResponse, PingResult, Region, Server, SortCriteria,
    SortKey, TypeFilter,
};
use crate::util::weak_cb;

use super::data::{IterableTableSource, Reindex, RowFilter};
use super::{alert_error, glyph, wrapper_factory, Handler};

mod actions_pane;
mod add_server_dialog;
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

pub enum ServerBrowserAction {
    LoadServers,
    JoinServer {
        addr: SocketAddr,
        password: Option<String>,
        battleye_required: Option<bool>,
    },
    PingServer(PingRequest),
    PingServers(Vec<PingRequest>),
    UpdateFavorites(Vec<FavoriteServer>),
    UpdateConfig(ServerBrowserConfig),
    ToggleSavedServer {
        server: Server,
        idx: Option<usize>,
    },
}

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
    grid: Grid,
    root: Group,
    on_action: Box<dyn Handler<ServerBrowserAction>>,
    list_pane: Rc<ListPane>,
    details_pane: DetailsPane,
    actions_pane: Rc<ActionsPane>,
    stats_group: Group,
    total_players: Cell<usize>,
    total_players_text: Frame,
    loading_label: Frame,
    state: Rc<RefCell<ServerBrowserState>>,
    deferred_action: Cell<Option<DeferredAction>>,
    filter_dirty: Cell<bool>,
    refreshing: Cell<bool>,
}

enum DeferredAction {
    Refresh,
    AlertError(&'static str, anyhow::Error),
    PingServers,
}

impl ServerBrowserTab {
    pub fn new(
        logger: Logger,
        bus: &mut AppBus,
        game: Arc<Game>,
        config: &ServerBrowserConfig,
        mod_resolver: Rc<dyn ModDirectory>,
        can_save_servers: bool,
        on_action: impl Handler<ServerBrowserAction> + 'static,
    ) -> Rc<Self> {
        let state = Rc::new(RefCell::new(ServerBrowserState::new(
            Vec::new(),
            Filter::from_config(config),
            SortOrder::new(config.sort_criteria, region_sort_order()),
        )));

        let mut grid = Grid::builder_with_factory(wrapper_factory())
            .with_col_spacing(10)
            .with_row_spacing(10);
        grid.col().with_stretch(1).add();

        grid.row().add();
        let filter_pane = FilterPane::new(Arc::clone(game.maps()));
        grid.cell()
            .unwrap()
            .add_shared(Rc::<FilterPane>::clone(&filter_pane));

        grid.row().add();
        let mut status_overlay = Overlay::builder_with_factory(wrapper_factory());
        let mut stats_grid = Grid::builder_with_factory(wrapper_factory())
            .with_col_spacing(10)
            .with_row_spacing(10)
            .with_padding(2, 2, 2, 2);
        stats_grid.row().add();
        stats_grid
            .col()
            .with_default_align(CellAlign::End)
            .with_stretch(1)
            .add();
        stats_grid
            .cell()
            .unwrap()
            .wrap(Frame::default())
            .with_label("Total Players Online:");
        stats_grid
            .col()
            .with_default_align(CellAlign::Stretch)
            .with_stretch(1)
            .add();
        let total_players_text = stats_grid
            .cell()
            .unwrap()
            .wrap(Frame::default())
            .with_label("?")
            .with_align(Align::Left | Align::Inside);
        let stats_grid = stats_grid.end();
        let mut stats_group = stats_grid.group();
        stats_group.set_frame(fltk::enums::FrameType::EngravedBox);
        stats_group.hide();
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

        let list_pane = ListPane::new(&state.borrow().order().criteria, config.scroll_lock);

        upper_tile.end();

        let lower_tile = Group::default_fill();
        tiles.row().with_stretch(1).add();
        tiles
            .cell()
            .unwrap()
            .with_vert_align(CellAlign::Stretch)
            .add(SimpleWrapper::new(lower_tile.clone(), Default::default()));

        let details_pane = DetailsPane::new(mod_resolver);

        lower_tile.end();

        let tiles = tiles.end();
        tiles.layout_children(); // necessary for Tile

        grid.row().with_stretch(1).add();
        grid.cell()
            .unwrap()
            .with_vert_align(CellAlign::Stretch)
            .add(tiles);

        grid.row().add();
        let actions_pane = ActionsPane::new(config.scroll_lock, can_save_servers);
        grid.cell().unwrap().add(actions_pane.element());

        let grid = grid.end();
        grid.layout_children();

        let mut root = grid.group();
        root.hide();

        let this = Rc::new(Self {
            logger,
            game,
            grid,
            root: root.clone(),
            on_action: Box::new(on_action),
            list_pane: Rc::clone(&list_pane),
            details_pane,
            actions_pane: Rc::clone(&actions_pane),
            stats_group,
            total_players: Cell::new(0),
            total_players_text,
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
                    Action::Join => {
                        if let Some(server_idx) = this.list_pane.selected_index() {
                            let action = {
                                let state = this.state.borrow();
                                let server = &state[server_idx];
                                if server.password_protected {
                                    let dialog = ConnectDialog::server_password(&this.root, server);

                                    // The following line is necessary, otherwise the incoming
                                    // server list updates panic because the state remains
                                    // borrowed while the dialog is displayed. ¯\_(ツ)_/¯
                                    drop(state);

                                    match dialog.run() {
                                        Some(action) => action,
                                        None => return,
                                    }
                                } else {
                                    ServerBrowserAction::JoinServer {
                                        addr: server.game_addr().unwrap(),
                                        password: None,
                                        battleye_required: Some(server.general.battleye_required),
                                    }
                                }
                            };
                            if let Err(err) = (this.on_action)(action) {
                                error!(this.logger, "Error joining server"; "error" => %err);
                                alert_error(ERR_JOINING_SERVER, &err);
                            }
                        }
                    }
                    Action::Ping => {
                        if let Some(server_idx) = this.list_pane.selected_index() {
                            let state = this.state.borrow();
                            let server = &state[server_idx];
                            let source_idx = state.to_source_index(server_idx);
                            let request = PingRequest::for_server(source_idx, server).unwrap();
                            let action = ServerBrowserAction::PingServer(request);
                            drop(state);

                            this.update_servers(1, |all_servers, updated_indices, _, _| {
                                all_servers[source_idx].waiting_for_pong = true;
                                updated_indices.push(source_idx);
                                Reindex::Nothing
                            });

                            if let Err(err) = (this.on_action)(action) {
                                error!(this.logger, "Error pinging server"; "error" => %err);
                                alert_error(ERR_PINGING_SERVERS, &err);
                            }
                        }
                    }
                    Action::ToggleSaved => {
                        if let Some(server_idx) = this.list_pane.selected_index() {
                            let state = this.state.borrow();
                            let src_idx = state.to_source_index(server_idx);
                            let mut server = state[server_idx].clone();
                            if !server.is_saved() {
                                server.merged = true;
                            }
                            let action = ServerBrowserAction::ToggleSavedServer {
                                server,
                                idx: Some(src_idx),
                            };
                            if let Err(err) = (this.on_action)(action) {
                                error!(
                                    this.logger,
                                    "Error updating saved servers";
                                    "error" => %err,
                                );
                                alert_error(ERR_UPDATING_SAVED_SERVERS, &err);
                            }
                        }
                    }
                    Action::ToggleFavorite => {
                        if let Some(server_idx) = this.list_pane.selected_index() {
                            // TODO: Only update if action was performed without error
                            let src_idx = this.state.borrow().to_source_index(server_idx);
                            this.update_servers(1, |all_servers, updated_indices, filter, _| {
                                all_servers[src_idx].favorite = !all_servers[src_idx].favorite;
                                updated_indices.push(src_idx);
                                Reindex::Order
                                    .filter_if(filter.type_filter() == TypeFilter::Favorite)
                            });
                            let state = this.state.borrow_mut();
                            let favorites = state
                                .source()
                                .iter()
                                .filter_map(|server| {
                                    if server.favorite {
                                        Some(FavoriteServer::from_server(server))
                                    } else {
                                        None
                                    }
                                })
                                .collect();
                            let action = ServerBrowserAction::UpdateFavorites(favorites);
                            if let Err(err) = (this.on_action)(action) {
                                error!(
                                    this.logger,
                                    "Error updating favorites";
                                    "error" => %err,
                                );
                                alert_error(ERR_UPDATING_FAVORITES, &err);
                            }
                        }
                    }
                    Action::Refresh => {
                        this.refreshing.set(true);
                        {
                            let mut state = this.state.borrow_mut();
                            state.update_source(Vec::clear);
                        }
                        this.list_pane.mark_refreshing();
                        this.list_pane.set_selected_index(None, false);
                        this.loading_label.clone().show();
                        this.stats_group.clone().hide();
                        let mut total_players_text = this.total_players_text.clone();
                        total_players_text.set_label("?");
                        total_players_text.redraw();
                        (this.on_action)(ServerBrowserAction::LoadServers).unwrap();
                    }
                    Action::DirectConnect => {
                        let dialog = ConnectDialog::direct_connect(&this.root);
                        let action = match dialog.run() {
                            Some(action) => action,
                            None => return,
                        };
                        if let Err(err) = (this.on_action)(action) {
                            error!(this.logger, "Error on direct connect"; "error" => %err);
                            alert_error(ERR_JOINING_SERVER, &err);
                        }
                    }
                    Action::AddSaved => {
                        let dialog = AddServerDialog::new(&this.root, Arc::clone(&this.game));
                        let action = match dialog.run() {
                            Some(action) => action,
                            None => return,
                        };
                        if let Err(err) = (this.on_action)(action) {
                            error!(this.logger, "Error on adding saved server"; "error" => %err);
                            alert_error(ERR_UPDATING_SAVED_SERVERS, &err);
                        }
                    }
                    Action::ScrollLock(scroll_lock) => {
                        this.list_pane.set_scroll_lock(scroll_lock);
                        this.update_config();
                    }
                }
            }
        ));

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
            [this] => |RefreshServerDetails| this.refresh_server_details()));

        this
    }

    pub fn root(&self) -> &impl WidgetExt {
        &self.root
    }

    fn on_show(&self) {
        match self.deferred_action.take() {
            None => (),
            Some(DeferredAction::Refresh) => {
                (self.on_action)(ServerBrowserAction::LoadServers).unwrap();
            }
            Some(DeferredAction::AlertError(msg, err)) => {
                alert_error(msg, &err);
            }
            Some(DeferredAction::PingServers) => self.ping_servers(),
        }
    }

    fn populate_servers(&self, payload: Result<Vec<Server>>, done: bool) {
        self.deferred_action.set(None);

        if done {
            self.refreshing.set(false);
            self.loading_label.clone().hide();
            self.stats_group.clone().show();
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

        {
            let mut state = self.state.borrow_mut();
            state.update(|servers, _, _| {
                *servers = all_servers;
                Reindex::all()
            });
        }

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

        self.set_total_player_count(0);

        if let Err(err) = (self.on_action)(ServerBrowserAction::PingServers(ping_requests)) {
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

        let mut total_players = self.total_players.get();
        self.update_servers(
            updates.len(),
            |all_servers, updated_indices, filter, sort_criteria| {
                let mut reindex = Reindex::Nothing;
                for update in updates {
                    let server = match all_servers.get_mut(update.server_idx) {
                        Some(server) => server,
                        None => continue,
                    };
                    updated_indices.push(update.server_idx);
                    if Self::update_pinged_server(server, update, filter, &mut total_players) {
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
        self.set_total_player_count(total_players);
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
    ) -> bool {
        *total_players -= server.connected_players.unwrap_or_default();
        let matched_before = filter.matches(server);
        match update.result {
            PingResult::Pong {
                connected_players,
                age,
                round_trip,
            } => {
                server.connected_players = Some(connected_players);
                server.age = Some(age);
                server.ping = Some(round_trip);
                *total_players += connected_players;
            }
            PingResult::Timeout => {
                server.connected_players = None;
                server.age = None;
                server.ping = None;
            }
        };
        server.waiting_for_pong = false;
        filter.matches(server) != matched_before
    }

    fn set_total_player_count(&self, count: usize) {
        self.total_players.set(count);
        let mut total_players_text = self.total_players_text.clone();
        total_players_text.set_label(&count.to_string());
        total_players_text.redraw();
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
        let config = ServerBrowserConfig {
            filter: filter.as_ref().clone(),
            sort_criteria: order.criteria,
            scroll_lock: self.list_pane.scroll_lock(),
        };
        (self.on_action)(ServerBrowserAction::UpdateConfig(config)).unwrap();
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
        accessor(self.state.borrow().filter());
    }

    fn mutate_filter(&self, mutator: impl FnOnce(&mut Filter)) {
        let selected_idx = self.selected_server_index();
        self.state.borrow_mut().update_filter(mutator);
        self.list_pane.populate(self.state.clone());
        self.set_selected_server_index(selected_idx, false);
        self.filter_dirty.set(true);
    }

    fn persist_filter(&self) {
        if self.filter_dirty.take() {
            self.update_config();
        }
    }
}

const ERR_LOADING_SERVERS: &str = "Error while loading the server list.";
const ERR_PINGING_SERVERS: &str = "Error while pinging servers.";
const ERR_JOINING_SERVER: &str = "Error while trying to launch the game to join the server.";
const ERR_UPDATING_FAVORITES: &str = "Error while updating favorites.";
const ERR_UPDATING_SAVED_SERVERS: &str = "Error while updating saved servers.";

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
        Community::Unspecified => "",
        Community::Purist => "Purist",
        Community::Relaxed => "Relaxed",
        Community::Hardcore => "Hardcore",
        Community::RolePlaying => "Role Playing",
        Community::Experimental => "Experimental",
    }
}
