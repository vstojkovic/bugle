use std::cell::{RefCell, RefMut};
use std::marker::PhantomData;
use std::rc::Rc;

use fltk::app::{self, TimeoutHandle};
use slog::{debug, o, trace, warn, Logger};
use steamworks::{AuthTicket, Client, ClientManager, PublishedFileId, SingleClient, User};

use crate::auth::PlatformUser;
use crate::game::Branch;
use crate::logger::IteratorFormatter;

use super::app_id;

pub struct SteamClient {
    logger: Logger,
    branch: Branch,
    api: RefCell<Option<SteamAPI>>,
    ticket: RefCell<Option<Rc<SteamTicket>>>,
    callback_timer: Rc<RefCell<CallbackTimer>>,
    // explicitly make sure SteamClient is neither Send nor Sync, see CallbackWrapper below
    _marker: PhantomData<*const ()>,
}

struct SteamAPI {
    client: Client,
    cb_runner: SingleClient,
}

impl SteamClient {
    pub(super) fn new(logger: Logger, branch: Branch) -> Rc<Self> {
        let logger = logger.new(o!("branch" => format!("{:?}", branch)));
        let callback_timer = Rc::new(RefCell::new(CallbackTimer::new(logger.clone())));
        Rc::new(Self {
            logger,
            branch,
            api: RefCell::new(init_client(branch)),
            ticket: RefCell::new(None),
            callback_timer,
            _marker: PhantomData,
        })
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

    pub fn run_callbacks(&self) {
        if let Some(api) = &*self.api.borrow() {
            api.cb_runner.run_callbacks();
        }
    }

    fn check_client(&self) -> Option<RefMut<Client>> {
        let mut api = self.api.borrow_mut();
        if api.is_none() {
            *api = init_client(self.branch);
        }
        RefMut::filter_map(api, |opt| opt.as_mut().map(|api| &mut api.client)).ok()
    }
}

pub struct SteamTicket {
    user: User<ClientManager>,
    ticket: AuthTicket,
    data: Vec<u8>,
}

impl SteamTicket {
    fn new(user: User<ClientManager>) -> Self {
        let (ticket, data) = user.authentication_session_ticket();
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
    fn new(logger: Logger) -> Self {
        Self {
            logger,
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

    #[allow(dead_code)]
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
