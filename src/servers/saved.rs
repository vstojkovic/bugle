use std::collections::HashMap;
use std::fs::File;
use std::ops::Index;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::Serialize;
use serde_json::ser::PrettyFormatter;
use uuid::Uuid;

use crate::env::current_exe_dir;

use super::Server;

pub struct SavedServers {
    path: PathBuf,
    servers: HashMap<Uuid, Server>,
}

impl SavedServers {
    #[cfg(not(windows))]
    pub fn new() -> Result<Self> {
        Self::for_current_exe()
    }

    #[cfg(windows)]
    pub fn new() -> Result<Self> {
        Self::for_current_exe().or_else(|_| Self::in_appdata())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&mut self) -> Result<()> {
        let json = std::fs::read_to_string(&self.path)?;
        let mut servers: HashMap<Uuid, Server> =
            if json.is_empty() { HashMap::new() } else { serde_json::from_str(&json)? };
        for (id, server) in servers.iter_mut() {
            server.saved_id = Some(*id);
        }
        self.servers = servers;
        Ok(())
    }

    pub fn save(&self) -> Result<()> {
        let file = File::options()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.path)?;
        let fmt = PrettyFormatter::with_indent(b"  ");
        let mut serializer = serde_json::Serializer::with_formatter(file, fmt);
        Ok(self.servers.serialize(&mut serializer)?)
    }

    pub fn add(&mut self, mut server: Server) -> Uuid {
        let id = Uuid::new_v4();
        server.saved_id = Some(id);
        self.servers.insert(id, server);
        id
    }

    pub fn remove(&mut self, id: Uuid) {
        self.servers.remove(&id);
    }

    pub fn is_empty(&self) -> bool {
        self.servers.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Server> {
        self.servers.values()
    }

    fn for_current_exe() -> Result<Self> {
        Self::open(current_exe_dir()?)
    }

    #[cfg(windows)]
    fn in_appdata() -> Result<Self> {
        use crate::env::{appdata_dir, AppDataFolder};

        let mut path = appdata_dir(AppDataFolder::Roaming)?;
        path.push("bugle");
        std::fs::create_dir_all(&path)?;

        Self::open(path)
    }

    fn open(mut path: PathBuf) -> Result<Self> {
        path.push("bugle-saved-servers.json");
        let _ = File::options()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)?;
        Ok(Self {
            path,
            servers: HashMap::new(),
        })
    }
}

impl Index<Uuid> for SavedServers {
    type Output = Server;
    fn index(&self, index: Uuid) -> &Self::Output {
        &self.servers[&index]
    }
}
