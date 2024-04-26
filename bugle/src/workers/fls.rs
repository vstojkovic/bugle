use std::sync::Arc;

use anyhow::Result;
use dynabus::mpsc::BusSender;
use slog::Logger;

use crate::auth::{playfab, Account};
use crate::bus::AppSender;
use crate::game::platform::steam::SteamTicket;
use crate::game::Game;

pub struct FlsWorker {
    logger: Logger,
    game: Arc<Game>,
    tx: BusSender<AppSender>,
}

#[derive(dynabus::Event)]
pub struct LoginComplete(pub Result<Account>);

impl FlsWorker {
    pub fn new(logger: Logger, game: Arc<Game>, tx: BusSender<AppSender>) -> Arc<Self> {
        Arc::new(Self { logger, game, tx })
    }

    pub fn login_with_steam(self: Arc<Self>, ticket: &SteamTicket) {
        let ticket = ticket.data().into();
        tokio::spawn(async move {
            let account = playfab::login_with_steam(&self.logger, &*self.game, ticket).await;
            self.tx.send(LoginComplete(account)).ok();
        });
    }
}
