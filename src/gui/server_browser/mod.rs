use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::Result;
use fltk::dialog;
use fltk::group::{Group, Tile};
use fltk::prelude::*;
use strum::IntoEnumIterator;

use crate::config::ServerBrowserConfig;
use crate::game::Maps;
use crate::gui::data::{Reindex, RowFilter};
use crate::servers::{
    Community, FavoriteServer, Mode, PingRequest, PingResponse, PingResult, Region, Server,
    SortCriteria, SortKey, TypeFilter,
};

use self::actions_pane::{Action, ActionsPane};
use self::details_pane::DetailsPane;
use self::filter_pane::{FilterChange, FilterHolder, FilterPane};
use self::list_pane::ListPane;
use self::state::{Filter, SortOrder};

use super::data::IterableTableSource;
use super::prelude::*;
use super::{alert_error, CleanupFn, Handler};

mod actions_pane;
mod details_pane;
mod filter_pane;
mod list_pane;
mod state;

use state::ServerBrowserState;

pub enum ServerBrowserAction {
    LoadServers,
    JoinServer {
        addr: SocketAddr,
        battleye_required: bool,
    },
    PingServer(PingRequest),
    PingServers(Vec<PingRequest>),
    UpdateFavorites(Vec<FavoriteServer>),
    UpdateConfig(ServerBrowserConfig),
}

pub enum ServerBrowserUpdate {
    PopulateServers(Result<Vec<Server>>),
    UpdateServer(PingResponse),
    BatchUpdateServers(Vec<PingResponse>),
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
    root: Group,
    on_action: Box<dyn Handler<ServerBrowserAction>>,
    list_pane: Rc<ListPane>,
    details_pane: DetailsPane,
    actions_pane: Rc<ActionsPane>,
    pending_update: Rc<Cell<Option<ServerBrowserUpdate>>>,
    state: Rc<RefCell<ServerBrowserState>>,
}

impl ServerBrowser {
    pub fn new(
        maps: Arc<Maps>,
        config: &ServerBrowserConfig,
        on_action: impl Handler<ServerBrowserAction> + 'static,
    ) -> Rc<Self> {
        let state = Rc::new(RefCell::new(ServerBrowserState::new(
            Vec::new(),
            Filter::from_config(config),
            SortOrder::new(config.sort_criteria, region_sort_order()),
        )));

        let mut root = Group::default_fill();

        let filter_pane = FilterPane::new(maps);

        let actions_pane = ActionsPane::new(config.scroll_lock);

        let tiles = Tile::default_fill()
            .below_of(filter_pane.root(), 10)
            .stretch_to_parent(0, actions_pane.root().height());

        let upper_tile = Group::default_fill()
            .inside_parent(0, 0)
            .with_size_flex(0, tiles.height() * 3 / 4);

        let list_pane = ListPane::new(&state.borrow().order().criteria, config.scroll_lock);

        upper_tile.end();

        let lower_tile = Group::default_fill()
            .below_of(&upper_tile, 0)
            .stretch_to_parent(0, 0);

        let details_pane = DetailsPane::new();

        lower_tile.end();

        tiles.end();

        root.end();
        root.hide();

        let browser = Rc::new(Self {
            root,
            on_action: Box::new(on_action),
            list_pane: Rc::clone(&list_pane),
            details_pane,
            actions_pane: Rc::clone(&actions_pane),
            pending_update: Rc::new(Cell::new(None)),
            state: Rc::clone(&state),
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
                                    let addr = SocketAddr::new(*server.ip(), server.port as _);
                                    ServerBrowserAction::JoinServer {
                                        addr,
                                        battleye_required: server.battleye_required,
                                    }
                                };
                                if let Err(err) = (browser.on_action)(action) {
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
                                    alert_error(ERR_UPDATING_FAVORITES, &err);
                                }
                            }
                        }
                        Action::Refresh => {
                            {
                                let mut state = browser.state.borrow_mut();
                                state.update_source(Vec::clear);
                            }
                            browser.list_pane.mark_refreshing();
                            browser.list_pane.set_selected_index(None, false);
                            (browser.on_action)(ServerBrowserAction::LoadServers).unwrap();
                        }
                        Action::DirectConnect => {
                            let addr = dialog::input_default("Connect to:", "127.0.0.1:7777");
                            let addr = match addr {
                                Some(str) => SocketAddr::from_str(&str).map_err(anyhow::Error::msg),
                                None => return,
                            };
                            let addr = match addr {
                                Ok(addr) => addr,
                                Err(err) => return alert_error(ERR_INVALID_ADDR, &err),
                            };
                            let action = ServerBrowserAction::JoinServer {
                                addr,
                                battleye_required: true,
                            };
                            if let Err(err) = (browser.on_action)(action) {
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
            ServerBrowserUpdate::PopulateServers(payload) => {
                if self.root.visible() {
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

        if let Err(err) = (self.on_action)(ServerBrowserAction::PingServers(ping_requests)) {
            alert_error(ERR_PINGING_SERVERS, &err);
        }
    }

    fn update_pinged_servers(&self, updates: &[PingResponse]) {
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
                    if Self::update_server(server, update, filter) {
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

    fn update_server(server: &mut Server, update: &PingResponse, filter: &Filter) -> bool {
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
            type_filter: filter.type_filter(),
            mode: filter.mode(),
            region: filter.region(),
            battleye_required: filter.battleye_required(),
            include_invalid: filter.include_invalid(),
            include_password_protected: filter.include_password_protected(),
            mods: filter.mods(),
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

    fn mutate_filter(&self, change: FilterChange, mutator: impl FnOnce(&mut Filter)) {
        let selected_idx = self.selected_server_index();
        self.state.borrow_mut().update_filter(mutator);
        self.list_pane.populate(self.state.clone());
        self.set_selected_server_index(selected_idx, false);
        if (change != FilterChange::Name) && (change != FilterChange::Map) {
            self.update_config();
        }
    }
}

const ERR_LOADING_SERVERS: &str = "Error while loading the server list.";
const ERR_PINGING_SERVERS: &str = "Error while pinging servers.";
const ERR_JOINING_SERVER: &str = "Error while trying to launch the game to join the server.";
const ERR_INVALID_ADDR: &str = "Invalid server address.";
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
