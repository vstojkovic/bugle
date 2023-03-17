use std::fs::File;
use std::io::{BufRead, BufReader};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use ini::Properties;
use lazy_static::lazy_static;
use regex::Regex;
use slog::{info, warn, Logger};
use steamlocate::SteamDir;

mod engine;
mod mod_info;

use crate::config;
use crate::servers::{FavoriteServer, FavoriteServers};

pub use self::engine::db::{list_mod_controllers, GameDB};
use self::engine::map::MapExtractor;
pub use self::engine::map::{MapInfo, Maps};
pub use self::mod_info::{ModInfo, ModRef, Mods};

pub struct Game {
    logger: Logger,
    root: PathBuf,
    build_id: u32,
    save_path: PathBuf,
    game_ini_path: PathBuf,
    mod_list_path: PathBuf,
    installed_mods: Arc<Mods>,
    maps: Arc<Maps>,
}

impl Game {
    pub fn locate() -> Option<GameLocation> {
        let mut steam = SteamDir::locate()?;
        let app = steam.app(&440900)?;
        let game_path = app.path.clone();

        let workshop_path = steam
            .libraryfolders()
            .paths
            .iter()
            .find(|path| game_path.starts_with(path))?
            .join("workshop");

        Some(GameLocation {
            game_path,
            workshop_path,
        })
    }

    pub fn new(logger: Logger, location: GameLocation) -> Result<Self> {
        let save_path = location.game_path.join("ConanSandbox/Saved");
        let config_path = save_path.join("Config/WindowsNoEditor");

        let cooked_ini_path = location.game_path.join("ConanSandbox/CookedIniVersion.txt");

        let cooked_ini = config::load_ini(cooked_ini_path)?;
        let build_id = cooked_ini
            .section(Some("UsedSettings"))
            .and_then(|section| {
                section
                    .get_all("Windows.Engine")
                    .find_map(|val| BUILD_ID_REGEX.captures(val))
            })
            .map(|captures| captures.get(1).unwrap().as_str())
            .ok_or_else(|| anyhow::Error::msg("Missing build ID override"))
            .and_then(|s| Ok(s.parse::<u32>()?))?;

        let mod_list_path = location.game_path.join("ConanSandbox/Mods/modlist.txt");
        let mut installed_mods = location.collect_mods()?;
        installed_mods.sort_by(|lhs, rhs| lhs.name.cmp(&rhs.name));

        let mut maps = Maps::new();
        let map_extractor = MapExtractor::new();

        // TODO: Improved error handling
        map_extractor.extract_base_game_maps(
            &location
                .game_path
                .join("ConanSandbox/Content/Paks/Base.pak"),
            &mut maps,
        )?;
        for mod_info in installed_mods.iter() {
            map_extractor.extract_mod_maps(&*mod_info.pak_path, &mut maps)?;
        }

        info!(
            logger,
            "Valid Conan Exiles installation found";
            "path" => location.game_path.display(),
            "build_id" => build_id,
        );

        Ok(Self {
            logger,
            root: location.game_path,
            build_id,
            save_path,
            game_ini_path: config_path.join("Game.ini"),
            mod_list_path,
            installed_mods: Arc::new(Mods::new(installed_mods)),
            maps: Arc::new(maps),
        })
    }

    pub fn build_id(&self) -> u32 {
        self.build_id
    }

    pub fn revision(&self) -> (u32, u16) {
        let maj = (self.build_id | 0x80000000) >> 13;
        let min = (self.build_id & 0x1fff) as u16;
        let min = min | (0x8000 - (min & 0x1000));
        (maj, min)
    }

    pub fn installation_path(&self) -> &Path {
        &self.root
    }

    pub fn save_path(&self) -> &Path {
        &self.save_path
    }

    pub fn in_progress_game_path(&self, map_id: usize) -> PathBuf {
        self.save_path.join(&self.maps[map_id].db_name)
    }

    pub fn installed_mods(&self) -> &Arc<Mods> {
        &self.installed_mods
    }

    pub fn maps(&self) -> &Arc<Maps> {
        &self.maps
    }

    pub fn load_favorites(&self) -> Result<FavoriteServers> {
        let game_ini = config::load_ini(&self.game_ini_path)?;
        let mut favorites = FavoriteServers::new();

        if let Some(section) = game_ini.section(Some("FavoriteServers")) {
            for (key, value) in section.iter() {
                if key != "ServersList" {
                    continue;
                }
                if let Ok(favorite) = FavoriteServer::parse(value) {
                    favorites.insert(favorite);
                }
            }
        }

        Ok(favorites)
    }

    pub fn save_favorites(
        &self,
        favorites: impl IntoIterator<Item = FavoriteServer>,
    ) -> Result<()> {
        let mut game_ini = config::load_ini(&self.game_ini_path)?;
        let section = game_ini
            .entry(Some("FavoriteServers".to_string()))
            .or_insert_with(Properties::new);
        let _ = section.remove_all("ServersList");
        for favorite in favorites {
            section.append("ServersList", favorite.to_string());
        }
        info!(self.logger, "Saving favorites");
        config::save_ini(&game_ini, &self.game_ini_path)
    }

