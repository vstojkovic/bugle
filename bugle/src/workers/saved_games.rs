use std::sync::Arc;

use dynabus::mpsc::BusSender;

use crate::bus::AppSender;
use crate::game::Game;
use crate::gui::PopulateSinglePlayerGames;

pub struct SavedGamesWorker {
    game: Arc<Game>,
    tx: BusSender<AppSender>,
}

impl SavedGamesWorker {
    pub fn new(game: Arc<Game>, tx: BusSender<AppSender>) -> Arc<Self> {
        Arc::new(Self { game, tx })
    }

    pub fn list_games(self: &Arc<Self>) {
        let this = Arc::clone(self);
        tokio::spawn(async move {
            let games = this.game.load_saved_games();
            this.tx.send(PopulateSinglePlayerGames(games)).ok();
        });
    }
}
