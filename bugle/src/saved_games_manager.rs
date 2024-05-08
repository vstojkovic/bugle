use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use anyhow::Result;
use fs_extra::file::{copy_with_progress, CopyOptions};

use crate::bus::AppBus;
use crate::game::{create_empty_db, Game};
use crate::gui::{PopulateSinglePlayerGames, TaskProgressMonitor, TaskProgressUpdate};

pub struct SavedGamesManager {
    bus: Rc<RefCell<AppBus>>,
    game: Arc<Game>,
}

pub enum SaveGame {
    InProgress { map_id: usize },
    Backup { name: PathBuf },
    External { path: PathBuf },
}

impl SavedGamesManager {
    pub fn new(bus: Rc<RefCell<AppBus>>, game: Arc<Game>) -> Rc<Self> {
        Rc::new(Self { bus, game })
    }

    pub fn list_games(&self) {
        let game = Arc::clone(&self.game);
        let tx = self.bus.borrow().sender().clone();
        tokio::spawn(async move {
            let games = game.load_saved_games();
            tx.send(PopulateSinglePlayerGames(games)).ok();
        });
    }

    pub fn clear_progress(&self, map_id: usize, fls_account_id: Option<&str>) -> Result<()> {
        create_empty_db(self.game.in_progress_game_path(map_id), fls_account_id)
    }

    pub fn copy_save(&self, src: SaveGame, dest: SaveGame) -> Result<()> {
        let src_path = self.save_path(src);
        let dest_path = self.save_path(dest);
        let result_cell = Arc::new(OnceLock::new());

        {
            let tx = self.bus.borrow().sender().clone();
            let result_cell = Arc::clone(&result_cell);
            tokio::spawn(async move {
                let opts = CopyOptions::new().overwrite(true);
                let result = copy_with_progress(src_path, dest_path, &opts, |progress| {
                    tx.send(TaskProgressUpdate::Running {
                        done: progress.copied_bytes as f64,
                        total: progress.total_bytes as f64,
                    })
                    .ok();
                });
                result_cell.set(result).ok();
                drop(result_cell);
                tx.send(TaskProgressUpdate::Stopped).ok();
            });
        }

        let deadline = Instant::now() + Duration::from_millis(200);
        while result_cell.get().is_none() {
            if !self.bus.borrow().recv_deadline(deadline).unwrap().is_some() {
                break;
            }
        }

        if result_cell.get().is_some() {
            Arc::into_inner(result_cell).unwrap().take().unwrap()?;
            return Ok(());
        }

        let monitor = TaskProgressMonitor::default(
            Rc::clone(&self.bus),
            fltk::app::first_window().as_ref().unwrap(),
            "Progress",
            "Copying the game database",
        );
        monitor.run();

        Arc::into_inner(result_cell).unwrap().take().unwrap()?;
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
