use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::rc::Rc;
use std::sync::Arc;

use anyhow::Result;
use fltk::enums::Align;
use fltk::frame::Frame;
use fltk::group::{Group, Tile};
use fltk::prelude::*;
use fltk_float::grid::{CellAlign, Grid, GridBuilder};
use fltk_float::SimpleWrapper;
use slog::{error, Logger};
use strum::IntoEnumIterator;

use crate::config::ServerBrowserConfig;
use crate::game::platform::ModDirectory;
use crate::game::Maps;
use crate::gui::data::{Reindex, RowFilter};
use crate::servers::{
    Community, FavoriteServer, Mode, PingRequest, PingResponse, PingResult, Region, Server,
    SortCriteria, SortKey, TypeFilter, Weekday,
};

use self::actions_pane::{Action, ActionsPane};
use self::connect_dialog::ConnectDialog;
use self::details_pane::DetailsPane;
use self::filter_pane::{FilterHolder, FilterPane};
use self::list_pane::ListPane;
use self::state::{Filter, SortOrder};

use super::data::IterableTableSource;
use super::{alert_error, wrapper_factory, CleanupFn, Handler};

mod actions_pane;
mod connect_dialog;
mod details_pane;
mod filter_pane;
mod list_pane;
mod state;

use state::ServerBrowserState;

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
}

pub enum ServerBrowserUpdate {
    PrefetchDisabled,
    PopulateServers(Result<Vec<Server>>),
    UpdateServer(PingResponse),
    BatchUpdateServers(Vec<PingResponse>),
    RefreshDetails,
}

impl ServerBrowserUpdate {
    pub fn try_consolidate(
        self,
        other: ServerBrowserUpdate,
    ) -> std::result::Result<Self, (Self, Self)> {
        match (self, other) {
            (Self::BatchUpdateServers(mut consolidated), Self::UpdateServer(response)) => {
                consolidated.push(response);
                Ok(Self::BatchUpdateServers(consolidated))
            }
            (Self::UpdateServer(first), Self::UpdateServer(second)) => {
                Ok(Self::BatchUpdateServers(vec![first, second]))
            }
            (this, other) => Err((this, other)),
        }
    }
}

impl From<ServerBrowserUpdate> for super::Update {
    fn from(update: ServerBrowserUpdate) -> Self {
        Self::ServerBrowser(update)
    }
}

pub(super) struct ServerBrowser {
    logger: Logger,
    root: Group,
    on_action: Box<dyn Handler<ServerBrowserAction>>,
    list_pane: Rc<ListPane>,
    details_pane: DetailsPane,
    actions_pane: Rc<ActionsPane>,
    total_players: Cell<usize>,
    total_players_text: Frame,
    pending_update: Rc<Cell<Option<ServerBrowserUpdate>>>,
    state: Rc<RefCell<ServerBrowserState>>,
    filter_dirty: Cell<bool>,
    refreshing: Cell<bool>,
}

