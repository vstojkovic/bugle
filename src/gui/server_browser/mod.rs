use std::cell::RefCell;
use std::net::SocketAddr;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::Result;
use fltk::dialog;
use fltk::group::{Group, Tile};
use fltk::prelude::*;

use crate::game::Maps;
use crate::gui::data::{Reindex, RowFilter};
use crate::servers::{Community, FavoriteServer, Mode, PingRequest, PingResponse, Region, Server};

use self::actions_pane::{Action, ActionsPane};
use self::details_pane::DetailsPane;
use self::filter_pane::{FilterHolder, FilterPane};
use self::list_pane::ListPane;
use self::state::{Filter, SortCriteria, SortKey, TypeFilter};

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
    JoinServer(SocketAddr),
    PingServer(PingRequest),
    UpdateFavorites(Vec<FavoriteServer>),
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
    state: Rc<RefCell<ServerBrowserState>>,
}

impl ServerBrowser {
    pub fn new(
        maps: Arc<Maps>,
        on_action: impl Handler<ServerBrowserAction> + 'static,
    ) -> Rc<Self> {
        let state = Rc::new(RefCell::new(ServerBrowserState::new(
            Vec::new(),
            Default::default(),
            SortCriteria {
                key: SortKey::Name,
                ascending: true,
            },
        )));

        let mut root = Group::default_fill();

        let filter_pane = FilterPane::new(maps);

        let actions_pane = ActionsPane::new();

        let tiles = Tile::default_fill()
            .below_of(filter_pane.root(), 10)
            .stretch_to_parent(0, actions_pane.root().height());

        let upper_tile = Group::default_fill()
            .inside_parent(0, 0)
            .with_size_flex(0, tiles.height() * 3 / 4);

        let list_pane = ListPane::new(state.borrow().order());

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
                        .update_order(|criteria| *criteria = sort_criteria);
                    browser.list_pane.populate(browser.state.clone());
                    browser.set_selected_server_index(selected_idx, true);
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
                                let server = &browser.state.borrow()[server_idx];
                                let addr = SocketAddr::new(*server.ip(), server.port as _);
                                let action = ServerBrowserAction::JoinServer(addr);
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
                                (browser.on_action)(action).unwrap();
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
                            browser.list_pane.populate(browser.state.clone());
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
                            let action = ServerBrowserAction::JoinServer(addr);
                            if let Err(err) = (browser.on_action)(action) {
                                alert_error(ERR_JOINING_SERVER, &err);
                            }
                        }
                        Action::ScrollLock(scroll_lock) => {
                            browser.list_pane.set_scroll_lock(scroll_lock);
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

        (self.on_action)(ServerBrowserAction::LoadServers).unwrap();

        Box::new(move || {
            root.hide();
        })
    }

    pub fn handle_update(&self, update: ServerBrowserUpdate) {
        match update {
            ServerBrowserUpdate::PopulateServers(payload) => match payload {
                Ok(all_servers) => self.populate_servers(all_servers),
                Err(err) => super::alert_error(ERR_LOADING_SERVERS, &err),
            },
            ServerBrowserUpdate::UpdateServer(response) => {
                self.update_pinged_servers(&[response]);
            }
            ServerBrowserUpdate::BatchUpdateServers(responses) => {
                self.update_pinged_servers(&responses);
            }
        }
    }

    fn populate_servers(&self, all_servers: Vec<Server>) {
        self.state.borrow_mut().update(|servers, _, _| {
            *servers = all_servers;
            Reindex::all()
        });
        self.list_pane.populate(self.state.clone());
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
            state.update(|all_servers, filter, sort_criteria| {
                mutator(all_servers, &mut updated_indices, filter, sort_criteria)
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
        server.connected_players = Some(update.connected_players);
        server.age = Some(update.age);
        server.ping = Some(update.round_trip);
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
    }
}

const ERR_LOADING_SERVERS: &str = "Error while loading the server list.";
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
