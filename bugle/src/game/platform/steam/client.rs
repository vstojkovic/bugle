use std::cell::{RefCell, RefMut};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::rc::Rc;

use dynabus::mpsc::BusSender;
use dynabus::Bus;
use fltk::app::{self, TimeoutHandle};
use slog::{debug, o, trace, warn, Logger};
use steamworks::networking_types::NetworkingIdentity;
use steamworks::{
    AuthTicket, CallbackHandle, Client, ClientManager, DownloadItemResult, ItemState,
    PublishedFileId, SingleClient, SteamError, User,
};
use tokio::task::JoinHandle;

use crate::auth::PlatformUser;
use crate::bus::{AppBus, AppSender};
use crate::game::Branch;
use crate::logger::IteratorFormatter;
use crate::util::weak_cb;

use super::app_id;

pub struct SteamClient {
    logger: Logger,
    branch: Branch,
    api: RefCell<Option<SteamAPI>>,
    api_initializer: RefCell<Option<JoinHandle<()>>>,
    tx: BusSender<AppSender>,
    ticket: RefCell<Option<Rc<SteamTicket>>>,
    downloads: RefCell<Downloads>,
    callback_timer: Rc<RefCell<CallbackTimer>>,
    // explicitly make sure SteamClient is neither Send nor Sync, see CallbackWrapper below
    _marker: PhantomData<*const ()>,
}

#[derive(dynabus::Event)]
pub struct PlatformReady;

pub type DownloadCallback = Rc<dyn Fn(Option<SteamError>)>;

struct SteamAPI {
    client: Client,
    cb_runner: SingleClient,
}

#[derive(dynabus::Event)]
struct InitializerDone(Option<SteamAPI>);

#[derive(Default)]
struct Downloads {
    api_callback_handle: Option<CallbackHandle>,
    dispatch_map: HashMap<PublishedFileId, DownloadCallback>,
}

impl SteamClient {
    pub(super) fn new(logger: &Logger, branch: Branch, bus: Rc<RefCell<AppBus>>) -> Rc<Self> {
        let logger = logger.new(o!("branch" => format!("{:?}", branch)));
        let tx = bus.borrow().sender().clone();
        let callback_timer = Rc::new(RefCell::new(CallbackTimer::new(&logger)));
        let this = Rc::new(Self {
            logger,
            branch,
            api: RefCell::new(init_client(branch)),
            api_initializer: RefCell::new(None),
            tx,
            ticket: RefCell::new(None),
            downloads: RefCell::new(Default::default()),
            callback_timer,
            _marker: PhantomData,
        });

        {
            let mut bus = bus.borrow_mut();
            bus.subscribe_consumer(weak_cb!([this] => |InitializerDone(maybe_api)| {
                let ready = maybe_api.is_some();
                *this.api.borrow_mut() = maybe_api;
                *this.api_initializer.borrow_mut() = None;
                if ready {
                    this.tx.send(PlatformReady).ok();
                }
            }));
        }

        this
    }

    pub fn branch(&self) -> Branch {
        self.branch
    }

    pub fn can_launch(&self) -> bool {
        let client = self.check_client();
        client.is_some()
    }

    pub fn can_play_online(&self) -> bool {
        match self.check_client() {
            Some(client) => client.user().logged_on(),
            None => false,
        }
    }

    pub fn user(&self) -> Option<PlatformUser> {
        self.check_client().as_ref().map(|client| PlatformUser {
            id: client.user().steam_id().raw().to_string(),
            display_name: client.friends().name(),
        })
    }

    pub fn auth_ticket(&self) -> Option<Rc<SteamTicket>> {
        let mut ticket = self.ticket.borrow_mut();
        if ticket.is_none() {
            *ticket = self.check_client().as_ref().and_then(|client| {
                let user = client.user();
                if user.logged_on() {
                    Some(Rc::new(SteamTicket::new(user)))
                } else {
                    None
                }
            });
        }
        ticket.clone()
    }

    pub fn query_mods(
        &self,
        mod_ids: impl Iterator<Item = u64> + Clone,
        callback: impl Fn(Vec<(u64, String)>) + 'static,
    ) {
        use std::convert::identity;

        debug!(
            self.logger,
            "Querying published mods";
            "mod_ids" => %IteratorFormatter(mod_ids.clone())
        );
        let client = match self.check_client() {
            Some(client) => client,
            None => {
                trace!(self.logger, "Cannot query mods, Steam is not running");
                return;
            }
        };

        let file_ids = mod_ids.map(PublishedFileId).collect();
        let query = match client.ugc().query_items(file_ids) {
            Ok(query) => query,
            Err(err) => {
                warn!(self.logger, "Error creating UGC query"; "error" => %err);
                return;
            }
        };
        let callback = {
            let callback_timer = Rc::clone(&self.callback_timer);
            move |results| {
                callback(results);
                callback_timer.borrow_mut().callback_completed();
            }
        };
        query.fetch({
            let logger = self.logger.clone();
            let callback = CallbackWrapper(callback);
            move |results| {
                trace!(logger, "Received UGC query results");
                let results = match results {
                    Ok(results) => results,
                    Err(err) => {
                        warn!(logger, "UGC query returned an error"; "error" => %err);
                        return;
                    }
                };
                let results = results
                    .iter()
                    .filter_map(identity)
                    .map(|result| (result.published_file_id.0, result.title.clone()))
                    .collect();
                callback.call_once(results);
            }
        });
        self.callback_timer.borrow_mut().callback_pending();
    }