impl ServerBrowser {
    pub fn new(
        logger: Logger,
        maps: Arc<Maps>,
        config: &ServerBrowserConfig,
        mod_resolver: Rc<dyn ModDirectory>,
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
        let filter_pane = FilterPane::new(maps);
        grid.cell().unwrap().add(filter_pane.element());

        grid.row().add();
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
        stats_grid
            .group()
            .set_frame(fltk::enums::FrameType::BorderBox);
        grid.cell().unwrap().add(stats_grid);

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
        let actions_pane = ActionsPane::new(config.scroll_lock);
        grid.cell().unwrap().add(actions_pane.element());

        let grid = grid.end();
        grid.layout_children();

        let mut root = grid.group();
        root.hide();

        let browser = Rc::new(Self {
            logger,
            root,
            on_action: Box::new(on_action),
            list_pane: Rc::clone(&list_pane),
            details_pane,
            actions_pane: Rc::clone(&actions_pane),
            total_players: Cell::new(0),
            total_players_text,
            pending_update: Rc::new(Cell::new(None)),
            state: Rc::clone(&state),
            filter_dirty: Cell::new(false),
            refreshing: Cell::new(true),
        });

        filter_pane.set_filter_holder(Rc::clone(&browser));
        {
            let browser = Rc::downgrade(&browser);
            list_pane.set_on_sort_changed(move |sort_criteria| {
                if let Some(browser) = browser.upgrade() {
                    let selected_idx = browser.selected_server_index();
                    browser
                        .state
                        .borrow_mut()
                        .update_order(|order| order.criteria = sort_criteria);
                    browser.list_pane.populate(browser.state.clone());
                    browser.set_selected_server_index(selected_idx, true);
                    browser.update_config();
                }
            });
        }
        {
            let browser = Rc::downgrade(&browser);
            list_pane.set_on_server_selected(move |server| {
                if let Some(browser) = browser.upgrade() {
                    browser.details_pane.populate(server);
                    browser.actions_pane.server_selected(server);
                }
            });
        }
        {
            let browser = Rc::downgrade(&browser);
            actions_pane.set_on_action(move |action| {
                if let Some(browser) = browser.upgrade() {
                    match action {
                        Action::Join => {
                            if let Some(server_idx) = browser.list_pane.selected_index() {
                                let action = {
                                    let server = &browser.state.borrow()[server_idx];
                                    if server.password_protected {
                                        let dialog =
                                            ConnectDialog::server_password(&browser.root, server);
                                        dialog.show();
                                        // TODO: Ensure main loop is run
                                        while dialog.shown() {
                                            fltk::app::wait();
                                        }
                                        match dialog.result() {
                                            Some(action) => action,
                                            None => return,
                                        }
                                    } else {
                                        ServerBrowserAction::JoinServer {
                                            addr: server.game_addr().unwrap(),
                                            password: None,
                                            battleye_required: Some(server.battleye_required),
                                        }
                                    }
                                };
                                if let Err(err) = (browser.on_action)(action) {
                                    error!(browser.logger, "Error joining server"; "error" => %err);
                                    alert_error(ERR_JOINING_SERVER, &err);
                                }
                            }
                        }
                        Action::Ping => {
                            if let Some(server_idx) = browser.list_pane.selected_index() {
                                let state = browser.state.borrow();
                                let server = &state[server_idx];
                                let source_idx = state.to_source_index(server_idx);
                                let request = PingRequest::for_server(source_idx, server).unwrap();
                                let action = ServerBrowserAction::PingServer(request);
                                drop(state);

                                browser.update_servers(1, |all_servers, updated_indices, _, _| {
                                    all_servers[source_idx].waiting_for_pong = true;
                                    updated_indices.push(source_idx);
                                    Reindex::Nothing
                                });

                                if let Err(err) = (browser.on_action)(action) {
                                    error!(browser.logger, "Error pinging server"; "error" => %err);
                                    alert_error(ERR_PINGING_SERVERS, &err);
                                }
                            }
                        }
                        Action::ToggleFavorite => {
                            if let Some(server_idx) = browser.list_pane.selected_index() {
                                let src_idx = browser.state.borrow().to_source_index(server_idx);
                                browser.update_servers(
                                    1,
                                    |all_servers, updated_indices, filter, _| {
                                        all_servers[src_idx].favorite =
                                            !all_servers[src_idx].favorite;
                                        updated_indices.push(src_idx);
                                        Reindex::Order
                                            .filter_if(filter.type_filter() == TypeFilter::Favorite)
                                    },
                                );
                                let state = browser.state.borrow_mut();
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
                                if let Err(err) = (browser.on_action)(action) {
                                    error!(
                                        browser.logger,
                                        "Error updating favorites";
                                        "error" => %err,
                                    );
                                    alert_error(ERR_UPDATING_FAVORITES, &err);
                                }
                            }
                        }
                        Action::Refresh => {
                            browser.refreshing.set(true);
                            {
                                let mut state = browser.state.borrow_mut();
                                state.update_source(Vec::clear);
                            }
                            browser.list_pane.mark_refreshing();
                            browser.list_pane.set_selected_index(None, false);
                            let mut total_players_text = browser.total_players_text.clone();
                            total_players_text.set_label("?");
                            total_players_text.redraw();
                            (browser.on_action)(ServerBrowserAction::LoadServers).unwrap();
                        }
                        Action::DirectConnect => {
                            let dialog = ConnectDialog::direct_connect(&browser.root);
                            dialog.show();
                            // TODO: Ensure main loop is run
                            while dialog.shown() {
                                fltk::app::wait();
                            }
                            let action = match dialog.result() {
                                Some(action) => action,
                                None => return,
                            };
                            if let Err(err) = (browser.on_action)(action) {
                                error!(browser.logger, "Error on direct connect"; "error" => %err);
                                alert_error(ERR_JOINING_SERVER, &err);
                            }
                        }
                        Action::ScrollLock(scroll_lock) => {
                            browser.list_pane.set_scroll_lock(scroll_lock);
                            browser.update_config();
                        }
                    }
                }
            });
        }

        browser
    }

