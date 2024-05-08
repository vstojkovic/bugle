use std::cell::{Ref, RefCell};
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::str::FromStr;

use anyhow::Result;
use ini::{EscapePolicy, Ini, LineSeparator, ParseOption, WriteOption};
use ini_persist::load::{IniLoad, LoadProperty, ParseProperty};
use ini_persist::save::{DisplayProperty, IniSave, SaveProperty};
use slog::{warn, Logger};

use crate::env::current_exe_dir;
use crate::game::Branch;
use crate::servers::{Filter, SortCriteria};

pub struct ConfigManager {
    logger: Logger,
    config: RefCell<Config>,
    persister: Box<dyn ConfigPersister>,
}

#[derive(Debug, Default, IniLoad, IniSave)]
pub struct Config {
    #[ini(general)]
    pub general: GeneralConfig,

    #[ini(section = "ServerBrowser")]
    pub server_browser: ServerBrowserConfig,
}

#[derive(Debug, Default, LoadProperty, SaveProperty)]
pub struct GeneralConfig {
    #[ini(rename = "LogLevel")]
    pub log_level: LogLevel,

    #[ini(rename = "Branch", ignore_errors)]
    pub branch: Branch,

    #[ini(rename = "UseBattlEye", ignore_errors)]
    pub use_battleye: BattlEyeUsage,

    #[ini(rename = "UseAllCores", ignore_errors)]
    pub use_all_cores: bool,

    #[ini(rename = "ExtraArgs", ignore_errors)]
    pub extra_args: String,

    #[ini(rename = "DisableModMismatchChecks", ignore_errors)]
    pub mod_mismatch_checks: ModMismatchChecks,

    #[ini(rename = "Theme", ignore_errors)]
    pub theme: ThemeChoice,
}

#[derive(Debug, Default, LoadProperty, SaveProperty)]
pub struct ServerBrowserConfig {
    #[ini(flatten)]
    pub filter: Filter,

    #[ini(rename = "SortBy")]
    pub sort_criteria: SortCriteria,

    #[ini(rename = "ScrollLock")]
    pub scroll_lock: bool,
}

impl Deref for Config {
    type Target = GeneralConfig;
    fn deref(&self) -> &Self::Target {
        &self.general
    }
}

impl DerefMut for Config {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.general
    }
}

pub trait ConfigPersister {
    fn load(&self) -> Result<Config>;
    fn save(&self, config: &Config) -> Result<()>;
}

impl ConfigManager {
    pub fn new(logger: &Logger, persister: Box<dyn ConfigPersister>) -> Rc<Self> {
        let logger = logger.clone();
        let config = RefCell::new(persister.load().unwrap_or_else(|err| {
            warn!(logger, "Error while loading the configuration"; "error" => err.to_string());
            Config::default()
        }));
        Rc::new(Self {
            logger,
            config,
            persister,
        })
    }

    pub fn get(&self) -> Ref<Config> {
        self.config.borrow()
    }

    pub fn update(&self, mutator: impl FnOnce(&mut Config)) {
        if let Err(err) = self.try_update(mutator) {
            warn!(self.logger, "Error while saving the configuration"; "error" => err.to_string());
        }
    }