    pub fn load_mod_list(&self) -> Result<Vec<ModRef>> {
        if !self.mod_list_path.exists() {
            return Ok(Vec::new());
        }

        self.load_mod_list_from(&self.mod_list_path)
    }

    pub fn load_mod_list_from(&self, path: &Path) -> Result<Vec<ModRef>> {
        let file = File::open(path)?;
        let mut mod_list = Vec::new();
        for line in BufReader::new(file).lines() {
            // TODO: Logging?
            if let Ok(mod_path) = line {
                let mod_path: PathBuf = mod_path.into();
                mod_list.push(self.installed_mods.by_pak_path(&mod_path));
            }
        }

        Ok(mod_list)
    }

    pub fn save_mod_list<'m>(&self, mod_list: impl IntoIterator<Item = &'m ModRef>) -> Result<()> {
        self.save_mod_list_to(&self.mod_list_path, mod_list)
    }

    pub fn save_mod_list_to<'m>(
        &self,
        path: &Path,
        mod_list: impl IntoIterator<Item = &'m ModRef>,
    ) -> Result<()> {
        use std::io::Write;

        let mut file = File::create(path)?;
        for mod_ref in mod_list {
            let pak_path = match mod_ref {
                ModRef::Installed(_) => &self.installed_mods.get(mod_ref).unwrap().pak_path,
                ModRef::UnknownPakPath(path) => path,
                ModRef::UnknownFolder(_) => continue,
            };
            writeln!(&mut file, "{}", pak_path.display())?;
        }

        Ok(())
    }

    pub fn load_saved_games(&self) -> Result<Vec<GameDB>> {
        let mut saves = Vec::new();

        for entry in std::fs::read_dir(&self.save_path)? {
            let entry = if let Ok(entry) = entry {
                entry
            } else {
                continue;
            };

            let db_path = entry.path();
            if db_path.extension() != Some("db".as_ref()) {
                continue;
            }

            match GameDB::new(&db_path, |key| {
                self.maps.by_object_name(key).map(|map| map.id)
            }) {
                Ok(game_db) => saves.push(game_db),
                Err(err) => warn!(
                    self.logger,
                    "Error parsing the saved game {db_file}",
                    db_file = db_path.file_name().unwrap_or_default().to_string_lossy().as_ref();
                    "error" => err.to_string()
                ),
            }
        }

        Ok(saves)
    }

    pub fn launch(&self, enable_battleye: bool, args: &[&str]) -> Result<Child> {
        let mut exe_path = self.root.clone();
        exe_path.extend(["ConanSandbox", "Binaries", "Win64"]);
        exe_path.push(if enable_battleye { "ConanSandbox_BE.exe" } else { "ConanSandbox.exe" });

        let mut cmd = Command::new(exe_path);
        cmd.args(args);
        if enable_battleye {
            cmd.arg("-BattlEye");
        }

        info!(self.logger, "Launching Conan Exiles"; "command" => format!("{:?}", cmd));

        Ok(cmd.spawn()?)
    }

    pub fn continue_session(&self, enable_battleye: bool) -> Result<Child> {
        self.launch(enable_battleye, &["-continuesession"])
    }

    pub fn join_server(&self, addr: SocketAddr, enable_battleye: bool) -> Result<Child> {
        let mut game_ini = config::load_ini(&self.game_ini_path)?;
        game_ini
            .with_section(Some("SavedServers"))
            .set("LastConnected", addr.to_string());
        game_ini
            .with_section(Some("SavedCoopData"))
            .set("StartedListenServerSession", "False");
        config::save_ini(&game_ini, &self.game_ini_path)?;

        self.continue_session(enable_battleye)
    }

    pub fn launch_single_player(&self, map_id: usize, enable_battleye: bool) -> Result<Child> {
        let mut game_ini = config::load_ini(&self.game_ini_path)?;
        let map = &self.maps[map_id];
        game_ini
            .with_section(Some("SavedCoopData"))
            .set("LastMap", &map.asset_path)
            .set("StartedListenServerSession", "True")
            .set("WasCoopEnabled", "False");
        config::save_ini(&game_ini, &self.game_ini_path)?;

        self.continue_session(enable_battleye)
    }
}

pub struct GameLocation {
    pub game_path: PathBuf,
    workshop_path: PathBuf,
}

impl GameLocation {
    fn collect_mods(&self) -> Result<Vec<ModInfo>> {
        // TODO: Log warnings for recoverable errors

        let manifest = steamy_vdf::load(self.workshop_path.join("appworkshop_440900.acf"))?;
        let mod_ids = collect_mod_ids(&manifest).ok_or(anyhow!("Malformed workshop manifest"))?;

        let mut path = self.workshop_path.join("content/440900");
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
}

fn collect_mod_ids(manifest: &steamy_vdf::Entry) -> Option<Vec<&String>> {
    Some(
        manifest
            .lookup("AppWorkshop.WorkshopItemsInstalled")?
            .as_table()?
            .keys()
            .into_iter()
            .collect(),
    )
}

lazy_static! {
    static ref BUILD_ID_REGEX: Regex =
        Regex::new(r"^OnlineSubsystem:BuildIdOverride:0\s*=\s*(\d+)$").unwrap();
}
