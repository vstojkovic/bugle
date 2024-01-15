use std::fs::File;
use std::io::{BufRead, BufReader};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;
use std::sync::{Arc, Mutex, MutexGuard};

use anyhow::Result;
use ini::Properties;
use lazy_static::lazy_static;
use regex::Regex;
use slog::{debug, info, warn, Logger};

mod engine;
mod launch;
mod mod_info;
pub mod platform;

use crate::auth::{CachedUser, CachedUsers};
use crate::config;
use crate::game::engine::version::get_game_version;
use crate::servers::{FavoriteServer, FavoriteServers, Server};

pub use self::engine::db::{create_empty_db, list_mod_controllers, GameDB};
use self::engine::map::MapExtractor;
pub use self::engine::map::{MapInfo, Maps};
pub use self::launch::Launch;
pub use self::mod_info::{ModInfo, ModProvenance, ModRef, Mods};

pub struct Game {
    logger: Logger,
    root: PathBuf,
    branch: Branch,
    needs_update: bool,
    version: (u32, u16),
    save_path: PathBuf,
    game_ini_path: PathBuf,
    mod_list_path: PathBuf,
    installed_mods: Arc<Mods>,
    maps: Arc<Maps>,
    last_session: Mutex<Option<Session>>,
}

#[derive(Debug, Clone, Copy)]
pub enum Branch {
    Main,
    PublicBeta,
}

impl Default for Branch {
    fn default() -> Self {
        Branch::Main
    }
}

#[derive(Debug)]
pub enum Session {
    SinglePlayer(MapRef),
    CoOp(MapRef),
    Online(ServerRef),
}

#[derive(Debug)]
pub enum MapRef {
    Known { map_id: usize },
    Unknown { asset_path: String },
}

#[derive(Debug)]
pub enum ServerRef {
    Known(Server),
    Unknown(SocketAddr),
}

#[derive(Debug)]
pub struct LaunchOptions {
    pub enable_battleye: bool,
    pub use_all_cores: bool,
    pub extra_args: String,
}

impl Game {
    fn new(
        logger: Logger,
        game_path: PathBuf,
        branch: Branch,
        needs_update: bool,
        mut installed_mods: Vec<ModInfo>,
    ) -> Result<Self> {
        let save_path = game_path.join("ConanSandbox/Saved");
        let config_path = save_path.join("Config/WindowsNoEditor");

        debug!(logger, "Querying game version");
        let version = get_game_version(&game_path)?;

        let mod_list_path = game_path.join("ConanSandbox/Mods/modlist.txt");
        installed_mods.sort_by(|lhs, rhs| lhs.name.cmp(&rhs.name));

        let mut maps = Maps::new();
        let map_extractor = MapExtractor::new(logger.clone());

        debug!(logger, "Enumerating base game maps");
        map_extractor.extract_base_game_maps(
            game_path.join("ConanSandbox/Content/Paks/Base.pak"),
            &mut maps,
        )?;

        debug!(logger, "Enumerating mod-provided maps");
        for mod_info in installed_mods.iter() {
            if let Err(err) = map_extractor.extract_mod_maps(&*mod_info.pak_path, &mut maps) {
                warn!(
                    logger,
                    "Failed to enumerate maps in mod";
                    "mod_path" => mod_info.pak_path.display(),
                    "error" => %err,
                );
            }
        }

        let game_ini_path = config_path.join("Game.ini");
        let game_ini =
            if game_ini_path.exists() { Some(config::load_ini(&game_ini_path)?) } else { None };

        debug!(logger, "Reading last session information");
        let last_session = if let Some(game_ini) = &game_ini {
            let coop_section = game_ini.section(Some(SECTION_SAVED_COOP_DATA));
            let is_local = coop_section
                .and_then(|section| section.get(KEY_STARTED_LISTEN_SERVER_SESSION))
                .map(|val| val.to_ascii_lowercase() == "true")
                .unwrap_or(true);
            let is_coop = coop_section
                .and_then(|section| section.get(KEY_WAS_COOP_ENABLED))
                .map(|val| val.to_ascii_lowercase() == "true")
                .unwrap_or(true);
            let local_map = coop_section.and_then(|section| section.get(KEY_LAST_MAP));

            let online_section = game_ini.section(Some(SECTION_SAVED_SERVERS));
            let server_addr = online_section
                .and_then(|section| section.get(KEY_LAST_CONNECTED))
                .and_then(|val| SocketAddr::from_str(val).ok());

            if is_local {
                local_map
                    .map(|asset_path| {
                        if let Some(map) = maps.by_asset_path(asset_path) {
                            MapRef::Known { map_id: map.id }
                        } else {
                            MapRef::Unknown {
                                asset_path: asset_path.to_string(),
                            }
                        }
                    })
                    .map(|map_ref| {
                        if is_coop {
                            Session::CoOp(map_ref)
                        } else {
                            Session::SinglePlayer(map_ref)
                        }
                    })
            } else {
                server_addr.map(|addr| Session::Online(ServerRef::Unknown(addr)))
            }
        } else {
            None
        };

        info!(
            logger,
            "Valid Conan Exiles installation found";
            "path" => game_path.display(),
            "version" => ?version,
        );

        Ok(Self {
            logger,
            root: game_path,
            branch,
            needs_update,
            version,
            save_path,
            game_ini_path,
            mod_list_path,
            installed_mods: Arc::new(Mods::new(installed_mods)),
            maps: Arc::new(maps),
            last_session: Mutex::new(last_session),
        })
    }

