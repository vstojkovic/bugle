use std::cell::{Ref, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use anyhow::Result;
use ini::{EscapePolicy, Ini, LineSeparator, ParseOption, WriteOption};
use slog::{warn, Logger};

use crate::env::current_exe_dir;
use crate::game::Branch;
use crate::servers::{Filter, Mode, Region, SortCriteria, SortKey, TypeFilter};

pub struct ConfigManager {
    logger: Logger,
    config: RefCell<Config>,
    persister: Box<dyn ConfigPersister>,
}

#[derive(Debug, Default)]
pub struct Config {
    pub log_level: LogLevel,
    pub branch: Branch,
    pub use_battleye: BattlEyeUsage,
    pub use_all_cores: bool,
    pub extra_args: String,
    pub mod_mismatch_checks: ModMismatchChecks,
    pub theme: ThemeChoice,
    pub server_browser: ServerBrowserConfig,
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

#[derive(Debug, Clone, Copy)]
pub enum ModMismatchChecks {
    Enabled,
    Disabled,
}

impl Default for ModMismatchChecks {
    fn default() -> Self {
        Self::Enabled
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ThemeChoice {
    Light,
    Dark,
}

impl Default for ThemeChoice {
    fn default() -> Self {
        Self::Light
    }
}

#[derive(Debug, Default)]
pub struct ServerBrowserConfig {
    pub filter: Filter,
    pub sort_criteria: SortCriteria,
    pub scroll_lock: bool,
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
        use std::str::FromStr;

        let ini = load_ini(&self.config_path)?;
        let section = ini.section(None::<String>);
        let log_level = section
            .and_then(|section| section.get(KEY_LOG_LEVEL))
            .and_then(|value| slog::FilterLevel::from_str(value.trim()).ok())
            .map(|level| LogLevel(level))
            .unwrap_or_default();
        let branch = section
            .and_then(|section| section.get(KEY_BRANCH))
            .and_then(|value| match value.trim().to_ascii_lowercase().as_str() {
                BRANCH_MAIN => Some(Branch::Main),
                BRANCH_PUBLIC_BETA => Some(Branch::PublicBeta),
                _ => None,
            })
            .unwrap_or_default();
        let use_battleye = section
            .and_then(|section| section.get(KEY_USE_BATTLEYE))
            .and_then(|value| match value.trim().to_ascii_lowercase().as_str() {
                BATTLEYE_AUTO => Some(BattlEyeUsage::Auto),
                BATTLEYE_ALWAYS => Some(BattlEyeUsage::Always(true)),
                BATTLEYE_NEVER => Some(BattlEyeUsage::Always(false)),
                _ => None,
            })
            .unwrap_or_default();
        let use_all_cores = section
            .and_then(|section| section.get(KEY_USE_ALL_CORES))
            .and_then(|s| bool::from_str(&s.to_ascii_lowercase()).ok())
            .unwrap_or_default();
        let extra_args = section
            .and_then(|section| section.get(KEY_EXTRA_ARGS))
            .unwrap_or_default()
            .to_string();
        let mod_mismatch_checks = section
            .and_then(|section| section.get(KEY_DISABLE_MOD_MISMATCH_CHECKS))
            .and_then(|value| match value.trim().to_ascii_lowercase().as_str() {
                MOD_MISMATCH_CHECKS_ENABLED => Some(ModMismatchChecks::Enabled),
                MOD_MISMATCH_CHECKS_DISABLED => Some(ModMismatchChecks::Disabled),
                _ => None,
            })
            .unwrap_or_default();
        let theme = section
            .and_then(|section| section.get(KEY_THEME))
            .and_then(|value| match value.trim().to_ascii_lowercase().as_str() {
                THEME_LIGHT => Some(ThemeChoice::Light),
                THEME_DARK => Some(ThemeChoice::Dark),
                _ => None,
            })
            .unwrap_or_default();

        Ok(Config {
            log_level,
            branch,
            use_battleye,
            use_all_cores,
            extra_args,
            mod_mismatch_checks,
            theme,
            server_browser: load_server_browser_config(&ini),
        })
    }

    fn save(&self, config: &Config) -> Result<()> {
        let mut ini = Ini::new();
        let setter = &mut ini.with_general_section();
        let setter = setter
            .set(
                KEY_LOG_LEVEL,
                config.log_level.0.as_str().to_ascii_lowercase(),
            )
            .set(
                KEY_BRANCH,
                match config.branch {
                    Branch::Main => BRANCH_MAIN,
                    Branch::PublicBeta => BRANCH_PUBLIC_BETA,
                },
            )
            .set(
                KEY_USE_BATTLEYE,
                match config.use_battleye {
                    BattlEyeUsage::Auto => BATTLEYE_AUTO,
                    BattlEyeUsage::Always(true) => BATTLEYE_ALWAYS,
                    BattlEyeUsage::Always(false) => BATTLEYE_NEVER,
                },
            )
            .set(KEY_USE_ALL_CORES, config.use_all_cores.to_string());
        let setter = if config.extra_args.is_empty() {
            setter
        } else {
            setter.set(KEY_EXTRA_ARGS, &config.extra_args)
        };
        setter
            .set(
                KEY_DISABLE_MOD_MISMATCH_CHECKS,
                match config.mod_mismatch_checks {
                    ModMismatchChecks::Enabled => MOD_MISMATCH_CHECKS_ENABLED,
                    ModMismatchChecks::Disabled => MOD_MISMATCH_CHECKS_DISABLED,
                },
            )
            .set(
                KEY_THEME,
                match config.theme {
                    ThemeChoice::Light => THEME_LIGHT,
                    ThemeChoice::Dark => THEME_DARK,
                },
            );
        save_server_browser_config(&mut ini, &config.server_browser);
        save_ini(&ini, &self.config_path)
    }
}

pub fn load_ini<P: AsRef<Path>>(path: P) -> Result<Ini> {
    let text = load_text_lossy(path)?;
    Ok(Ini::load_from_str_opt(
        &text,
        ParseOption {
            enabled_escape: false,
            enabled_quote: false,
        },
    )?)
}

pub fn save_ini<P: AsRef<Path>>(ini: &Ini, path: P) -> Result<()> {
    Ok(ini.write_to_file_opt(
        path,
        WriteOption {
            escape_policy: EscapePolicy::Nothing,
            line_separator: LineSeparator::SystemDefault,
        },
    )?)
}

fn load_text_lossy<P: AsRef<Path>>(path: P) -> std::io::Result<String> {
    let bytes = std::fs::read(path.as_ref())?;

    // check for UTF-16LE BOM
    if bytes.len() >= 2 && bytes[0] == 0xff && bytes[1] == 0xfe {
        let (_, utf_16, _) = unsafe { bytes[2..].align_to::<u16>() };
        Ok(String::from_utf16_lossy(utf_16))
    } else {
        Ok(String::from_utf8_lossy(&bytes).to_string())
    }
}

fn load_server_browser_config(ini: &Ini) -> ServerBrowserConfig {
    use std::str::FromStr;

    let section = ini.section(Some(SECTION_SERVER_BROWSER));
    let name = section
        .and_then(|section| section.get(KEY_NAME))
        .unwrap_or_default()
        .to_string();
    let map = section
        .and_then(|section| section.get(KEY_MAP))
        .unwrap_or_default()
        .to_string();
    let type_filter = section
        .and_then(|section| section.get(KEY_TYPE_FILTER))
        .and_then(|s| TypeFilter::from_str(s).ok())
        .unwrap_or_default();
    let mode = section
        .and_then(|section| section.get(KEY_MODE))
        .and_then(|s| Mode::from_str(s).ok());
    let region = section
        .and_then(|section| section.get(KEY_REGION))
        .and_then(|s| Region::from_str(s).ok());
    let battleye_required = section
        .and_then(|section| section.get(KEY_BATTLEYE_REQUIRED))
        .and_then(|s| bool::from_str(&s.to_ascii_lowercase()).ok());
    let include_invalid = section
        .and_then(|section| section.get(KEY_INCLUDE_INVALID))
        .and_then(|s| bool::from_str(&s.to_ascii_lowercase()).ok())
        .unwrap_or_default();
    let include_password_protected = section
        .and_then(|section| section.get(KEY_INCLUDE_PASSWORD_PROTECTED))
        .and_then(|s| bool::from_str(&s.to_ascii_lowercase()).ok())
        .unwrap_or(true);
    let mods = section
        .and_then(|section| section.get(KEY_MODS))
        .and_then(|s| bool::from_str(&s.to_ascii_lowercase()).ok());
    let sort_criteria = section
        .and_then(|section| section.get(KEY_SORT_CRITERIA))
        .map(|s| if s.starts_with('-') { (false, &s[1..]) } else { (true, s) })
        .and_then(|(ascending, s)| {
            SortKey::from_str(s)
                .ok()
                .map(|key| SortCriteria { key, ascending })
        })
        .unwrap_or_default();
    let scroll_lock = section
        .and_then(|section| section.get(KEY_SCROLL_LOCK))
        .and_then(|s| bool::from_str(&s.to_ascii_lowercase()).ok())
        .unwrap_or(true);
    ServerBrowserConfig {
        filter: Filter {
            name,
            map,
            type_filter,
            mode,
            region,
            battleye_required,
            include_invalid,
            exclude_password_protected: !include_password_protected,
            mods,
        },
        sort_criteria,
        scroll_lock,
    }
}

fn save_server_browser_config(ini: &mut Ini, config: &ServerBrowserConfig) {
    let setter = &mut ini.with_section(Some(SECTION_SERVER_BROWSER));
    let setter = if config.filter.name.is_empty() {
        setter
    } else {
        setter.set(KEY_NAME, &config.filter.name)
    };
    let setter =
        if config.filter.map.is_empty() { setter } else { setter.set(KEY_MAP, &config.filter.map) };
    let setter = setter.set(KEY_TYPE_FILTER, config.filter.type_filter.as_ref());
    let setter = match config.filter.mode {
        Some(mode) => setter.set(KEY_MODE, mode.as_ref()),
        None => setter,
    };
    let setter = match config.filter.region {
        Some(region) => setter.set(KEY_REGION, region.as_ref()),
        None => setter,
    };
    let setter = match config.filter.battleye_required {
        Some(required) => setter.set(KEY_BATTLEYE_REQUIRED, required.to_string()),
        None => setter,
    };
    let setter = match config.filter.mods {
        Some(mods) => setter.set(KEY_MODS, mods.to_string()),
        None => setter,
    };
    setter
        .set(
            KEY_INCLUDE_INVALID,
            config.filter.include_invalid.to_string(),
        )
        .set(
            KEY_INCLUDE_PASSWORD_PROTECTED,
            (!config.filter.exclude_password_protected).to_string(),
        )
        .set(
            KEY_SORT_CRITERIA,
            sort_criteria_to_string(&config.sort_criteria),
        )
        .set(KEY_SCROLL_LOCK, config.scroll_lock.to_string());
}

fn sort_criteria_to_string(criteria: &SortCriteria) -> String {
    let prefix = if criteria.ascending { "" } else { "-" };
    format!("{}{}", prefix, criteria.key.as_ref())
}

const SECTION_SERVER_BROWSER: &str = "ServerBrowser";

const KEY_LOG_LEVEL: &str = "LogLevel";
const KEY_BRANCH: &str = "Branch";
const KEY_USE_BATTLEYE: &str = "UseBattlEye";
const KEY_USE_ALL_CORES: &str = "UseAllCores";
const KEY_EXTRA_ARGS: &str = "ExtraArgs";
const KEY_DISABLE_MOD_MISMATCH_CHECKS: &str = "DisableModMismatchChecks";
const KEY_THEME: &str = "Theme";
const KEY_NAME: &str = "Name";
const KEY_MAP: &str = "Map";
const KEY_TYPE_FILTER: &str = "Type";
const KEY_MODE: &str = "Mode";
const KEY_REGION: &str = "Region";
const KEY_BATTLEYE_REQUIRED: &str = "BattlEyeRequired";
const KEY_INCLUDE_INVALID: &str = "IncludeInvalid";
const KEY_INCLUDE_PASSWORD_PROTECTED: &str = "IncludePasswordProtected";
const KEY_MODS: &str = "Mods";
const KEY_SORT_CRITERIA: &str = "SortBy";
const KEY_SCROLL_LOCK: &str = "ScrollLock";

const BRANCH_MAIN: &str = "live";
const BRANCH_PUBLIC_BETA: &str = "testlive";

const BATTLEYE_AUTO: &str = "auto";
const BATTLEYE_ALWAYS: &str = "always";
const BATTLEYE_NEVER: &str = "never";

const MOD_MISMATCH_CHECKS_ENABLED: &str = "enabled";
const MOD_MISMATCH_CHECKS_DISABLED: &str = "disabled";

const THEME_LIGHT: &str = "light";
const THEME_DARK: &str = "dark";
