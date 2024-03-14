use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use anyhow::{anyhow, bail, Result};
use fltk::app;
use slog::{debug, Logger};
use steamworks::SteamError;

use crate::game::platform::steam::client::DownloadCallback;
use crate::game::platform::{ModDirectory, ModUpdate};
use crate::game::{ModEntry, ModProvenance, Mods};
use crate::gui::ServerBrowserUpdate;
use crate::logger::IteratorFormatter;
use crate::workers::TaskState;
use crate::Message;

use super::SteamClient;

pub struct SteamModDirectory {
    logger: Logger,
    map: RefCell<HashMap<u64, String>>,
    client: Rc<SteamClient>,
    tx: app::Sender<Message>,
}

impl SteamModDirectory {
    pub fn new(
        logger: Logger,
        client: Rc<SteamClient>,
        tx: app::Sender<Message>,
        installed_mods: &Mods,
    ) -> Rc<Self> {
        let branch = client.branch();

        let mut map = HashMap::with_capacity(installed_mods.len());
        for entry in installed_mods.iter() {
            if let Ok(info) = entry.info.as_ref() {
                if let Some(id) = info.steam_file_id(branch) {
                    map.insert(id, info.name.clone());
                }
            }
        }

        Rc::new(Self {
            logger,
            map: RefCell::new(map),
            client,
            tx,
        })
    }
}

impl ModDirectory for SteamModDirectory {
    fn resolve(self: Rc<Self>, mods: &mut [(u64, Option<String>)]) {
        debug!(
            self.logger,
            "Resolving mod names";
            "mod_ids" => %IteratorFormatter(mods.iter().map(|(id, _)| id))
        );

        let map = self.map.borrow();
        let mut should_query = false;
        for (id, name) in mods.iter_mut() {
            let cached = map.get(id);
            if cached.is_none() {
                should_query = true;
            }
            *name = cached.map(String::clone);
        }

        if should_query {
            let mod_ids = mods
                .iter()
                .filter_map(|(id, name)| if name.is_none() { Some(*id) } else { None });
            let this = Rc::clone(&self);
            let tx = self.tx.clone();
            self.client.query_mods(mod_ids, move |results| {
                let mut map = this.map.borrow_mut();
                for (id, name) in results {
                    map.insert(id, name);
                }
                tx.send(Message::Update(ServerBrowserUpdate::RefreshDetails.into()));
            });
        }
    }

    fn needs_update(self: Rc<Self>, entry: &ModEntry) -> Result<bool> {
        if entry.provenance != ModProvenance::Steam {
            return Ok(false);
        }
        let mod_info = match entry.info.as_ref() {
            Ok(info) => info,
            Err(_) => return Ok(false),
        };
        let mod_id = mod_info
            .steam_file_id(self.client.branch())
            .ok_or_else(|| anyhow!("Mod does not have a Steam file ID"))?;
        self.client
            .mod_needs_update(mod_id)
            .ok_or_else(|| anyhow!("Steam not running"))
    }

    fn can_update(self: Rc<Self>) -> bool {
        self.client.can_play_online()
    }

    fn start_update(self: Rc<Self>, entry: &ModEntry) -> Result<Rc<dyn ModUpdate>> {
        let mod_id = entry
            .info
            .as_ref()
            .unwrap() // start_update should not be called if needs_update returned false
            .steam_file_id(self.client.branch())
            .ok_or(anyhow!("Mod does not have a Steam file ID"))?;
        let update = Rc::new(SteamModUpdate {
            client: Rc::clone(&self.client),
            mod_id,
            result: RefCell::new(TaskState::Pending),
        });
        let success = self.client.start_mod_update(mod_id, {
            let update = Rc::downgrade(&update);
            let callback: DownloadCallback = Rc::new(move |result| {
                if let Some(update) = update.upgrade() {
                    *update.result.borrow_mut() = TaskState::Ready(result);
                }
            });
            callback
        });
        if success.ok_or_else(|| anyhow!("Steam not running"))? {
            Ok(update)
        } else {
            bail!("Error starting the mod update download");
        }
    }
}

struct SteamModUpdate {
    client: Rc<SteamClient>,
    mod_id: u64,
    result: RefCell<TaskState<Option<SteamError>>>,
}

impl ModUpdate for SteamModUpdate {
    fn progress(&self) -> Option<(u64, u64)> {
        self.client.download_progress(self.mod_id)
    }

    fn state(&self) -> TaskState<Result<()>> {
        match &*self.result.borrow() {
            TaskState::Pending => TaskState::Pending,
            TaskState::Ready(Some(err)) => TaskState::Ready(Err(err.clone().into())),
            TaskState::Ready(None) => TaskState::Ready(Ok(())),
        }
    }
}