    pub fn branch(&self) -> Branch {
        self.branch
    }

    pub fn needs_update(&self) -> bool {
        self.needs_update
    }

    pub fn build_id(&self) -> u32 {
        let revision_bits = (self.version.0 & 0x3ffff) << 13;
        let snapshot_bits = (self.version.1 & 0x1fff) as u32;
        revision_bits + snapshot_bits
    }

    pub fn version(&self) -> (u32, u16) {
        self.version
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

    pub fn load_cached_users(&self) -> Result<CachedUsers> {
        debug!(self.logger, "Loading cached users");

        let game_ini = config::load_ini(&self.game_ini_path)?;
        let mut cached_users = CachedUsers::new();

        let section = match game_ini.section(Some(SECTION_FUNCOM_LIVE_SERVICES)) {
            Some(section) => section,
            None => return Ok(cached_users),
        };

        for value in section.get_all(KEY_CACHED_USERS) {
            match CachedUser::parse(value) {
                Ok(user) => cached_users.insert(user),
                Err(err) => warn!(
                    self.logger,
                    "Error parsing cached user";
                    "value" => value,
                    "error" => %err,
                ),
            }
        }

        Ok(cached_users)
    }

    pub fn save_cached_users(&self, cached_users: &CachedUsers) -> Result<()> {
        debug!(self.logger, "Saving cached users");

        let mut game_ini = config::load_ini(&self.game_ini_path)?;
        let section = game_ini
            .entry(Some(SECTION_FUNCOM_LIVE_SERVICES.to_string()))
            .or_insert_with(Properties::new);
        let _ = section.remove_all(KEY_CACHED_USERS);
        for user in cached_users.iter() {
            section.append(KEY_CACHED_USERS, user.to_string());
        }
        config::save_ini(&game_ini, &self.game_ini_path)
    }

    pub fn load_favorites(&self) -> Result<FavoriteServers> {
        debug!(self.logger, "Loading favorite servers");

        let game_ini = config::load_ini(&self.game_ini_path)?;
        let mut favorites = FavoriteServers::new();

        if let Some(section) = game_ini.section(Some(SECTION_FAVORITE_SERVERS)) {
            for value in section.get_all(KEY_SERVERS_LIST) {
                match FavoriteServer::parse(value) {
                    Ok(favorite) => {
                        favorites.insert(favorite);
                    }
                    Err(err) => warn!(
                        self.logger,
                        "Error parsing favorite";
                        "value" => value,
                        "error" => %err,
                    ),
                }
            }
        }

        Ok(favorites)
    }

    pub fn save_favorites(
        &self,
        favorites: impl IntoIterator<Item = FavoriteServer>,
    ) -> Result<()> {
        debug!(self.logger, "Saving favorite servers");

        let mut game_ini = config::load_ini(&self.game_ini_path)?;
        let section = game_ini
            .entry(Some(SECTION_FAVORITE_SERVERS.to_string()))
            .or_insert_with(Properties::new);
        let _ = section.remove_all(KEY_SERVERS_LIST);
        for favorite in favorites {
            section.append(KEY_SERVERS_LIST, favorite.to_string());
        }
        config::save_ini(&game_ini, &self.game_ini_path)
    }

    pub fn load_mod_list(&self) -> Result<Vec<ModRef>> {
        if !self.mod_list_path.exists() {
            debug!(self.logger, "No modlist file"; "path" => self.mod_list_path.display());
            return Ok(Vec::new());
        }

        self.load_mod_list_from(&self.mod_list_path)
    }

    pub fn load_mod_list_from(&self, path: &Path) -> Result<Vec<ModRef>> {
        debug!(self.logger, "Loading modlist"; "path" => path.display());

        let file = File::open(path)?;
        let mut mod_list = Vec::new();
        for line in BufReader::new(file).lines() {
            if let Ok(mod_path) = line {
                if !mod_path.starts_with('#') {
                    let mod_path: PathBuf = mod_path.trim().into();
                    mod_list.push(self.installed_mods.by_pak_path(&mod_path));
                }
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

        debug!(self.logger, "Saving modlist"; "path" => path.display());

        let mut file = File::create(path)?;
        for mod_ref in mod_list {
            let pak_path = match mod_ref {
                ModRef::Installed(_) => &self.installed_mods.get(mod_ref).unwrap().pak_path,
                ModRef::Custom(mod_info) => &mod_info.pak_path,
                ModRef::UnknownPakPath(path) => path,
                ModRef::UnknownFolder(_) => continue,
            };
            writeln!(&mut file, "{}", pak_path.display())?;
        }

        Ok(())
    }

    pub fn load_saved_games(&self) -> Result<Vec<GameDB>> {
        let mut saves = Vec::new();

        debug!(self.logger, "Enumerating saved games"; "path" => self.save_path.display());
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
                    "Error parsing the saved game";
                    "db_file" => db_path.file_name().unwrap_or_default().to_str(),
                    "error" => err.to_string(),
                ),
            }
        }

        Ok(saves)
    }

