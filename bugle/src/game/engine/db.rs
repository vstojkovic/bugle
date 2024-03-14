use std::fs::File;
use std::ops::Deref;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use chrono::{DateTime, Local, NaiveDateTime};
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ValueRef};
use rusqlite::Connection;

#[derive(Clone, Copy, Debug)]
pub struct UnixTimestamp(NaiveDateTime);

impl FromSql for UnixTimestamp {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let millis = i64::column_result(value)? * 1000;
        if let Some(ts) = DateTime::from_timestamp_millis(millis) {
            Ok(Self(ts.with_timezone(&Local).naive_local()))
        } else {
            Err(FromSqlError::OutOfRange(millis))
        }
    }
}

impl Deref for UnixTimestamp {
    type Target = NaiveDateTime;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub struct GameDB {
    pub file_name: PathBuf,
    pub map_id: usize,
    pub last_played_char: Option<Character>,
}

#[derive(Clone, Debug)]
pub struct Character {
    pub name: String,
    pub clan: Option<String>,
    pub level: u32,
    pub last_played_timestamp: UnixTimestamp,
}

impl GameDB {
    pub(in crate::game) fn new<P: AsRef<Path>, F: Fn(&str) -> Option<usize>>(
        file_path: P,
        map_resolver: F,
    ) -> Result<Self> {
        let file_path = file_path.as_ref();
        let db = Connection::open(file_path)?;
        let map_id = get_db_map_id(&db, map_resolver)?;
        let last_played_char = get_db_last_played_char(&db)?;

        Ok(Self {
            file_name: file_path.file_name().unwrap().into(),
            map_id,
            last_played_char,
        })
    }

    pub fn copy_from(other: &Self, file_name: &Path) -> Self {
        Self {
            file_name: file_name.to_owned(),
            map_id: other.map_id,
            last_played_char: other.last_played_char.clone(),
        }
    }
}

fn get_db_map_id<F: Fn(&str) -> Option<usize>>(db: &Connection, map_resolver: F) -> Result<usize> {
    let mut query = db.prepare("SELECT DISTINCT map FROM actor_position")?;
    let mut rows = query.query([])?;

    let row = if let Some(row) = rows.next()? {
        row
    } else {
        bail!("No actors found in game database.");
    };

    let map_obj_name: String = row.get(0)?;
    let map_id = if let Some(id) = map_resolver(&map_obj_name) {
        id
    } else {
        bail!("Unrecognized map found in game database.")
    };

    if rows.next()?.is_some() {
        bail!("Multiple maps found in game database.");
    };

    Ok(map_id)
}

pub fn list_mod_controllers<P: AsRef<Path>>(db_path: P) -> Result<Vec<String>> {
    let db = Connection::open(db_path.as_ref())?;
    let mut query = db
        .prepare("SELECT class FROM actor_position WHERE id IN (SELECT id FROM mod_controllers)")?;
    let controllers: rusqlite::Result<_> = query.query_map([], |row| row.get(0))?.collect();
    Ok(controllers?)
}

pub fn create_empty_db<P: AsRef<Path>>(db_path: P, fls_account_id: Option<&str>) -> Result<()> {
    let _ = File::create(db_path.as_ref())?;
    if let Some(account_id) = fls_account_id {
        let db = Connection::open(db_path.as_ref())?;

        let mut stmt = db.prepare(
            "CREATE TABLE account (
                id     INTEGER PRIMARY KEY AUTOINCREMENT,
                user   TEXT    UNIQUE,
                online BOOL    NOT NULL
                            DEFAULT 0
            );",
        )?;
        stmt.execute([])?;

        let mut stmt = db.prepare("INSERT INTO account (user, online) VALUES (:account_id, 1)")?;
        stmt.execute([account_id])?;
    }
    Ok(())
}

fn get_db_last_played_char(db: &Connection) -> Result<Option<Character>> {
    let mut query = db.prepare(
        "
        SELECT
            c.char_name as name,
            g.name as clan,
            c.level as level,
            c.lastTimeOnline as last_played_timestamp
        FROM characters c LEFT JOIN guilds g ON c.guild = g.guildId
        ORDER BY c.lastTimeOnline DESC
        LIMIT 1
    ",
    )?;
    let mut rows = query.query([])?;

    let row = if let Some(row) = rows.next()? {
        row
    } else {
        return Ok(None);
    };

    let name = row.get("name")?;
    let clan = row.get("clan")?;
    let level = row.get("level")?;
    let last_played_timestamp = row.get("last_played_timestamp")?;

    Ok(Some(Character {
        name,
        clan,
        level,
        last_played_timestamp,
    }))
}
