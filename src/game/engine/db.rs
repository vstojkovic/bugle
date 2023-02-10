use std::ops::Deref;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use chrono::NaiveDateTime;
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ValueRef};
use rusqlite::{Connection, OpenFlags};

#[derive(Debug)]
pub struct UnixTimestamp(NaiveDateTime);

impl FromSql for UnixTimestamp {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let millis = i64::column_result(value)? * 1000;
        if let Some(ts) = NaiveDateTime::from_timestamp_millis(millis) {
            Ok(Self(ts))
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
    pub file_path: PathBuf,
    pub map_id: usize,
    pub last_played_char: Option<Character>,
}

#[derive(Debug)]
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
        let db = Connection::open_with_flags(file_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let map_id = get_db_map_id(&db, map_resolver)?;
        let last_played_char = get_db_last_played_char(&db)?;

        Ok(GameDB {
            file_path: file_path.into(),
            map_id,
            last_played_char,
        })
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
