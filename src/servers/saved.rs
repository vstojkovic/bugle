use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::net::IpAddr;
use std::ops::{Index, IndexMut};
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
    by_id: HashMap<String, HashSet<Uuid>>,
    by_name: HashMap<String, HashSet<Uuid>>,
    by_addr: HashMap<(IpAddr, u32), HashSet<Uuid>>,
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
        self.servers = if json.is_empty() { HashMap::new() } else { serde_json::from_str(&json)? };
        self.by_name.clear();
        self.by_addr.clear();
        for (id, server) in self.servers.iter_mut() {
            server.saved_id = Some(*id);

            let by_id = match self.by_id.get_mut(&server.id) {
                Some(set) => set,
                None => self.by_id.entry(server.id.clone()).or_default(),
            };
            by_id.insert(*id);

            let by_name = match self.by_name.get_mut(&server.name) {
                Some(set) => set,
                None => self.by_name.entry(server.name.clone()).or_default(),
            };
            by_name.insert(*id);

            self.by_addr
                .entry((server.ip, server.port))
                .or_default()
                .insert(*id);
        }
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
        let server = match self.servers.remove(&id) {
            Some(server) => server,
            None => return,
        };
        self.by_id.get_mut(&server.id).unwrap().remove(&id);
        self.by_name.get_mut(&server.name).unwrap().remove(&id);
        self.by_addr
            .get_mut(&(server.ip, server.port))
            .unwrap()
            .remove(&id);
    }

    pub fn is_empty(&self) -> bool {
        self.servers.is_empty()
    }

    pub fn len(&self) -> usize {
        self.servers.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Server> {
        self.servers.values()
    }

    pub fn get(&self, id: &Uuid) -> Option<&Server> {
        self.servers.get(&id)
    }

    pub fn get_mut(&mut self, id: &Uuid) -> Option<&mut Server> {
        self.servers.get_mut(&id)
    }

    pub fn by_id(&self, id: &str) -> impl Iterator<Item = Uuid> + '_ {
        self.by_id.get(id).into_iter().flatten().copied()
    }

    pub fn by_name(&self, name: &str) -> impl Iterator<Item = Uuid> + '_ {
        self.by_name.get(name).into_iter().flatten().copied()
    }

    pub fn by_addr(&self, ip: IpAddr, port: u32) -> impl Iterator<Item = Uuid> + '_ {
        self.by_addr.get(&(ip, port)).into_iter().flatten().copied()
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
            by_id: HashMap::new(),
            by_name: HashMap::new(),
            by_addr: HashMap::new(),
        })
    }
}

impl Index<Uuid> for SavedServers {
    type Output = Server;
    fn index(&self, id: Uuid) -> &Self::Output {
        self.get(&id).unwrap()
    }
}

impl IndexMut<Uuid> for SavedServers {
    fn index_mut(&mut self, id: Uuid) -> &mut Self::Output {
        self.get_mut(&id).unwrap()
    }
}
