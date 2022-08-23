use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};

use anyhow::Result;
use ini::Properties;
use steamlocate::SteamDir;

use crate::config::{self, save_ini};
use crate::servers::{FavoriteServer, FavoriteServers};

pub struct Game {
    root: PathBuf,
    build_id: u32,
    game_ini_path: PathBuf,
}

impl Game {
    pub fn locate() -> Option<PathBuf> {
        let mut steam = SteamDir::locate()?;
        let app = steam.app(&440900)?;

        Some(app.path.clone())
    }

    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut config_path = path.as_ref().to_path_buf();
        config_path.extend(["ConanSandbox", "Saved", "Config", "WindowsNoEditor"]);

        let engine_ini_path = config_path.join("Engine.ini");

        let engine_ini = config::load_ini(engine_ini_path)?;
        let build_id = engine_ini
            .get_from(Some("OnlineSubsystem"), "BuildIdOverride")
            .ok_or_else(|| anyhow::Error::msg("Missing build ID override"))
            .and_then(|s| Ok(s.parse::<u32>()?))?;

        Ok(Self {
            root: path.as_ref().into(),
            build_id,
            game_ini_path: config_path.join("Game.ini"),
        })
    }

    pub fn build_id(&self) -> u32 {
        self.build_id
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
        section.remove_all("ServersList");
        for favorite in favorites {
            section.append("ServersList", favorite.to_string());
        }
        save_ini(&game_ini, &self.game_ini_path)
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
}
