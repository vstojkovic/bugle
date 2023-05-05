use std::path::{Path, PathBuf};

use anyhow::Result;
use ini::{EscapePolicy, Ini, LineSeparator, ParseOption, WriteOption};

use crate::env::current_exe_dir;
use crate::servers::{Mode, Region, SortCriteria, SortKey, TypeFilter};

#[derive(Debug, Default)]
pub struct Config {
    pub log_level: LogLevel,
    pub use_battleye: BattlEyeUsage,
    pub server_browser: ServerBrowserConfig,
}

#[derive(Debug, Clone, Copy)]
pub struct LogLevel(pub slog::FilterLevel);

impl Default for LogLevel {
    fn default() -> Self {
        Self(slog::FilterLevel::Info)
    }
}

#[derive(Debug)]
pub enum BattlEyeUsage {
    Auto,
    Always(bool),
}

impl Default for BattlEyeUsage {
    fn default() -> Self {
        Self::Always(true)
    }
}

#[derive(Debug, Default)]
pub struct ServerBrowserConfig {
    pub type_filter: TypeFilter,
    pub mode: Option<Mode>,
    pub region: Option<Region>,
    pub battleye_required: Option<bool>,
    pub include_invalid: bool,
    pub include_password_protected: bool,
    pub mods: Option<bool>,
    pub sort_criteria: SortCriteria,
    pub scroll_lock: bool,
}

pub trait ConfigPersister {
    fn load(&self) -> Result<Config>;
    fn save(&self, config: &Config) -> Result<()>;
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
    pub fn for_current_exe() -> Result<Self> {
        Self::new(current_exe_dir()?.join("bugle.ini"))
    }

    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
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
        let use_battleye = section
            .and_then(|section| section.get(KEY_USE_BATTLEYE))
            .and_then(|value| match value.trim().to_ascii_lowercase().as_str() {
                BATTLEYE_AUTO => Some(BattlEyeUsage::Auto),
                BATTLEYE_ALWAYS => Some(BattlEyeUsage::Always(true)),
                BATTLEYE_NEVER => Some(BattlEyeUsage::Always(false)),
                _ => None,
            })
            .unwrap_or_default();

        Ok(Config {
            log_level,
            use_battleye,
            server_browser: load_server_browser_config(&ini),
        })
    }

    fn save(&self, config: &Config) -> Result<()> {
        let mut ini = Ini::new();
        ini.with_general_section()
            .set(
                KEY_USE_BATTLEYE,
                match config.use_battleye {
                    BattlEyeUsage::Auto => BATTLEYE_AUTO,
                    BattlEyeUsage::Always(true) => BATTLEYE_ALWAYS,
                    BattlEyeUsage::Always(false) => BATTLEYE_NEVER,
                },
            )
            .set(
                KEY_LOG_LEVEL,
                config.log_level.0.as_str().to_ascii_lowercase(),
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
        .unwrap_or_default();
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
        type_filter,
        mode,
        region,
        battleye_required,
        include_invalid,
        include_password_protected,
        mods,
        sort_criteria,
        scroll_lock,
    }
}

fn save_server_browser_config(ini: &mut Ini, config: &ServerBrowserConfig) {
    let mut setter = ini.with_section(Some(SECTION_SERVER_BROWSER));
    let setter = setter.set(KEY_TYPE_FILTER, config.type_filter.as_ref());
    let setter = match config.mode {
        Some(mode) => setter.set(KEY_MODE, mode.as_ref()),
        None => setter,
    };
    let setter = match config.region {
        Some(region) => setter.set(KEY_REGION, region.as_ref()),
        None => setter,
    };
    let setter = match config.battleye_required {
        Some(required) => setter.set(KEY_BATTLEYE_REQUIRED, required.to_string()),
        None => setter,
    };
    let setter = match config.mods {
        Some(mods) => setter.set(KEY_MODS, mods.to_string()),
        None => setter,
    };
    setter
        .set(KEY_INCLUDE_INVALID, config.include_invalid.to_string())
        .set(
            KEY_INCLUDE_PASSWORD_PROTECTED,
            config.include_password_protected.to_string(),
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
const KEY_USE_BATTLEYE: &str = "UseBattlEye";
const KEY_TYPE_FILTER: &str = "Type";
const KEY_MODE: &str = "Mode";
const KEY_REGION: &str = "Region";
const KEY_BATTLEYE_REQUIRED: &str = "BattlEyeRequired";
const KEY_INCLUDE_INVALID: &str = "IncludeInvalid";
const KEY_INCLUDE_PASSWORD_PROTECTED: &str = "IncludePasswordProtected";
const KEY_MODS: &str = "Mods";
const KEY_SORT_CRITERIA: &str = "SortBy";
const KEY_SCROLL_LOCK: &str = "ScrollLock";

const BATTLEYE_AUTO: &str = "auto";
const BATTLEYE_ALWAYS: &str = "always";
const BATTLEYE_NEVER: &str = "never";
