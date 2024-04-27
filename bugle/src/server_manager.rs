use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::Arc;

use anyhow::Result;
use dynabus::Bus;
use slog::{debug, error, trace, warn, Logger};
use uuid::Uuid;

use crate::bus::AppBus;
use crate::game::{Game, ServerRef, Session};
use crate::gui::{PopulateServers, ProcessPongs, UpdateLastSession, UpdateServer};
use crate::servers::{Confidence, PingRequest, PingResponse, SavedServers, Server, Similarity};
use crate::util::weak_cb;
use crate::workers::{PongReceived, ServerLoaderWorker, ServersLoaded};
use crate::Idle;

pub struct ServerManager {
    logger: Logger,
    bus: Rc<RefCell<AppBus>>,
    game: Arc<Game>,
    saved_servers: Option<RefCell<SavedServers>>,
    is_loading: Cell<bool>,
    pong_accumulator: RefCell<Vec<PingResponse>>,
    worker: Arc<ServerLoaderWorker>,
}

impl ServerManager {
    pub fn new(logger: &Logger, bus: Rc<RefCell<AppBus>>, game: Arc<Game>) -> Rc<Self> {
        let logger = logger.clone();

        let saved_servers = match SavedServers::new() {
            Ok(mut servers) => {
                if let Err(err) = servers.load() {
                    warn!(
                        logger,
                        "Error loading the saved servers list";
                        "path" => servers.path().display(),
                        "error" => %err,
                    );
                }
                Some(RefCell::new(servers))
            }
            Err(err) => {
                warn!(logger, "Error opening the saved servers list"; "error" => %err);
                None
            }
        };

        let worker =
            ServerLoaderWorker::new(&logger, Arc::clone(&game), bus.borrow().sender().clone());

        let this = Rc::new(Self {
            logger,
            bus,
            game,
            saved_servers,
            is_loading: Cell::new(false),
            pong_accumulator: RefCell::new(Vec::new()),
            worker,
        });

        {
            let mut bus = this.bus.borrow_mut();
            bus.subscribe_consumer(weak_cb!(
                [this] => |ServersLoaded(payload)| this.servers_loaded(payload)
            ));
            bus.subscribe_consumer(weak_cb!(
                [this] => |PongReceived(pong)| this.pong_received(pong)
            ));
            bus.subscribe_observer(weak_cb!([this] => |&Idle| this.process_pongs()));
        }

        this
    }

    pub fn load_server_list(&self) {
        if let Some(servers) = self.saved_servers.as_ref() {
            let servers = servers.borrow();
            if !servers.is_empty() {
                self.bus.borrow().publish(PopulateServers {
                    payload: Ok(servers.iter().cloned().collect()),
                    done: false,
                });
            }
        }
        self.is_loading.set(true);
        self.worker.load_servers();
    }

    pub fn is_loading(&self) -> bool {
        self.is_loading.get()
    }

    pub fn ping_servers(&self, requests: Vec<PingRequest>) -> Result<()> {
        self.worker.ping_servers(requests)
    }

    pub fn ping_server(&self, request: PingRequest) -> Result<()> {
        self.worker.ping_server(request)
    }

    pub fn can_save_servers(&self) -> bool {
        self.saved_servers.is_some()
    }

    pub fn save_server(&self, server: Server, idx: Option<usize>) -> Result<()> {
        let servers = self.saved_servers.as_ref().unwrap();
        let mut servers = servers.borrow_mut();
        let id = servers.add(server);
        servers.save()?;
        self.bus
            .borrow()
            .sender()
            .send(UpdateServer {
                idx,
                server: servers[id].clone(),
            })
            .unwrap();
        Ok(())
    }

    pub fn unsave_server(&self, mut server: Server, idx: Option<usize>) -> Result<()> {
        let servers = self.saved_servers.as_ref().unwrap();
        let mut servers = servers.borrow_mut();
        servers.remove(server.saved_id.unwrap());
        servers.save()?;

        server.saved_id = None;
        if !server.merged {
            server.tombstone = true;
        }
        self.bus
            .borrow()
            .sender()
            .send(UpdateServer { idx, server })
            .unwrap();
        Ok(())
    }

