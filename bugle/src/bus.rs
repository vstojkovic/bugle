use dynabus::local::LocalBus;
use dynabus::mpsc::{ChannelBus, Message, Sender};
use fltk::app;

type CrossBeamTx = crossbeam_channel::Sender<Message>;
type CrossBeamRx = crossbeam_channel::Receiver<Message>;

#[derive(Clone)]
pub struct AppSender(CrossBeamTx);

impl Sender for AppSender {
    type Error = crossbeam_channel::TrySendError<Message>;
    fn try_send(&self, message: Message) -> Result<(), Self::Error> {
        self.0.try_send(message)?;
        app::awake();
        Ok(())
    }
}

pub type AppBus = ChannelBus<AppSender, CrossBeamRx, LocalBus>;

pub fn bus() -> AppBus {
    let (tx, rx) = crossbeam_channel::unbounded();
    AppBus::new((AppSender(tx), rx), LocalBus::new())
}
