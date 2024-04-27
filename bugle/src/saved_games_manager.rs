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

    pub fn restore_backup(&self, map_id: usize, backup_name: PathBuf) -> Result<()> {
        let src_db_path = self.game.save_path().join(backup_name);
        let dest_db_path = self
            .game
            .save_path()
            .join(&self.game.maps()[map_id].db_name);
        let _ = std::fs::copy(src_db_path, dest_db_path)?;
        Ok(())
    }

    pub fn create_backup(&self, map_id: usize, backup_name: PathBuf) -> Result<()> {
        let src_db_path = self
            .game
            .save_path()
            .join(&self.game.maps()[map_id].db_name);
        let dest_db_path = self.game.save_path().join(backup_name);
        let _ = std::fs::copy(src_db_path, dest_db_path)?;
        Ok(())
    }

    pub fn delete_backup(&self, backup_name: PathBuf) -> Result<()> {
        std::fs::remove_file(self.game.save_path().join(backup_name))?;
        Ok(())
    }
}
