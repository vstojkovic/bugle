use std::path::{Path, PathBuf};

use anyhow::Result;
use ini::{EscapePolicy, Ini, LineSeparator, ParseOption, WriteOption};

#[derive(Debug, Default)]
pub struct Config {
    pub use_battleye: BattlEyeUsage,
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
        use std::io::{Error, ErrorKind};
        let exe_path = std::env::current_exe()?;
        let exe_dir = exe_path
            .parent()
            .ok_or_else(|| Error::new(ErrorKind::Other, "Malformed executable path"))?;
        let config_path = exe_dir.join("bugle.ini");
        Self::new(config_path)
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
        let ini = load_ini(&self.config_path)?;
        let use_battleye = ini
            .section(None::<String>)
            .and_then(|section| {
                section.get("UseBattlEye").and_then(|value| {
                    match value.trim().to_ascii_lowercase().as_str() {
                        BATTLEYE_AUTO => Some(BattlEyeUsage::Auto),
                        BATTLEYE_ALWAYS => Some(BattlEyeUsage::Always(true)),
                        BATTLEYE_NEVER => Some(BattlEyeUsage::Always(false)),
                        _ => None,
                    }
                })
            })
            .unwrap_or_default();
        Ok(Config { use_battleye })
    }

    fn save(&self, config: &Config) -> Result<()> {
        let mut ini = Ini::new();
        ini.with_general_section().set(
            "UseBattlEye",
            match config.use_battleye {
                BattlEyeUsage::Auto => BATTLEYE_AUTO,
                BattlEyeUsage::Always(true) => BATTLEYE_ALWAYS,
                BattlEyeUsage::Always(false) => BATTLEYE_NEVER,
            },
        );
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

const BATTLEYE_AUTO: &str = "auto";
const BATTLEYE_ALWAYS: &str = "always";
const BATTLEYE_NEVER: &str = "never";
