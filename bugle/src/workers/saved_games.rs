use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use fltk::app;

use crate::game::{create_empty_db, Game};
use crate::gui::{SinglePlayerUpdate, Update};
use crate::Message;

pub struct SavedGamesWorker {
    game: Arc<Game>,
    tx: app::Sender<Message>,
}

impl SavedGamesWorker {
    pub fn new(game: Arc<Game>, tx: app::Sender<Message>) -> Arc<Self> {
        Arc::new(Self { game, tx })
    }

    pub fn list_games(self: Arc<Self>) -> Result<()> {
        tokio::spawn(async move {
            let games = self.game.load_saved_games();
            self.tx.send(Message::Update(Update::SinglePlayer(
                SinglePlayerUpdate::PopulateList(games),
            )));
        });
        Ok(())
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
}