    pub fn try_update(&self, mutator: impl FnOnce(&mut Config)) -> Result<()> {
        let mut config = self.config.borrow_mut();
        mutator(&mut config);
        self.persister.save(&config)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LogLevel(pub slog::FilterLevel);

impl Default for LogLevel {
    fn default() -> Self {
        Self(crate::logger::DEFAULT_LOG_LEVEL)
    }
}

impl ParseProperty for LogLevel {
    fn parse(text: &str) -> ini_persist::Result<Self> {
        Ok(match slog::FilterLevel::from_str(text) {
            Ok(level) => LogLevel(level),
            Err(_) => LogLevel::default(),
        })
    }
}

impl DisplayProperty for LogLevel {
    fn display(&self) -> String {
        self.0.as_str().to_ascii_lowercase()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattlEyeUsage {
    Auto,
    Always(bool),
}

impl Default for BattlEyeUsage {
    fn default() -> Self {
        Self::Always(true)
    }
}

impl ParseProperty for BattlEyeUsage {
    fn parse(text: &str) -> ini_persist::Result<Self> {
        Ok(match text.to_lowercase().as_str() {
            BATTLEYE_AUTO => Self::Auto,
            BATTLEYE_ALWAYS => Self::Always(true),
            BATTLEYE_NEVER => Self::Always(false),
            _ => Self::default(),
        })
    }
}

impl DisplayProperty for BattlEyeUsage {
    fn display(&self) -> String {
        match self {
            Self::Auto => BATTLEYE_AUTO.to_string(),
            Self::Always(true) => BATTLEYE_ALWAYS.to_string(),
            Self::Always(false) => BATTLEYE_NEVER.to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, LoadProperty, SaveProperty)]
#[ini(ignore_case)]
pub enum ModMismatchChecks {
    Enabled,
    Disabled,
}

impl Default for ModMismatchChecks {
    fn default() -> Self {
        Self::Enabled
    }
}

#[derive(Debug, Clone, Copy, LoadProperty, SaveProperty)]
#[ini(ignore_case)]
pub enum ThemeChoice {
    Light,
    Dark,
}

impl Default for ThemeChoice {
    fn default() -> Self {
        Self::Light
    }
}

pub struct TransientConfig;

impl ConfigPersister for TransientConfig {
    fn load(&self) -> Result<Config> {
        Ok(Config::default())
    }

    fn save(&self, _: &Config) -> Result<()> {
        Ok(())
    }
}

pub struct IniConfigPersister {
    config_path: PathBuf,
}

impl IniConfigPersister {
    #[cfg(not(windows))]
    pub fn new() -> Result<Self> {
        Self::for_current_exe()
    }

    #[cfg(windows)]
    pub fn new() -> Result<Self> {
        Self::for_current_exe().or_else(|_| Self::in_appdata())
    }

    fn for_current_exe() -> Result<Self> {
        Self::open(current_exe_dir()?.join("bugle.ini"))
    }

    #[cfg(windows)]
    fn in_appdata() -> Result<Self> {
        use crate::env::{appdata_dir, AppDataFolder};

        let mut path = appdata_dir(AppDataFolder::Roaming)?;
        path.push("bugle");
        std::fs::create_dir_all(&path)?;

        path.push("bugle.ini");
        Self::open(path)
    }

    fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let _ = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;
        Ok(Self {
            config_path: path.to_owned(),
        })
    }

    pub fn path(&self) -> &Path {
        &self.config_path
    }
}

impl ConfigPersister for IniConfigPersister {
    fn load(&self) -> Result<Config> {
        let ini = load_ini(&self.config_path)?;
        let mut config = Config::default();
        config.load_from_ini(&ini)?;
        Ok(config)
    }

    fn save(&self, config: &Config) -> Result<()> {
        let mut ini = Ini::new();
        config.save_to_ini(&mut ini);
        save_ini(&ini, &self.config_path)
    }
}

pub fn load_ini<P: AsRef<Path>>(path: P) -> Result<Ini> {
    load_ini_from_file(File::open(path.as_ref())?)
}

pub fn load_ini_from_file(file: File) -> Result<Ini> {
    let text = load_text_lossy(file)?;
    Ok(Ini::load_from_str_opt(
        &text,
        ParseOption {
            enabled_escape: false,
            enabled_quote: false,
        },
    )?)
}

pub fn save_ini<P: AsRef<Path>>(ini: &Ini, path: P) -> Result<()> {
    let file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(path.as_ref())?;
    save_ini_to_file(ini, file)
}

pub fn save_ini_to_file(ini: &Ini, mut file: File) -> Result<()> {
    Ok(ini.write_to_opt(
        &mut file,
        WriteOption {
            escape_policy: EscapePolicy::Nothing,
            line_separator: LineSeparator::SystemDefault,
        },
    )?)
}

fn load_text_lossy(mut file: File) -> std::io::Result<String> {
    let mut bytes = vec![];
    file.read_to_end(&mut bytes)?;

    // check for UTF-16LE BOM
    if bytes.len() >= 2 && bytes[0] == 0xff && bytes[1] == 0xfe {
        let (_, utf_16, _) = unsafe { bytes[2..].align_to::<u16>() };
        Ok(String::from_utf16_lossy(utf_16))
    } else {
        Ok(String::from_utf8_lossy(&bytes).to_string())
    }
}

const BATTLEYE_AUTO: &str = "auto";
const BATTLEYE_ALWAYS: &str = "always";
const BATTLEYE_NEVER: &str = "never";
