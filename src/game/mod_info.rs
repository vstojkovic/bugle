use std::borrow::Cow;
use std::collections::HashMap;
use std::io::Read;
use std::ops::Index;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use anyhow::{anyhow, bail, Result};
use binread::{BinReaderExt, BinResult};
use serde::Deserialize;

use super::engine::pak::Archive;
use super::Branch;

#[derive(Debug, Deserialize)]
pub struct ModInfo {
    pub name: String,

    #[serde(default)]
    pub description: String,

    #[serde(rename = "changenote")]
    #[serde(default)]
    pub change_notes: String,

    #[serde(default)]
    pub author: String,

    #[serde(rename = "authorurl")]
    pub author_url: Option<String>,

    #[serde(flatten)]
    pub version: ModVersion,

    #[serde(rename = "brequiresloadonstartup")]
    pub requires_load_on_startup: bool,

    #[serde(rename = "steampublishedfileid")]
    pub live_steam_file_id: Option<String>,

    #[serde(rename = "steamtestlivepublishedfileid")]
    pub testlive_steam_file_id: Option<String>,

    #[serde(rename = "foldername")]
    pub folder_name: String,

    #[serde(rename = "revisionnumber")]
    pub devkit_revision: u32,

    #[serde(rename = "snapshotid")]
    pub devkit_snapshot: u16,

    #[serde(skip)]
    pub pak_path: PathBuf,

    #[serde(skip)]
    pub pak_size: u64,

    #[serde(skip)]
    needs_update: AtomicBool,
}

#[derive(Debug, Deserialize)]
pub struct ModVersion {
    #[serde(rename = "versionmajor")]
    major: u64,

    #[serde(rename = "versionminor")]
    minor: u64,

    #[serde(rename = "versionbuild")]
    build: u64,
}

impl ModInfo {
    pub(super) fn new(pak_path: PathBuf) -> Result<Self> {
        let pak_size = std::fs::metadata(&pak_path)?.len();
        let pak = Archive::new(&pak_path)?;
        let entry = pak
            .entry("modinfo.json")
            .ok_or(anyhow!("Missing modinfo.json"))?;
        if entry.encrypted {
            bail!("Encrypted archives are not supported");
        }

        let mut reader = pak.open_entry("modinfo.json")?;

        let bom: u16 = reader.read_le()?;
        let json_bytes = if bom == 0xfeff {
            let ucs2_len = (reader.entry.size / 2 - 1) as usize;

            let ucs2_buf = (0..ucs2_len)
                .map(|_| reader.read_le())
                .collect::<BinResult<Vec<u16>>>()?;

            let mut utf8_bytes = vec![0u8; ucs2_len * 3];
            let utf8_len =
                ucs2::decode(&ucs2_buf, &mut utf8_bytes).map_err(|err| anyhow!("{:?}", err))?;

            utf8_bytes.truncate(utf8_len);

            utf8_bytes
        } else {
            let mut bytes = vec![0u8; reader.entry.size as _];
            bytes[0] = (bom & 0xff) as _;
            bytes[1] = (bom >> 8) as _;
            reader.read_exact(&mut bytes[2..])?;

            bytes
        };

        let json = serde_json::from_slice(&json_bytes)?;
        let json = json_lowercase_keys(json);

        Ok(Self {
            pak_path,
            pak_size,
            ..serde_json::from_value(json)?
        })
    }

    pub fn steam_file_id(&self, branch: Branch) -> Option<u64> {
        let id_str = match branch {
            Branch::Main => self.live_steam_file_id.as_ref()?,
            Branch::PublicBeta => self.testlive_steam_file_id.as_ref()?,
        };
        id_str.parse().ok()
    }

    pub fn needs_update(&self) -> bool {
        self.needs_update.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn set_needs_update(&self, value: bool) {
        self.needs_update
            .store(value, std::sync::atomic::Ordering::Relaxed)
    }
}

impl ToString for ModVersion {
    fn to_string(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.build)
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum ModRef {
    Installed(usize),
    UnknownFolder(String),
    UnknownPakPath(PathBuf),
}

impl ModRef {
    pub fn to_index(&self) -> Option<usize> {
        if let &Self::Installed(idx) = self {
            Some(idx)
        } else {
            None
        }
    }
}

pub struct Mods {
    mods: Vec<ModInfo>,
    by_pak_path: HashMap<PathBuf, usize>,
    by_folder: HashMap<String, usize>,
}

impl Mods {
    pub(super) fn new(mods: Vec<ModInfo>) -> Self {
        let mut by_pak_path = HashMap::with_capacity(mods.len());
        for (idx, mod_info) in mods.iter().enumerate() {
            by_pak_path.insert(mod_info.pak_path.clone(), idx);
        }

        let mut by_folder = HashMap::with_capacity(mods.len());
        for (idx, mod_info) in mods.iter().enumerate() {
            by_folder.insert(mod_info.folder_name.clone(), idx);
        }

        Self {
            mods,
            by_pak_path,
            by_folder,
        }
    }

    pub fn len(&self) -> usize {
        self.mods.len()
    }

    pub fn get(&self, mod_ref: &ModRef) -> Option<&ModInfo> {
        if let ModRef::Installed(idx) = mod_ref {
            self.mods.get(*idx)
        } else {
            None
        }
    }

    pub fn by_pak_path<'p, P: Into<Cow<'p, Path>>>(&self, pak_path: P) -> ModRef {
        let pak_path: Cow<'p, Path> = pak_path.into();
        if let Some(&idx) = self.by_pak_path.get(pak_path.as_ref()) {
            ModRef::Installed(idx)
        } else {
            ModRef::UnknownPakPath(pak_path.into_owned())
        }
    }

    pub fn by_folder<'s, S: Into<Cow<'s, str>>>(&self, folder: S) -> ModRef {
        let folder: Cow<'s, str> = folder.into();
        if let Some(&idx) = self.by_folder.get(folder.as_ref()) {
            ModRef::Installed(idx)
        } else {
            ModRef::UnknownFolder(folder.into_owned())
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &ModInfo> {
        self.mods.iter()
    }
}

impl Index<usize> for Mods {
    type Output = ModInfo;
    fn index(&self, index: usize) -> &Self::Output {
        &self.mods[index]
    }
}

fn json_lowercase_keys(json: serde_json::Value) -> serde_json::Value {
    if let serde_json::Value::Object(obj) = json {
        obj.into_iter()
            .map(|(k, v)| (k.to_ascii_lowercase(), v))
            .collect()
    } else {
        json
    }
}