    pub fn show(&self) -> CleanupFn {
        let mut root = self.root.clone();
        root.show();

        if let Some(update) = self.pending_update.take() {
            self.handle_update(update);
        }

        Box::new(move || {
            root.hide();
        })
    }

    pub fn handle_update(&self, update: ServerBrowserUpdate) {
        match update {
            ServerBrowserUpdate::PrefetchDisabled => {
                if self.root.visible() {
                    (self.on_action)(ServerBrowserAction::LoadServers).unwrap();
                } else {
                    self.pending_update
                        .set(Some(ServerBrowserUpdate::PrefetchDisabled));
                }
            }
            ServerBrowserUpdate::PopulateServers(payload) => {
                if self.root.visible() {
                    self.refreshing.set(false);
                    match payload {
                        Ok(all_servers) => self.populate_servers(all_servers),
                        Err(err) => {
                            self.list_pane.clear_refreshing();
                            alert_error(ERR_LOADING_SERVERS, &err);
                        }
                    }
                } else {
                    self.pending_update
                        .set(Some(ServerBrowserUpdate::PopulateServers(payload)));
                }
            }
            ServerBrowserUpdate::UpdateServer(response) => {
                self.update_pinged_servers(&[response]);
            }
            ServerBrowserUpdate::BatchUpdateServers(responses) => {
                self.update_pinged_servers(&responses);
            }
            ServerBrowserUpdate::RefreshDetails => {
                if self.root.visible() {
                    if let Some(selected_idx) = self.list_pane.selected_index() {
                        let server = &self.state.borrow()[selected_idx];
                        self.details_pane.populate(Some(server));
                    }
                }
            }
        }
    }

    fn populate_servers(&self, all_servers: Vec<Server>) {
        {
            let mut state = self.state.borrow_mut();
            state.update(|servers, _, _| {
                *servers = all_servers;
                Reindex::all()
            });
        }

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

        let state = Rc::clone(&self.state);
        self.list_pane.populate(state);

        self.set_total_player_count(0);

        if let Err(err) = (self.on_action)(ServerBrowserAction::PingServers(ping_requests)) {
            error!(self.logger, "Error pinging server list"; "error" => %err);
            alert_error(ERR_PINGING_SERVERS, &err);
        }
    }

    fn update_pinged_servers(&self, updates: &[PingResponse]) {
        if self.refreshing.get() {
            return;
        }

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
                    if Self::update_server(server, update, filter, &mut total_players) {
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

    fn update_server(
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

impl FilterHolder for ServerBrowser {
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

fn weekday_name(weekday: Weekday) -> &'static str {
    match weekday {
        Weekday::Mon => "Mon",
        Weekday::Tue => "Tue",
        Weekday::Wed => "Wed",
        Weekday::Thu => "Thu",
        Weekday::Fri => "Fri",
        Weekday::Sat => "Sat",
        Weekday::Sun => "Sun",
    }
}
