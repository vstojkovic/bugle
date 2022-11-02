use std::collections::VecDeque;
use std::io::Result;
use std::net::SocketAddr;
use std::ops::Deref;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use governor::clock::{QuantaClock, QuantaInstant};
use governor::middleware::NoOpMiddleware;
use governor::state::{InMemoryState, NotKeyed};
use governor::Quota;
use linked_hash_map::{Entry, LinkedHashMap};
use slog::{info, warn, Logger};
use tokio::net::UdpSocket;
use tokio::task::JoinHandle;
use tokio::time::timeout;

use crate::net::bind_udp_socket;
use crate::servers::Server;

#[derive(Debug)]
pub struct PingRequest {
    server_idx: usize,
    pub addr: SocketAddr,
}

impl PingRequest {
    pub fn for_server(server_idx: usize, server: &Server) -> Option<Self> {
        if server.is_valid() {
            Some(Self {
                server_idx,
                addr: SocketAddr::new(*server.ip(), (server.port + 1) as _),
            })
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct PingResponse {
    pub server_idx: usize,
    pub connected_players: usize,
    pub age: Duration,
    pub round_trip: Duration,
}

pub struct PingClient {
    client_impl: Arc<ClientImpl>,
}

impl PingClient {
    pub fn new(
        logger: Logger,
        build_id: u32,
        on_response: impl Fn(PingResponse) + Send + 'static,
    ) -> Result<Self> {
        ClientImpl::new(logger, build_id, on_response).map(|client_impl| Self { client_impl })
    }
}

impl Deref for PingClient {
    type Target = Arc<ClientImpl>;
    fn deref(&self) -> &Self::Target {
        &self.client_impl
    }
}

impl Drop for PingClient {
    fn drop(&mut self) {
        let mut pending = self.client_impl.pending.lock().unwrap();
        if let Some(task) = pending.task.take() {
            info!(self.logger, "Stopping ping receiver");
            task.abort();
        }
        let mut unsent = self.client_impl.unsent.lock().unwrap();
        if let Some(task) = unsent.task.take() {
            info!(self.logger, "Stopping ping sender");
            task.abort();
        }
    }
}

pub struct ClientImpl {
    logger: Logger,
    build_id: u32,
    socket: UdpSocket,
    unsent: Mutex<UnsentRequests>,
    pending: Mutex<PendingRequests>,
}

impl ClientImpl {
    fn new(
        logger: Logger,
        build_id: u32,
        on_response: impl Fn(PingResponse) + Send + 'static,
    ) -> Result<Arc<Self>> {
        let bind_addr = SocketAddr::from(([0, 0, 0, 0], 0));
        let socket = {
            let socket = bind_udp_socket(bind_addr)?;
            socket.set_nonblocking(true)?;
            UdpSocket::from_std(socket)?
        };

        let client = Arc::new(Self {
            logger,
            build_id,
            socket,
            unsent: Mutex::new(UnsentRequests::new()),
            pending: Mutex::new(PendingRequests::new()),
        });

        client.pending.lock().unwrap().task = Some(Arc::clone(&client).spawn_receiver(on_response));

        Ok(client)
    }

    pub fn send<R: IntoIterator<Item = PingRequest>>(self: &Arc<Self>, requests: R) {
        let mut unsent = self.unsent.lock().unwrap();
        unsent.requests.extend(requests);
        self.ensure_sender(unsent);
    }

    pub fn priority_send(self: &Arc<Self>, request: PingRequest) {
        let mut unsent = self.unsent.lock().unwrap();
        unsent.requests.push_front(request);
        self.ensure_sender(unsent);
    }

    fn ensure_sender(self: &Arc<Self>, mut unsent: MutexGuard<UnsentRequests>) {
        if unsent.requests.is_empty() {
            return;
        }
        if let None = unsent.task {
            unsent.task = Some(Arc::clone(self).spawn_sender());
        }
    }

    fn spawn_receiver(
        self: Arc<Self>,
        on_response: impl Fn(PingResponse) + Send + 'static,
    ) -> JoinHandle<()> {
        info!(self.logger, "Starting ping receiver");
        tokio::spawn(Receiver::new(self, on_response).run())
    }

    fn spawn_sender(self: Arc<Self>) -> JoinHandle<()> {
        info!(self.logger, "Starting ping sender");
        tokio::spawn(Sender::new(self).run())
    }
}

type RateLimiter =
    governor::RateLimiter<NotKeyed, InMemoryState, QuantaClock, NoOpMiddleware<QuantaInstant>>;

struct PendingRequests {
    requests: LinkedHashMap<SocketAddr, PendingRequest>,
    task: Option<JoinHandle<()>>,
}

impl PendingRequests {
    fn new() -> Self {
        Self {
            requests: LinkedHashMap::new(),
            task: None,
        }
    }
}

struct PendingRequest {
    idx: usize,
    sent_timestamp: Instant,
    should_retry: bool,
}

struct UnsentRequests {
    requests: VecDeque<PingRequest>,
    task: Option<JoinHandle<()>>,
}

impl UnsentRequests {
    fn new() -> Self {
        Self {
            requests: VecDeque::new(),
            task: None,
        }
    }
}

struct Sender {
    client: Arc<ClientImpl>,
    rate_limiter: RateLimiter,
}

impl Sender {
    fn new(client: Arc<ClientImpl>) -> Self {
        Self {
            client,
            rate_limiter: RateLimiter::direct(Quota::per_second(500u32.try_into().unwrap())),
        }
    }

    async fn run(self) {
        let req_packet = self.client.build_id.to_be_bytes();
        let mut pings_sent = 0;
        loop {
            let next = {
                let mut unsent = self.client.unsent.lock().unwrap();
                match unsent.requests.pop_front() {
                    Some(request) => request,
                    None => {
                        unsent.task = None;
                        break;
                    }
                }
            };
            {
                let mut pending = self.client.pending.lock().unwrap();
                if let Entry::Occupied(mut entry) = pending.requests.entry(next.addr) {
                    if entry.get().idx == next.server_idx {
                        entry.get_mut().should_retry = true;
                    } else {
                        warn!(
                            self.client.logger,
                            "Discarding ping request for duplicate address {addr}",
                            addr = next.addr
                        );
                    }
                    continue;
                }
            }
            self.rate_limiter.until_ready().await;
            if let Err(err) = self.client.socket.send_to(&req_packet, next.addr).await {
                warn!(self.client.logger, "Failed to send ping request"; "error" => err);
                continue;
            }
            pings_sent += 1;
            let sent_timestamp = Instant::now();
            {
                let mut pending = self.client.pending.lock().unwrap();
                pending
                    .requests
                    .entry(next.addr)
                    .or_insert_with(|| PendingRequest {
                        idx: next.server_idx,
                        sent_timestamp,
                        should_retry: false,
                    });
            }
        }
        info!(self.client.logger, "Finished sending {pings_sent} pings");
    }
}

struct Receiver<F: Fn(PingResponse) + Send> {
    client: Arc<ClientImpl>,
    on_response: F,
    max_time: Duration,
}

impl<F: Fn(PingResponse) + Send> Receiver<F> {
    fn new(client: Arc<ClientImpl>, on_response: F) -> Self {
        Self {
            client,
            on_response,
            max_time: Duration::from_secs(10),
        }
    }

    async fn run(self) {
        let mut buf = [0; 16];
        loop {
            let recv_result = timeout(self.max_time, self.client.socket.recv_from(&mut buf)).await;
            if let Ok(Ok((size, addr))) = recv_result {
                self.process_packet(&buf[..size], addr);
            }
            self.handle_timeouts();
        }
    }

    fn process_packet(&self, packet: &[u8], addr: SocketAddr) {
        let received_timestamp = Instant::now();
        if packet.len() != 16 {
            return;
        }
        let request = {
            match self.client.pending.lock().unwrap().requests.remove(&addr) {
                Some(req) => req,
                None => return,
            }
        };
        let players = i32::max(0, i32::from_le_bytes(packet[..4].try_into().unwrap()));
        let age = Duration::from_secs(u64::from_le_bytes(packet[8..].try_into().unwrap()));

        let response = PingResponse {
            server_idx: request.idx,
            connected_players: players as _,
            age,
            round_trip: received_timestamp - request.sent_timestamp,
        };
        (self.on_response)(response);
    }

    fn handle_timeouts(&self) {
        let cutoff = Instant::now() - self.max_time;
        let mut retries = Vec::new();
        {
            let mut pending = self.client.pending.lock().unwrap();
            for entry in pending.requests.entries() {
                if entry.get().sent_timestamp > cutoff {
                    break;
                }
                if entry.get().should_retry {
                    retries.push(PingRequest {
                        server_idx: entry.get().idx,
                        addr: *entry.key(),
                    });
                }
                entry.remove();
            }
        }
        self.client.send(retries);
    }
}
