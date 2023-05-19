use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use anyhow::{anyhow, Result};
use keyvalues_parser::Vdf;
use slog::{debug, o, Logger};
use steamlocate::SteamDir;
use steamworks::{AuthTicket, Client, ClientManager, User};

use crate::auth::PlatformUser;
use crate::game::{Game, ModInfo};

pub struct Steam {
    logger: Logger,
    installation: SteamDir,
    client: Option<Client>,
    ticket: RefCell<Option<Rc<SteamTicket>>>,
}

impl Steam {
    pub fn locate(logger: &Logger) -> Option<Self> {
        debug!(logger, "Locating Steam installation");
        let installation = SteamDir::locate()?;

        Some(Self {
            logger: logger.new(o!("platform" => "steam")),
            installation,
            client: init_client(),
            ticket: RefCell::new(None),
        })
    }

    pub fn locate_game(&mut self) -> Result<Game> {
        debug!(self.logger, "Locating game installation");
        let game_path = self
            .installation
            .app(&440900)
            .ok_or_else(|| {
                anyhow!(
                    "Cannot locate Conan Exiles installation. Please verify that you have Conan \
                    Exiles installed in a Steam library and try again."
                )
            })?
            .path
            .clone();

        debug!(self.logger, "Determining the workshop path");
        let workshop_path = self
            .installation
            .libraryfolders()
            .paths
            .iter()
            .find(|path| game_path.starts_with(path))
            .map(|path| path.join("workshop"));

        debug!(self.logger, "Enumerating installed mods"; "workshop_path" => ?workshop_path);
        let installed_mods = if let Some(workshop_path) = workshop_path {
            collect_mods(&workshop_path)?
        } else {
            Vec::new()
        };

        Game::new(self.logger.clone(), game_path, installed_mods)
    }

    pub fn check_client(&mut self) {
        if self.client.is_none() {
            self.client = init_client();
        }
    }

    pub fn can_launch(&mut self) -> bool {
        self.client.is_some()
    }

    pub fn can_play_online(&self) -> bool {
        match &self.client {
            Some(client) => client.user().logged_on(),
            None => false,
        }
    }

    pub fn user(&self) -> Option<PlatformUser> {
        self.client.as_ref().map(|client| PlatformUser {
            id: client.user().steam_id().raw().to_string(),
            display_name: client.friends().name(),
        })
    }

    pub fn auth_ticket(&self) -> Option<Rc<SteamTicket>> {
        let mut ticket = self.ticket.borrow_mut();
        if ticket.is_none() {
            *ticket = self.client.as_ref().and_then(|client| {
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

fn init_client() -> Option<Client> {
    Client::init_app(440900).ok().map(|(client, _)| client)
}

fn collect_mods(workshop_path: &Path) -> Result<Vec<ModInfo>> {
    // TODO: Log warnings for recoverable errors

    let manifest_path = workshop_path.join("appworkshop_440900.acf");
    if !manifest_path.exists() {
        return Ok(Vec::new());
    }

    let manifest = std::fs::read_to_string(manifest_path)?;
    let manifest = Vdf::parse(&manifest)?;
    let mod_ids = collect_mod_ids(&manifest).ok_or(anyhow!("Malformed workshop manifest"))?;

    let mut path = workshop_path.join("content/440900");
    let mut mods = Vec::with_capacity(mod_ids.len());
    for mod_id in mod_ids {
        path.push(mod_id);
        for pak_path in std::fs::read_dir(&path)? {
            let pak_path = pak_path?.path();
            match pak_path.extension() {
                Some(ext) if ext == "pak" => {
                    mods.push(ModInfo::new(pak_path)?);
                }
                _ => (),
            };
        }
        path.pop();
    }

    Ok(mods)
}

fn collect_mod_ids<'m>(manifest: &'m Vdf) -> Option<Vec<&'m str>> {
    Some(
        manifest
            .value
            .get_obj()?
            .get("WorkshopItemsInstalled")?
            .into_iter()
            .next()?
            .get_obj()?
            .keys()
            .into_iter()
            .map(|key| key.as_ref())
            .collect(),
    )
}