    pub fn mod_needs_update(&self, mod_id: u64) -> Option<bool> {
        self.check_client().map(|client| {
            client
                .ugc()
                .item_state(PublishedFileId(mod_id))
                .contains(ItemState::NEEDS_UPDATE)
        })
    }

    pub fn start_mod_update(
        self: &Rc<Self>,
        mod_id: u64,
        callback: DownloadCallback,
    ) -> Option<bool> {
        let client = self.check_client()?;
        let mod_id = PublishedFileId(mod_id);
        let success = client.ugc().download_item(mod_id, false);
        if success {
            let mut downloads = self.downloads.borrow_mut();
            downloads.dispatch_map.insert(mod_id, callback);
            if downloads.api_callback_handle.is_none() {
                let mut api_callback = CallbackWrapper(weak_cb!(
                    [this = self] => |result| this.handle_download_result(result)
                ));
                downloads.api_callback_handle =
                    Some(client.register_callback(move |result| api_callback.call_mut(result)));
                self.callback_timer.borrow_mut().callback_pending();
            }
        }
        Some(success)
    }

    pub fn download_progress(&self, mod_id: u64) -> Option<(u64, u64)> {
        let file_id = PublishedFileId(mod_id);
        self.check_client()
            .and_then(|client| client.ugc().item_download_info(file_id))
    }

    pub fn run_callbacks(&self) {
        if let Some(api) = &*self.api.borrow() {
            api.cb_runner.run_callbacks();
        }
    }

    fn check_client(&self) -> Option<RefMut<Client>> {
        let api = self.api.borrow_mut();
        if api.is_none() {
            let mut initializer = self.api_initializer.borrow_mut();
            if initializer.is_none() {
                let branch = self.branch;
                let tx = self.tx.clone();
                *initializer = Some(tokio::spawn(async move {
                    let maybe_api = init_client(branch);
                    tx.send(InitializerDone(maybe_api)).ok();
                }));
            }
        }
        RefMut::filter_map(api, |opt| opt.as_mut().map(|api| &mut api.client)).ok()
    }

    fn handle_download_result(&self, result: DownloadItemResult) {
        let mut downloads = self.downloads.borrow_mut();
        if let Some(callback) = downloads.dispatch_map.remove(&result.published_file_id) {
            callback(result.error);
        }
        if downloads.dispatch_map.is_empty() {
            downloads.api_callback_handle = None;
            self.callback_timer.borrow_mut().callback_completed();
        }
    }
}

pub struct SteamTicket {
    user: User<ClientManager>,
    ticket: AuthTicket,
    data: Vec<u8>,
}

impl SteamTicket {
    fn new(user: User<ClientManager>) -> Self {
        let identity = NetworkingIdentity::new_steam_id(user.steam_id());
        let (ticket, data) = user.authentication_session_ticket(identity);
        Self { user, ticket, data }
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

impl Drop for SteamTicket {
    fn drop(&mut self) {
        self.user.cancel_authentication_ticket(self.ticket);
    }
}

struct CallbackTimer {
    logger: Logger,
    handle: Option<TimeoutHandle>,
    pending_callbacks: usize,
}

impl CallbackTimer {
    fn new(logger: &Logger) -> Self {
        Self {
            logger: logger.clone(),
            handle: None,
            pending_callbacks: 0,
        }
    }

    fn callback_pending(&mut self) {
        self.pending_callbacks += 1;
        trace!(
            self.logger,
            "Steam callback pending";
            "pending_callbacks" => self.pending_callbacks,
        );
        if self.handle.is_none() {
            let logger = self.logger.clone();
            trace!(logger, "Installing Steam callback runner timer");
            self.handle = Some(app::add_timeout3(0.5, move |handle| {
                trace!(logger, "Firing Steam callback runner timer");
                app::repeat_timeout3(0.5, handle);
                app::awake();
            }));
        }
    }

    fn callback_completed(&mut self) {
        self.pending_callbacks -= 1;
        trace!(
            self.logger,
            "Steam callback completed";
            "pending_callbacks" => self.pending_callbacks,
        );
        if self.pending_callbacks == 0 {
            trace!(self.logger, "Removing Steam callback runner timer");
            app::remove_timeout3(self.handle.take().unwrap());
        }
    }
}

// The callbacks in the Steamworks crate are required to be Send, because they are handed over to
// the Client, which can be used on any thread, but will be executed only on the thread that calls
// SimpleClient::run_callbacks. However, in this case, SteamClient is deliberately and explicitly
// made non-Send and non-Sync, to ensure that it can only be used from the thread where it was
// created, which means that callbacks will be called on the same thread on which they were created.
// Therefore, those callbacks need not be Send. The CallbackWrapper removes that requirement, by
// unsafely wrapping non-Send callbacks in a Send wrapper.
struct CallbackWrapper<T>(T);

impl<T> CallbackWrapper<T> {
    #[allow(dead_code)]
    fn call<A>(&self, arg: A)
    where
        T: Fn(A),
    {
        (self.0)(arg)
    }

    fn call_mut<A>(&mut self, arg: A)
    where
        T: FnMut(A),
    {
        (self.0)(arg)
    }

    fn call_once<A>(self, arg: A)
    where
        T: FnOnce(A),
    {
        (self.0)(arg)
    }
}

unsafe impl<T> Send for CallbackWrapper<T> {}

fn init_client(branch: Branch) -> Option<SteamAPI> {
    Client::init_app(app_id(branch))
        .ok()
        .map(|(client, cb_runner)| SteamAPI { client, cb_runner })
}
