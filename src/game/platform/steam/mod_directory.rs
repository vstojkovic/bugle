use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use anyhow::{anyhow, Result};
use fltk::app;
use slog::{debug, Logger};

use crate::game::platform::ModDirectory;
use crate::game::{ModInfo, Mods};
use crate::gui::ServerBrowserUpdate;
use crate::logger::IteratorFormatter;
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
        for mod_info in installed_mods.iter() {
            if let Some(id) = mod_info.steam_file_id(branch) {
                map.insert(id, mod_info.name.clone());
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

    fn needs_update(self: Rc<Self>, mod_ref: &ModInfo) -> Result<bool> {
        let mod_id = mod_ref
            .steam_file_id(self.client.branch())
            .ok_or(anyhow!("Mod does not have a Steam file ID"))?;
        self.client
            .mod_needs_update(mod_id)
            .ok_or(anyhow!("Steam not running"))
    }
}