    fn servers_loaded(&self, mut payload: Result<Vec<Server>>) {
        match payload.as_mut() {
            Ok(servers) => {
                self.merge_server_list(servers, Confidence::High);

                match self.game.load_favorites() {
                    Err(err) => {
                        warn!(self.logger, "Failed to load favorites"; "error" => %err);
                    }
                    Ok(favorites) => {
                        for server in servers.iter_mut() {
                            server.check_favorites(&favorites);
                        }
                    }
                }

                let build_id = self.game.build_id();
                for server in servers.iter_mut() {
                    server.validate_build(build_id);
                    server.prepare_for_ping();
                }

                let mut last_session = self.game.last_session();
                if let Some(Session::Online(server_ref)) = &mut *last_session {
                    let addr = match server_ref {
                        ServerRef::Known(server) => server.game_addr().unwrap(),
                        ServerRef::Unknown(addr) => *addr,
                    };
                    let server = servers
                        .iter()
                        .filter(|server| server.is_valid())
                        .find(|server| server.game_addr().unwrap() == addr);
                    *server_ref = match server {
                        Some(server) => ServerRef::Known(server.clone()),
                        None => ServerRef::Unknown(addr),
                    };
                    debug!(
                        self.logger,
                        "Determined last session server";
                        "server" => ?server_ref
                    );
                }
            }
            Err(err) => error!(&self.logger, "Error fetching server list"; "error" => %err),
        }
        self.is_loading.set(false);
        self.bus.borrow().publish(UpdateLastSession);
        self.bus.borrow().publish(PopulateServers {
            payload,
            done: true,
        });
    }

    fn pong_received(&self, pong: PingResponse) {
        self.pong_accumulator.borrow_mut().push(pong);
    }

    fn process_pongs(&self) {
        let mut pong_accumulator = self.pong_accumulator.borrow_mut();
        match pong_accumulator.len() {
            0 => (),
            1 => {
                self.bus
                    .borrow()
                    .publish(ProcessPongs::One(pong_accumulator.pop().unwrap()));
            }
            _ => {
                self.bus
                    .borrow()
                    .publish(ProcessPongs::Many(pong_accumulator.drain(..).collect()));
            }
        }
    }

    fn merge_server_list(&self, servers: &mut Vec<Server>, min_confidence: Confidence) {
        struct MergeCandidate {
            list_idx: usize,
            saved_id: Uuid,
            similarity: Similarity,
        }

        let saved_servers = match self.saved_servers.as_ref() {
            Some(saved) => saved,
            None => return,
        };
        let mut saved_servers = saved_servers.borrow_mut();

        debug!(
            self.logger,
            "Merging server lists";
            "num_listed" => servers.len(),
            "num_saved" => saved_servers.len(),
        );

        let mut merge_candidates = Vec::new();
        let mut matches = HashSet::new();
        for (list_idx, list_server) in servers.iter().enumerate() {
            matches.extend(saved_servers.by_id(&list_server.id));
            matches.extend(saved_servers.by_name(&list_server.name));
            matches.extend(saved_servers.by_addr(list_server.ip, list_server.port));
            for saved_id in matches.drain() {
                let score = saved_servers[saved_id].similarity(list_server);
                merge_candidates.push(MergeCandidate {
                    list_idx,
                    saved_id,
                    similarity: score,
                });
            }
        }

        merge_candidates.sort_by(|lhs, rhs| rhs.similarity.cmp(&lhs.similarity));
        let mut tombstones = Vec::new();
        for candidate in merge_candidates {
            if !candidate.similarity.satisfies(min_confidence) {
                break;
            }
            let list_server = &mut servers[candidate.list_idx];
            let saved_server = &mut saved_servers[candidate.saved_id];
            if list_server.tombstone || saved_server.merged {
                continue;
            }
            trace!(
                self.logger,
                "Merging servers";
                "listed" => ?list_server,
                "saved" => ?saved_server,
                "similarity" => ?candidate.similarity,
            );
            saved_server.merge_from(list_server);
            tombstones.push(candidate.list_idx);
        }

        if !tombstones.is_empty() {
            saved_servers.reindex();
            if let Err(err) = saved_servers.save() {
                warn!(self.logger, "Error saving merged servers"; "error" => %err);
            }
        }

        tombstones.sort();
        for tombstone_idx in tombstones.into_iter().rev() {
            servers.swap_remove(tombstone_idx);
        }

        servers.extend(saved_servers.iter().cloned());
    }
}
