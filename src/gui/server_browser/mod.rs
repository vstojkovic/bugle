use std::cell::RefCell;
use std::net::SocketAddr;
use std::ops::{Index, IndexMut};
use std::rc::Rc;

use anyhow::Result;
use fltk::group::{Group, Tile};
use fltk::prelude::*;

use crate::servers::{
    Filter, Mode, Region, Server, ServerList, ServerListView, ServerQueryResponse, SortCriteria,
    SortKey,
};

use self::actions_pane::{Action, ActionsPane};
use self::details_pane::DetailsPane;
use self::filter_pane::{FilterHolder, FilterPane};
use self::list_pane::ListPane;

use super::prelude::*;
use super::{alert_error, alert_not_implemented, CleanupFn, Handler};

mod actions_pane;
mod details_pane;
mod filter_pane;
mod list_pane;

pub enum ServerBrowserAction {
    LoadServers,
    JoinServer(SocketAddr),
}

pub enum ServerBrowserUpdate {
    PopulateServers(Result<Vec<Server>>),
    UpdateServer(ServerQueryResponse),
    BatchUpdateServers(Vec<ServerQueryResponse>),
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

struct ServerBrowserState {
    servers: ServerListView<ServerListView<Vec<Server>, Filter>, SortCriteria>,
}

impl ServerBrowserState {
    fn new(all_servers: Vec<Server>, filter: Filter, sort_criteria: SortCriteria) -> Self {
        let filtered_servers = ServerListView::new(all_servers, filter);
        let sorted_servers = ServerListView::new(filtered_servers, sort_criteria);
        Self {
            servers: sorted_servers,
        }
    }

    fn set_servers(&mut self, servers: Vec<Server>) {
        self.servers.mutate(move |filtered_servers, _| {
            filtered_servers.mutate(move |all_servers, _| {
                *all_servers = servers;
                true
            })
        });
    }

    fn filter(&self) -> &Filter {
        &self.servers.source().indexer()
    }

    fn change_filter(&mut self, mut mutator: impl FnMut(&mut Filter)) {
        self.servers.mutate(move |filtered_servers, _| {
            filtered_servers.mutate(move |_, filter| {
                mutator(filter);
                true
            })
        });
    }

    fn sort_criteria(&self) -> &SortCriteria {
        &self.servers.indexer()
    }

    fn set_sort_criteria(&mut self, criteria: SortCriteria) {
        self.servers.mutate(move |_, sort_criteria| {
            *sort_criteria = criteria;
            true
        });
    }

    fn update_servers(
        &mut self,
        mutator: impl FnOnce(&mut Vec<Server>, &Filter) -> bool,
        should_sort: impl FnOnce(&SortCriteria) -> bool,
    ) -> bool {
        self.servers.mutate(move |filtered_servers, sort_criteria| {
            let should_reindex =
                filtered_servers.mutate(move |all_servers, filter| mutator(all_servers, filter));
            should_reindex || should_sort(sort_criteria)
        })
    }

    fn to_display_index(&self, index: usize) -> Option<usize> {
        self.servers
            .source()
            .from_source_index(index)
            .and_then(|index| self.servers.from_source_index(index))
    }
}

impl Index<usize> for ServerBrowserState {
    type Output = Server;
    fn index(&self, index: usize) -> &Self::Output {
        &self.servers[index]
    }
}

impl IndexMut<usize> for ServerBrowserState {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.servers[index]
    }
}

impl ServerList for ServerBrowserState {
    fn len(&self) -> usize {
        self.servers.len()
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
    pub fn new(build_id: u32, on_action: impl Handler<ServerBrowserAction> + 'static) -> Rc<Self> {
        let mut filter: Filter = Default::default();
        filter.set_build_id(build_id);
        let state = Rc::new(RefCell::new(ServerBrowserState::new(
            Vec::new(),
            filter,
            SortCriteria {
                key: SortKey::Name,
                ascending: true,
            },
        )));

        let mut root = Group::default_fill();

        let filter_pane = FilterPane::new(build_id);

        let actions_pane = ActionsPane::new();

        let tiles = Tile::default_fill()
            .below_of(filter_pane.root(), 10)
            .stretch_to_parent(0, actions_pane.root().height());

        let upper_tile = Group::default_fill()
            .inside_parent(0, 0)
            .with_size_flex(0, tiles.height() * 3 / 4);

        let list_pane = ListPane::new(state.borrow().sort_criteria());

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
                    browser.state.borrow_mut().set_sort_criteria(sort_criteria);
                    browser.list_pane.populate(browser.state.clone());
                }
            });
        }
        {
            let browser = Rc::downgrade(&browser);
            list_pane.set_on_server_selected(move |server| {
                if let Some(browser) = browser.upgrade() {
                    browser.details_pane.populate(server);
                    browser
                        .actions_pane
                        .set_server_actions_enabled(server.is_valid());
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
                        _ => alert_not_implemented(),
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
                self.update_servers(&[response]);
            }
            ServerBrowserUpdate::BatchUpdateServers(responses) => {
                self.update_servers(&responses);
            }
        }
    }

    fn populate_servers(&self, all_servers: Vec<Server>) {
        self.state.borrow_mut().set_servers(all_servers);
        self.list_pane.populate(self.state.clone());
    }

    fn update_servers(&self, updates: &[ServerQueryResponse]) {
        let mut updated_indices: Vec<usize> = Vec::with_capacity(updates.len());
        let repopulate = {
            let mut state = self.state.borrow_mut();
            state.update_servers(
                |all_servers, filter| {
                    let mut should_reindex = false;
                    for update in updates {
                        let server = match all_servers.get_mut(update.server_idx) {
                            Some(server) => server,
                            None => continue,
                        };
                        updated_indices.push(update.server_idx);
                        if Self::update_server(server, update, filter) {
                            should_reindex = true;
                        }
                    }
                    should_reindex
                },
                |sort_criteria| {
                    (sort_criteria.key == SortKey::Players)
                        || (sort_criteria.key == SortKey::Age)
                        || (sort_criteria.key == SortKey::Ping)
                },
            )
        };
        if repopulate {
            self.list_pane.populate(self.state.clone());
        } else {
            let state = self.state.borrow();
            self.list_pane.update(
                updated_indices
                    .iter()
                    .filter_map(|idx| state.to_display_index(*idx)),
            );
        };
    }

    fn update_server(server: &mut Server, update: &ServerQueryResponse, filter: &Filter) -> bool {
        let matched_before = filter.matches(server);
        server.connected_players = Some(update.connected_players);
        server.age = Some(update.age);
        server.ping = Some(update.round_trip);
        filter.matches(server) != matched_before
    }
}

impl FilterHolder for ServerBrowser {
    fn access_filter(&self, accessor: impl FnOnce(&Filter)) {
        accessor(self.state.borrow().filter());
    }

    fn mutate_filter(&self, mutator: impl FnMut(&mut Filter)) {
        self.state.borrow_mut().change_filter(mutator);
        self.list_pane.populate(self.state.clone());
    }
}

const ERR_LOADING_SERVERS: &str = "Error while loading the server list.";
const ERR_JOINING_SERVER: &str = "Error while trying to launch the game to join the server.";

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