    pub fn last_session(&self) -> MutexGuard<Option<Session>> {
        self.last_session.lock().unwrap()
    }

    pub fn launch(&self, options: LaunchOptions, args: &[&str]) -> Result<Launch> {
        let mut exe_path = self.root.join("ConanSandbox/Binaries/Win64");
        exe_path.push(if options.enable_battleye {
            "ConanSandbox_BE.exe"
        } else {
            "ConanSandbox.exe"
        });

        let mut cmd = Command::new(exe_path);
        cmd.args(args);
        if options.enable_battleye {
            cmd.arg("-BattlEye");
        }
        if options.use_all_cores {
            cmd.arg("-USEALLAVAILABLECORES");
        }

        match shlex::split(&options.extra_args) {
            Some(args) => {
                cmd.args(args);
            }
            None => warn!(
                self.logger,
                "Error parsing extra launch args";
                "extra_args" => &options.extra_args
            ),
        };

        info!(self.logger, "Launching Conan Exiles"; "command" => format!("{:?}", cmd));
        Launch::new(&self.logger, cmd)
    }

    pub fn continue_session(&self, options: LaunchOptions) -> Result<Launch> {
        self.launch(options, &["-continuesession"])
    }

    pub fn join_server(
        &self,
        addr: SocketAddr,
        password: Option<String>,
        options: LaunchOptions,
    ) -> Result<Launch> {
        let mut game_ini = config::load_ini(&self.game_ini_path)?;
        game_ini
            .with_section(Some(SECTION_SAVED_SERVERS))
            .set(KEY_LAST_CONNECTED, addr.to_string())
            .set(KEY_LAST_PASSWORD, password.unwrap_or_default());
        game_ini
            .with_section(Some(SECTION_SAVED_COOP_DATA))
            .set(KEY_STARTED_LISTEN_SERVER_SESSION, "False");
        config::save_ini(&game_ini, &self.game_ini_path)?;

        self.continue_session(options)
    }

    pub fn launch_single_player(&self, map_id: usize, options: LaunchOptions) -> Result<Launch> {
        let mut game_ini = config::load_ini(&self.game_ini_path)?;
        let map = &self.maps[map_id];
        game_ini
            .with_section(Some(SECTION_SAVED_COOP_DATA))
            .set(KEY_LAST_MAP, &map.asset_path)
            .set(KEY_STARTED_LISTEN_SERVER_SESSION, "True")
            .set(KEY_WAS_COOP_ENABLED, "False");
        config::save_ini(&game_ini, &self.game_ini_path)?;

        self.continue_session(options)
    }
}

lazy_static! {
    static ref BUILD_ID_REGEX: Regex =
        Regex::new(r"^OnlineSubsystem:BuildIdOverride:0\s*=\s*(\d+)$").unwrap();
}

const SECTION_FAVORITE_SERVERS: &str = "FavoriteServers";
const SECTION_FUNCOM_LIVE_SERVICES: &str = "FuncomLiveServices";
const SECTION_SAVED_SERVERS: &str = "SavedServers";
const SECTION_SAVED_COOP_DATA: &str = "SavedCoopData";

const KEY_CACHED_USERS: &str = "CachedUsers";
const KEY_LAST_CONNECTED: &str = "LastConnected";
const KEY_LAST_PASSWORD: &str = "LastPassword";
const KEY_LAST_MAP: &str = "LastMap";
const KEY_SERVERS_LIST: &str = "ServersList";
const KEY_STARTED_LISTEN_SERVER_SESSION: &str = "StartedListenServerSession";
const KEY_WAS_COOP_ENABLED: &str = "WasCoopEnabled";
