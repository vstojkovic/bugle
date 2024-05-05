use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use anyhow::Result;

use crate::bus::AppBus;
use crate::game::{create_empty_db, Game};
use crate::workers::SavedGamesWorker;

pub struct SavedGamesManager {
    game: Arc<Game>,
    worker: Arc<SavedGamesWorker>,
}

pub enum SaveGame {
    InProgress { map_id: usize },
    Backup { name: PathBuf },
    External { path: PathBuf },
}

impl SavedGamesManager {
    pub fn new(bus: Rc<RefCell<AppBus>>, game: Arc<Game>) -> Rc<Self> {
        let worker = SavedGamesWorker::new(Arc::clone(&game), bus.borrow().sender().clone());
        Rc::new(Self { game, worker })
    }

    pub fn list_games(&self) {
        self.worker.list_games();
    }

    pub fn clear_progress(&self, map_id: usize, fls_account_id: Option<&str>) -> Result<()> {
        create_empty_db(self.game.in_progress_game_path(map_id), fls_account_id)
    }

    pub fn copy_save(&self, src: SaveGame, dest: SaveGame) -> Result<()> {
        let src_path = self.save_path(src);
        let dest_path = self.save_path(dest);
        std::fs::copy(src_path, dest_path)?;
        Ok(())
    }

    fn save_path(&self, save_src: SaveGame) -> PathBuf {
        match save_src {
            SaveGame::InProgress { map_id } => self
                .game
                .save_path()
                .join(&self.game.maps()[map_id].db_name),
            SaveGame::Backup { name } => self.game.save_path().join(name),
            SaveGame::External { path } => path,
        }
    }

    pub fn delete_backup(&self, backup_name: PathBuf) -> Result<()> {
        std::fs::remove_file(self.game.save_path().join(backup_name))?;
        Ok(())
    }
}
