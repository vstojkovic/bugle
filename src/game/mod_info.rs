use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{anyhow, bail, Result};
use binread::{BinReaderExt, BinResult};
use serde::Deserialize;

use super::engine::pak::Archive;

#[derive(Debug, Deserialize)]
pub struct ModInfo {
    pub name: String,
    pub description: String,

    #[serde(rename = "changeNote")]
    pub change_notes: String,

    pub author: String,

    #[serde(rename = "authorUrl")]
    pub author_url: String,

    #[serde(flatten)]
    pub version: ModVersion,

    #[serde(rename = "bRequiresLoadOnStartup")]
    pub requires_load_on_startup: bool,

    #[serde(rename = "steamPublishedFileId")]
    pub live_steam_file_id: String,

    #[serde(rename = "steamTestLivePublishedFileId")]
    pub testlive_steam_file_id: Option<String>,

    #[serde(rename = "folderName")]
    pub folder_name: String,

    #[serde(rename = "revisionNumber")]
    pub revision_number: u64,

    #[serde(rename = "snapshotId")]
    pub snapshot_id: u64,

    #[serde(skip)]
    pub pak_path: Arc<PathBuf>,
}

#[derive(Debug, Deserialize)]
pub struct ModVersion {
    #[serde(rename = "versionMajor")]
    major: u64,

    #[serde(rename = "versionMinor")]
    minor: u64,

    #[serde(rename = "versionBuild")]
    build: u64,
}

impl ModInfo {
    pub(super) fn new(pak_path: PathBuf) -> Result<Self> {
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

        Ok(Self {
            pak_path: Arc::new(pak_path),
            ..serde_json::from_slice(&json_bytes)?
        })
    }
}

impl ToString for ModVersion {
    fn to_string(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.build)
    }
}
