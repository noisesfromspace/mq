use crossterm::event::{self, Event as CrosstermEvent};
use std::time::Duration;
use tokio::sync::mpsc;

pub enum AppEvent {
    Input(CrosstermEvent),
    Tick,
}

pub struct EventHandler {
    rx: mpsc::UnboundedReceiver<AppEvent>,
}

impl EventHandler {
    pub fn new(tick_rate: u64) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            loop {
                if event::poll(Duration::from_millis(tick_rate)).unwrap_or(false) {
                    if let Ok(event) = event::read() {
                        if tx_clone.send(AppEvent::Input(event)).is_err() {
                            break;
                        }
                    }
                }
                if tx_clone.send(AppEvent::Tick).is_err() {
                    break;
                }
            }
        });
        Self { rx }
    }

    pub async fn next(&mut self) -> Option<AppEvent> {
        self.rx.recv().await
    }
}
