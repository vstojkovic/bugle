use std::time::Instant;

use crossbeam_channel::{RecvTimeoutError, TryRecvError, TrySendError};

use crate::mpsc::{BlockingReceiver, Message, Receiver, Sender};

impl Sender for crossbeam_channel::Sender<Message> {
    type Error = TrySendError<Message>;
    fn try_send(&self, message: Message) -> Result<(), Self::Error> {
        self.try_send(message)
    }
}

impl Receiver for crossbeam_channel::Receiver<Message> {
    type Error = TryRecvError;
    fn try_recv(&self) -> Result<Option<Message>, Self::Error> {
        match self.try_recv() {
            Ok(message) => Ok(Some(message)),
            Err(err) => {
                if err.is_empty() {
                    Ok(None)
                } else {
                    Err(err)
                }
            }
        }
    }
}

impl BlockingReceiver for crossbeam_channel::Receiver<Message> {
    fn recv_deadline(&self, deadline: Instant) -> Result<Option<Message>, Self::Error> {
        match self.recv_deadline(deadline) {
            Ok(message) => Ok(Some(message)),
            Err(RecvTimeoutError::Timeout) => Ok(None),
            Err(RecvTimeoutError::Disconnected) => Err(Self::Error::Disconnected),
        }
    }
}
