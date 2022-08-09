use std::cell::RefCell;
use std::ops::{Index, IndexMut};
use std::rc::Rc;

use anyhow::Result;
use fltk::group::{Group, Tile};
use fltk::prelude::*;

use crate::servers::{
    Filter, Mode, Region, Server, ServerList, ServerListView, ServerQueryResponse, SortCriteria,
    SortKey,
};

use self::details_pane::DetailsPane;
use self::filter_pane::{FilterHolder, FilterPane};
use self::list_pane::ListPane;

use super::prelude::*;
use super::{CleanupFn, Handler};

mod details_pane;
mod filter_pane;
mod list_pane;

pub enum ServerBrowserAction {
    LoadServers,
}

pub enum ServerBrowserUpdate {
    PopulateServers(Result<Vec<Server>>),
    UpdateServer(ServerQueryResponse),
}

struct ServerBrowserData {
    servers: ServerListView<ServerListView<Vec<Server>, Filter>, SortCriteria>,
}

impl ServerBrowserData {
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

    fn update_server(&mut self, index: usize, mutator: impl FnOnce(&mut Server)) -> bool {
        self.servers.mutate(move |filtered_servers, sort_criteria| {
            let should_reindex = filtered_servers.mutate(move |all_servers, filter| {
                let server = match all_servers.get_mut(index) {
                    Some(server) => server,
                    _ => return false,
                };
                let matched_before = filter.matches(server);
                mutator(server);
                filter.matches(server) != matched_before
            });
            should_reindex
                || (sort_criteria.key == SortKey::Players)
                || (sort_criteria.key == SortKey::Age)
                || (sort_criteria.key == SortKey::Ping)
        })
    }

    fn to_display_index(&self, index: usize) -> Option<usize> {
        self.servers
            .source()
            .from_source_index(index)
            .and_then(|index| self.servers.from_source_index(index))
    }
}

impl Index<usize> for ServerBrowserData {
    type Output = Server;
    fn index(&self, index: usize) -> &Self::Output {
        &self.servers[index]
    }
}

impl IndexMut<usize> for ServerBrowserData {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.servers[index]
    }
}

impl ServerList for ServerBrowserData {
    fn len(&self) -> usize {
        self.servers.len()
    }
}

pub(super) struct ServerBrowser {
    root: Group,
    on_action: Box<dyn Handler<ServerBrowserAction>>,
    list_pane: Rc<ListPane>,
    details_pane: DetailsPane,
    state: Rc<RefCell<ServerBrowserData>>,
}

impl ServerBrowser {
    pub fn new(build_id: u32, on_action: impl Handler<ServerBrowserAction> + 'static) -> Rc<Self> {
        let mut filter: Filter = Default::default();
        filter.set_build_id(build_id);
        let state = Rc::new(RefCell::new(ServerBrowserData::new(
            Vec::new(),
            filter,
            SortCriteria {
                key: SortKey::Name,
                ascending: true,
            },
        )));

        let mut root = Group::default_fill();

        let filter_pane = FilterPane::new(build_id);

        let tiles = Tile::default_fill()
            .below_of(filter_pane.root(), 10)
            .stretch_to_parent(0, 0);

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
            state: Rc::clone(&state),
        });

        filter_pane.set_filter_holder(Rc::clone(&browser));
        {
            let browser = Rc::downgrade(&Rc::clone(&browser));
            list_pane.set_on_sort_changed(move |sort_criteria| {
                if let Some(browser) = browser.upgrade() {
                    browser.state.borrow_mut().set_sort_criteria(sort_criteria);
                    browser.list_pane.populate(browser.state.clone());
                }
            });
        }
        {
            let browser = Rc::downgrade(&Rc::clone(&browser));
            list_pane.set_on_server_selected(move |server| {
                if let Some(browser) = browser.upgrade() {
                    browser.details_pane.populate(server)
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
                Ok(all_servers) => {
                    self.state.borrow_mut().set_servers(all_servers);
                    self.list_pane.populate(self.state.clone());
                }
                Err(err) => super::alert_error(ERR_LOADING_SERVERS, &err),
            },
            ServerBrowserUpdate::UpdateServer(response) => {
                let repopulate = {
                    let mut state = self.state.borrow_mut();
                    state.update_server(response.server_idx, |server| {
                        server.connected_players = Some(response.connected_players);
                        server.age = Some(response.age);
                        server.ping = Some(response.round_trip);
                    })
                };
                if repopulate {
                    self.list_pane.populate(self.state.clone());
                } else if let Some(index) =
                    self.state.borrow().to_display_index(response.server_idx)
                {
                    self.list_pane.update(index);
                }
            }
        }
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
