use std::cell::RefCell;
use std::rc::Rc;

use anyhow::Result;
use fltk::group::{Group, Tile};
use fltk::prelude::*;

use crate::servers::{Filter, Mode, Region, ServerList, SortCriteria, SortKey};

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
    PopulateServers(Result<ServerList>),
}

struct ServerBrowserData {
    all_servers: ServerList,
    filter: Filter,
    filtered_servers: ServerList,
    sort_criteria: SortCriteria,
    sorted_servers: ServerList,
}

impl ServerBrowserData {
    fn new(all_servers: ServerList, filter: Filter, sort_criteria: SortCriteria) -> Self {
        let filtered_servers = all_servers.filtered(&filter);
        let sorted_servers = filtered_servers.sorted(sort_criteria);
        Self {
            all_servers,
            filter,
            filtered_servers,
            sort_criteria,
            sorted_servers,
        }
    }

    fn set_servers(&mut self, all_servers: ServerList) {
        self.all_servers = all_servers;
        self.update_filtered_servers();
    }

    fn filter(&self) -> &Filter {
        &self.filter
    }

    fn change_filter(&mut self, mut mutator: impl FnMut(&mut Filter)) {
        mutator(&mut self.filter);
        self.update_filtered_servers();
    }

    fn sort_criteria(&self) -> &SortCriteria {
        &self.sort_criteria
    }

    fn set_sort_criteria(&mut self, sort_criteria: SortCriteria) {
        self.sort_criteria = sort_criteria;
        self.update_sorted_servers();
    }

    fn servers(&self) -> ServerList {
        self.sorted_servers.clone()
    }

    fn update_filtered_servers(&mut self) {
        self.filtered_servers = self.all_servers.filtered(&self.filter);
        self.update_sorted_servers()
    }

    fn update_sorted_servers(&mut self) {
        self.sorted_servers = self.filtered_servers.sorted(self.sort_criteria);
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
            ServerList::empty(),
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
                    browser.list_pane.populate(browser.state.borrow().servers());
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
                    self.list_pane.populate(self.state.borrow().servers());
                }
                Err(err) => super::alert_error(ERR_LOADING_SERVERS, &err),
            },
        }
    }
}

impl FilterHolder for ServerBrowser {
    fn access_filter(&self, accessor: impl FnOnce(&Filter)) {
        accessor(self.state.borrow().filter());
    }

    fn mutate_filter(&self, mutator: impl FnMut(&mut Filter)) {
        self.state.borrow_mut().change_filter(mutator);
        self.list_pane.populate(self.state.borrow().servers());
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
