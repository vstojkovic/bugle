use crate::mpsc::{Message, Receiver, Sender};

impl Sender for crossbeam_channel::Sender<Message> {
    type Error = crossbeam_channel::TrySendError<Message>;
    fn try_send(&self, message: Message) -> Result<(), Self::Error> {
        self.try_send(message)
    }
}

impl Receiver for crossbeam_channel::Receiver<Message> {
    type Error = crossbeam_channel::TryRecvError;
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
