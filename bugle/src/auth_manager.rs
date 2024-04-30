use std::cell::{Ref, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use dynabus::Bus;
use slog::{warn, Logger};

use crate::auth::{Account, AuthState, CachedUser, CachedUsers, Capability, PlatformUser};
use crate::bus::AppBus;
use crate::game::platform::steam::{PlatformReady, SteamClient};
use crate::game::Game;
use crate::gui::UpdateAuthState;
use crate::util::weak_cb;
use crate::workers::{FlsWorker, LoginComplete, TaskState};

pub struct AuthManager {
    logger: Logger,
    bus: Rc<RefCell<AppBus>>,
    game: Arc<Game>,
    steam: Rc<SteamClient>,
    cached_users: RefCell<CachedUsers>,
    cached_users_persister: CachedUsersPersister,
    fls_worker: Arc<FlsWorker>,
}

type CachedUsersPersister = fn(&Game, &CachedUsers) -> Result<()>;

impl AuthManager {
    pub fn new(
        logger: &Logger,
        bus: Rc<RefCell<AppBus>>,
        game: Arc<Game>,
        steam: Rc<SteamClient>,
    ) -> Rc<Self> {
        let logger = logger.clone();

        let (cached_users, cached_users_persister) = match game.load_cached_users() {
            Ok(cached_users) => (
                cached_users,
                Game::save_cached_users as CachedUsersPersister,
            ),
            Err(err) => {
                warn!(logger, "Error loading cached users"; "error" => %err);
                fn noop_persister(_: &Game, _: &CachedUsers) -> Result<()> {
                    Ok(())
                }
                (CachedUsers::new(), noop_persister as CachedUsersPersister)
            }
        };
        let cached_users = RefCell::new(cached_users);

        let fls_worker = FlsWorker::new(&logger, Arc::clone(&game), bus.borrow().sender().clone());

        let this = Rc::new(Self {
            logger,
            bus,
            game,
            steam,
            cached_users,
            cached_users_persister,
            fls_worker,
        });

        {
            let mut bus = this.bus.borrow_mut();
            bus.subscribe_observer(weak_cb!([this] => |&PlatformReady| this.check_auth_state()));
            bus.subscribe_consumer(weak_cb!(
                [this] => |LoginComplete(payload)| this.login_complete(payload)
            ));
        }

        this
    }

    pub fn cached_user(&self) -> Option<Ref<CachedUser>> {
        let platform_user = self.steam.user()?;
        let cached_users = self.cached_users.borrow();
        Ref::filter_map(cached_users, |cache| {
            cache.by_platform_id(&platform_user.id)
        })
        .ok()
    }

    pub fn check_auth_state(&self) {
        let platform_user = self.steam.user().ok_or(anyhow!("Steam not running"));
        let fls_account = match &platform_user {
            Ok(user) => {
                if let Some(cached) = self
                    .cached_users
                    .borrow()
                    .by_platform_id(&user.id)
                    .as_deref()
                {
                    TaskState::Ready(Ok(cached.account.clone()))
                } else {
                    if self.steam.can_play_online() {
                        TaskState::Pending
                    } else {
                        TaskState::Ready(Err(anyhow!("Steam in offline mode")))
                    }
                }
            }
            Err(err) => TaskState::Ready(Err(anyhow!(err.to_string()))),
        };
        let online_capability = self.online_capability(&platform_user, &fls_account);
        let sp_capability = self.sp_capability(&platform_user, &fls_account);

        if let TaskState::Pending = &fls_account {
            Arc::clone(&self.fls_worker).login_with_steam(&*self.steam.auth_ticket().unwrap());
        }

        let auth_state = AuthState {
            platform_user,
            fls_account,
            online_capability,
            sp_capability,
        };
        self.bus.borrow().publish(UpdateAuthState(auth_state));
    }

    fn login_complete(&self, payload: Result<Account>) {
        if let Ok(account) = &payload {
            if let Err(err) = self.cache_user(account) {
                warn!(self.logger, "Error saving cached users"; "error" => %err);
            }
        }

        let platform_user = self.steam.user().ok_or(anyhow!("Steam not running"));
        let fls_account = TaskState::Ready(payload);
        let online_capability = self.online_capability(&platform_user, &fls_account);
        let sp_capability = self.sp_capability(&platform_user, &fls_account);
        let auth_state = AuthState {
            platform_user,
            fls_account,
            online_capability,
            sp_capability,
        };
        self.bus.borrow().publish(UpdateAuthState(auth_state));
    }

    fn online_capability(
        &self,
        platform_user: &Result<PlatformUser>,
        fls_account: &TaskState<Result<Account>>,
    ) -> TaskState<Capability> {
        match &platform_user {
            Err(err) => TaskState::Ready(Err(anyhow!(err.to_string()))),
            Ok(_) => {
                if !self.steam.can_play_online() {
                    TaskState::Ready(Err(anyhow!("Steam in offline mode")))
                } else {
                    match &fls_account {
                        TaskState::Pending => TaskState::Pending,
                        TaskState::Ready(Ok(_)) => TaskState::Ready(Ok(())),
                        TaskState::Ready(Err(_)) => TaskState::Ready(Err(anyhow!("FLS error"))),
                    }
                }
            }
        }
    }

    fn sp_capability(
        &self,
        platform_user: &Result<PlatformUser>,
        fls_account: &TaskState<Result<Account>>,
    ) -> TaskState<Capability> {
        match &platform_user {
            Err(err) => TaskState::Ready(Err(anyhow!(err.to_string()))),
            Ok(_) => match &fls_account {
                TaskState::Pending => TaskState::Pending,
                TaskState::Ready(Ok(_)) => TaskState::Ready(Ok(())),
                TaskState::Ready(Err(_)) => {
                    TaskState::Ready(Err(anyhow!("FLS account not cached")))
                }
            },
        }
    }

    fn cache_user(&self, account: &Account) -> Result<()> {
        let mut cached_users = self.cached_users.borrow_mut();
        cached_users.insert(CachedUser::new(account.clone()));
        (self.cached_users_persister)(&self.game, &*cached_users)
    }
}
