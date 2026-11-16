pub mod parser;
use crate::{
    aircraft::attitude::{Attitude, AttitudeState},
    config::Config,
};
use bevy::prelude::*;
use std::{
    io::{ErrorKind, Read},
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver},
    },
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Default)]
pub struct Snapshot {
    pub open: bool,
    pub last_received: Option<Instant>,
    pub raw: Attitude,
    pub sequence: u64,
    pub rate: f32,
    pub invalid: u64,
    pub error: String,
}
impl Snapshot {
    pub fn status(&self, now: Instant) -> &'static str {
        if !self.open {
            "DISCONNECTED"
        } else if self
            .last_received
            .is_some_and(|t| now.saturating_duration_since(t) < Duration::from_secs(2))
        {
            "CONNECTED"
        } else {
            "WAITING FOR UART"
        }
    }
}
#[derive(Resource)]
pub struct SerialFeed {
    pub snapshot: Snapshot,
    shared: Arc<Mutex<Snapshot>>,
    rx: Mutex<Receiver<()>>,
}
impl SerialFeed {
    pub fn start(port: Option<String>) -> Self {
        Self::start_with_baud(port, 115_200)
    }
    fn start_with_baud(port: Option<String>, baud: u32) -> Self {
        let shared = Arc::new(Mutex::new(Snapshot::default()));
        let (tx, rx) = mpsc::sync_channel(1);
        let feed = Self {
            snapshot: Snapshot::default(),
            shared: shared.clone(),
            rx: Mutex::new(rx),
        };
        if let Some(port) = port {
            thread::spawn(move || {
                let publish = |state: &Snapshot| -> bool {
                    if let Ok(mut slot) = shared.lock() {
                        *slot = state.clone();
                    }
                    !matches!(tx.try_send(()), Err(mpsc::TrySendError::Disconnected(_)))
                };
                let mut state = Snapshot::default();
                loop {
                    match serialport::new(&port, baud)
                        .timeout(Duration::from_millis(150))
                        .open()
                    {
                        Ok(mut serial) => {
                            state.open = true;
                            state.last_received = None;
                            state.error.clear();
                            state.rate = 0.;
                            if !publish(&state) {
                                return;
                            }
                            let mut decoder = parser::LineDecoder::default();
                            let mut buffer = [0u8; 512];
                            let mut rate_start = Instant::now();
                            let mut count = 0;
                            loop {
                                match serial.read(&mut buffer) {
                                    Ok(0) => {
                                        state.error = "Serial end of stream".into();
                                        break;
                                    }
                                    Ok(n) => {
                                        for &b in &buffer[..n] {
                                            if let Some(packet) = decoder.push(b) {
                                                if let Some(a) = packet {
                                                    state.raw = a;
                                                    state.sequence += 1;
                                                    count += 1;
                                                    state.last_received = Some(Instant::now());
                                                } else {
                                                    state.invalid += 1;
                                                }
                                            }
                                        }
                                        let elapsed = rate_start.elapsed().as_secs_f32();
                                        if elapsed >= 0.5 {
                                            state.rate = count as f32 / elapsed;
                                            count = 0;
                                            rate_start = Instant::now();
                                        }
                                        if !publish(&state) {
                                            return;
                                        }
                                    }
                                    Err(e)
                                        if matches!(
                                            e.kind(),
                                            ErrorKind::TimedOut
                                                | ErrorKind::WouldBlock
                                                | ErrorKind::Interrupted
                                        ) =>
                                    {
                                        if !publish(&state) {
                                            return;
                                        }
                                    }
                                    Err(e) => {
                                        state.error = e.to_string();
                                        break;
                                    }
                                }
                            }
                        }
                        Err(e) => state.error = e.to_string(),
                    }
                    state.open = false;
                    state.rate = 0.;
                    if !publish(&state) {
                        return;
                    }
                    thread::sleep(Duration::from_secs(1));
                }
            });
        }
        feed
    }
}

pub fn receive(
    config: Res<Config>,
    mut feed: ResMut<SerialFeed>,
    mut attitude: ResMut<AttitudeState>,
) {
    if config.demo() {
        return;
    }
    let notified = feed.rx.get_mut().is_ok_and(|rx| rx.try_recv().is_ok());
    if notified {
        let snapshot = feed.shared.lock().ok().map(|s| s.clone());
        if let Some(snapshot) = snapshot {
            if snapshot.sequence != feed.snapshot.sequence {
                attitude.ingest(snapshot.raw);
            }
            feed.snapshot = snapshot;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn status_requires_recent_data_and_open_port() {
        let now = Instant::now();
        let mut s = Snapshot::default();
        assert_eq!(s.status(now), "DISCONNECTED");
        s.open = true;
        assert_eq!(s.status(now), "WAITING FOR UART");
        s.last_received = Some(now);
        assert_eq!(s.status(now), "CONNECTED");
        assert_eq!(s.status(now + Duration::from_secs(2)), "WAITING FOR UART");
        s.open = false;
        assert_eq!(s.status(now), "DISCONNECTED");
    }
}

#[cfg(all(test, unix))]
mod pty_test;
