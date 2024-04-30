use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use anyhow::{anyhow, Result};
use keyvalues_parser::Vdf;
use slog::{debug, o, Logger};
use steamlocate::SteamDir;

mod client;
mod mod_directory;

pub use self::client::{PlatformReady, SteamClient, SteamTicket};
pub use self::mod_directory::SteamModDirectory;
use crate::bus::AppBus;
use crate::game::{Branch, Game, ModLibraryBuilder, ModProvenance};
use crate::util::PathExt;

pub struct Steam {
    logger: Logger,
    installation: SteamDir,
}

pub struct SteamGameLocation {
    game_path: PathBuf,
    workshop_path: Option<PathBuf>,
    branch: Branch,
    needs_update: bool,
}

impl Steam {
    pub fn locate(logger: &Logger) -> Option<Self> {
        debug!(logger, "Locating Steam installation");
        let installation = SteamDir::locate()?;

        Some(Self {
            logger: logger.new(o!("platform" => "steam")),
            installation,
        })
    }

    pub fn locate_game(&mut self, branch: Branch) -> Result<SteamGameLocation> {
        debug!(self.logger, "Locating game installation");
        let app = self.installation.app(&app_id(branch)).ok_or_else(|| {
            anyhow!(
                "Cannot locate Conan Exiles installation. Please verify that you have Conan \
                    Exiles installed in a Steam library and try again."
            )
        })?;
        let game_path = app.path.clone();
        let needs_update = match &app.state_flags {
            None => false,
            Some(flags) => flags.into_iter().any(|flag| match flag {
                steamlocate::steamapp::StateFlag::UpdateRequired => true,
                _ => false,
            }),
        };

        debug!(self.logger, "Determining the workshop path");
        let workshop_path = self
            .installation
            .libraryfolders()
            .paths
            .iter()
            .find(|path| game_path.starts_with(path))
            .map(|path| path.join("workshop"));

        Ok(SteamGameLocation {
            game_path,
            workshop_path,
            branch,
            needs_update,
        })
    }

    pub fn init_game(&mut self, location: SteamGameLocation) -> Result<Game> {
        debug!(
            self.logger,
            "Enumerating installed mods";
            "workshop_path" => ?location.workshop_path
        );
        let installed_mods = if let Some(workshop_path) = location.workshop_path {
            collect_mods(&workshop_path, location.branch)?
        } else {
            ModLibraryBuilder::new()
        };

        let game = Game::new(
            &self.logger,
            location.game_path,
            location.branch,
            location.needs_update,
            installed_mods,
        )?;

        Ok(game)
    }

    pub fn init_client(&self, game: &Game, bus: Rc<RefCell<AppBus>>) -> Rc<SteamClient> {
        SteamClient::new(&self.logger, game.branch(), bus)
    }
}

fn app_id(branch: Branch) -> u32 {
    match branch {
        Branch::Main => 440900,
        Branch::PublicBeta => 931180,
    }
}

fn collect_mods(workshop_path: &Path, branch: Branch) -> Result<ModLibraryBuilder> {
    let mut mods = ModLibraryBuilder::new();
    let manifest_path = workshop_path.join(format!("appworkshop_{}.acf", app_id(branch)));
    if !manifest_path.exists() {
        return Ok(mods);
    }

    let manifest = std::fs::read_to_string(manifest_path)?;
    let manifest = Vdf::parse(&manifest)?;
    let mod_ids = collect_mod_ids(&manifest).ok_or(anyhow!("Malformed workshop manifest"))?;

    let mut path = workshop_path.join_all(["content", &format!("{}", app_id(branch))]);
    for mod_id in mod_ids {
        path.push(mod_id);
        for pak_path in std::fs::read_dir(&path)? {
            let pak_path = pak_path?.path();
            match pak_path.extension() {
                Some(ext) if ext == "pak" => {
                    mods.add(pak_path, ModProvenance::Steam)?;
                }
                _ => (),
            };
        }
        path.pop();
    }
    mods.map_root(ModProvenance::Steam, path);

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
