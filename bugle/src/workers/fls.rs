use std::sync::Arc;

use fltk::app;
use slog::Logger;

use crate::auth::playfab;
use crate::game::platform::steam::SteamTicket;
use crate::game::Game;
use crate::Message;

pub struct FlsWorker {
    logger: Logger,
    game: Arc<Game>,
    tx: app::Sender<Message>,
}

impl FlsWorker {
    pub fn new(logger: Logger, game: Arc<Game>, tx: app::Sender<Message>) -> Arc<Self> {
        Arc::new(Self { logger, game, tx })
    }

    pub fn login_with_steam(self: Arc<Self>, ticket: &SteamTicket) {
        let ticket = ticket.data().into();
        tokio::spawn(async move {
            let account = playfab::login_with_steam(&self.logger, &*self.game, ticket).await;
            self.tx.send(Message::Account(account));
        });
    }
}
