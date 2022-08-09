use std::collections::{HashMap, VecDeque};
use std::io::Result;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use governor::clock::{QuantaClock, QuantaInstant};
use governor::middleware::NoOpMiddleware;
use governor::state::{InMemoryState, NotKeyed};
use governor::Quota;
use tokio::net::UdpSocket;
use tokio::task::JoinHandle;

use crate::net::is_valid_ip;
use crate::servers::Server;

#[derive(Debug)]
pub struct ServerQueryRequest {
    server_idx: usize,
    addr: SocketAddr,
}

impl ServerQueryRequest {
    pub fn for_server(server_idx: usize, server: &Server) -> Option<Self> {
        let port: u16 = (server.port + 1).try_into().ok()?;
        if !is_valid_ip(server.ip()) {
            return None;
        }

        Some(Self {
            server_idx,
            addr: SocketAddr::new(*server.ip(), port),
        })
    }
}

#[derive(Debug)]
pub struct ServerQueryResponse {
    pub server_idx: usize,
    pub connected_players: usize,
    pub age: Duration,
    pub round_trip: Duration,
}

pub struct ServerQueryClient {
    build_id: u32,
    socket: Arc<UdpSocket>,
    unsent: Arc<Mutex<UnsentRequests>>,
    pending: Arc<Mutex<HashMap<SocketAddr, PendingQuery>>>,
    rate_limiter: Arc<RateLimiter>,
}

impl ServerQueryClient {
    pub fn new(
        build_id: u32,
        on_response: impl Fn(ServerQueryResponse) + Send + 'static,
    ) -> Result<Self> {
        let bind_addr = SocketAddr::from(([0, 0, 0, 0], 0));
        let socket = Arc::new({
            let socket = std::net::UdpSocket::bind(bind_addr)?;
            socket.set_nonblocking(true)?;
            UdpSocket::from_std(socket)?
        });

        let pending = Arc::new(Mutex::new(HashMap::new()));

        Self::spawn_receiver(Arc::clone(&socket), Arc::clone(&pending), on_response);

        Ok(Self {
            build_id,
            socket,
            unsent: Arc::new(Mutex::new(UnsentRequests::new())),
            pending,
            rate_limiter: Arc::new(RateLimiter::direct(Quota::per_second(
                500u32.try_into().unwrap(),
            ))),
        })
    }

    pub fn send<R: IntoIterator<Item = ServerQueryRequest>>(&self, requests: R) {
        let mut unsent = self.unsent.lock().unwrap();
        unsent.requests.extend(requests);
        if let None = unsent.task {
            unsent.task = Some(self.spawn_sender());
        }
    }

    fn spawn_receiver(
        socket: Arc<UdpSocket>,
        pending: Arc<Mutex<HashMap<SocketAddr, PendingQuery>>>,
        on_response: impl Fn(ServerQueryResponse) + Send + 'static,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut buf = [0; 16];
            loop {
                // TODO: What to do on error? Just silently drop like this?
                if let Ok((size, addr)) = socket.recv_from(&mut buf).await {
                    let received_timestamp = Instant::now();
                    if size != 16 {
                        continue;
                    }
                    let request = {
                        match pending.lock().unwrap().remove(&addr) {
                            Some(req) => req,
                            None => continue,
                        }
                    };
                    let players = i32::max(0, i32::from_le_bytes(buf[..4].try_into().unwrap()));
                    let age = Duration::from_secs(u64::from_le_bytes(buf[8..].try_into().unwrap()));

                    let response = ServerQueryResponse {
                        server_idx: request.idx,
                        connected_players: players as _,
                        age,
                        round_trip: received_timestamp - request.sent_timestamp,
                    };
                    on_response(response);
                }
            }
        })
    }

    fn spawn_sender(&self) -> JoinHandle<()> {
        let socket = Arc::clone(&self.socket);
        let unsent = Arc::clone(&self.unsent);
        let pending = Arc::clone(&self.pending);
        let rate_limiter = Arc::clone(&self.rate_limiter);
        let req_packet = self.build_id.to_be_bytes();
        tokio::spawn(async move {
            loop {
                let next = {
                    let mut unsent = unsent.lock().unwrap();
                    match unsent.requests.pop_front() {
                        Some(request) => request,
                        None => {
                            unsent.task = None;
                            return;
                        }
                    }
                };
                rate_limiter.until_ready().await;
                if socket.send_to(&req_packet, next.addr).await.is_err() {
                    // TODO: Retry? Or just silently drop like this?
                    continue;
                }
                let sent_timestamp = Instant::now();
                // FIXME: Race!
                {
                    let mut pending = pending.lock().unwrap();
                    // TODO: What about dupes? Just silently drop like this?
                    pending
                        .entry(next.addr)
                        .or_insert_with_key(|_| PendingQuery {
                            idx: next.server_idx,
                            sent_timestamp,
                        });
                }
            }
        })
    }
}

struct PendingQuery {
    idx: usize,
    sent_timestamp: Instant,
}

struct UnsentRequests {
    requests: VecDeque<ServerQueryRequest>,
    task: Option<JoinHandle<()>>,
}

type RateLimiter =
    governor::RateLimiter<NotKeyed, InMemoryState, QuantaClock, NoOpMiddleware<QuantaInstant>>;

impl UnsentRequests {
    fn new() -> Self {
        Self {
            requests: VecDeque::new(),
            task: None,
        }
    }
}
