use tokio::sync::mpsc;

pub struct ProgressReporter(Option<mpsc::UnboundedSender<String>>);

impl ProgressReporter {
    pub fn new(tx: Option<mpsc::UnboundedSender<String>>) -> Self {
        Self(tx)
    }

    pub fn report(&self, message: impl Into<String>) {
        if let Some(tx) = &self.0 {
            let _ = tx.send(message.into());
        }
    }
}
