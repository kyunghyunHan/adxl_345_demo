//! Latest-value mailbox: UART I/O never runs on the rendering thread.
use bevy::prelude::*;
use std::{
    io::{ErrorKind, Read},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Attitude {
    pub roll: f32,
    pub pitch: f32,
}

pub fn parse(line: &str) -> Option<Attitude> {
    let (mut roll, mut pitch) = (None, None);
    for field in line.trim().split(',') {
        let (key, value) = field.trim().split_once(':')?;
        let value = value.trim().parse::<f32>().ok().filter(|n| n.is_finite())?;
        match key {
            "ROLL" if roll.is_none() && value.abs() <= 180. => roll = Some(value),
            "PITCH" if pitch.is_none() && value.abs() <= 90. => pitch = Some(value),
            _ => return None,
        }
    }
    Some(Attitude {
        roll: roll?,
        pitch: pitch?,
    })
}

#[derive(Default)]
pub struct Decoder {
    bytes: Vec<u8>,
    dropping: bool,
}
impl Decoder {
    pub fn push(&mut self, b: u8) -> Option<Option<Attitude>> {
        if b == b'\n' {
            let packet = if self.dropping {
                None
            } else {
                std::str::from_utf8(&self.bytes).ok().and_then(parse)
            };
            self.bytes.clear();
            self.dropping = false;
            return Some(packet);
        }
        if !self.dropping {
            if self.bytes.len() >= 256 {
                self.bytes.clear();
                self.dropping = true;
            } else {
                self.bytes.push(b);
            }
        }
        None
    }
}

#[derive(Clone, Default)]
pub struct Snapshot {
    pub port: String,
    pub open: bool,
    pub raw: Attitude,
    pub last: Option<Instant>,
    pub sequence: u64,
    pub invalid: u64,
    pub error: String,
}
impl Snapshot {
    pub fn fresh(&self) -> bool {
        self.fresh_at(Instant::now())
    }
    fn fresh_at(&self, now: Instant) -> bool {
        self.open
            && self
                .last
                .is_some_and(|t| now.saturating_duration_since(t) < Duration::from_millis(750))
    }
    pub fn status(&self) -> &'static str {
        if self.fresh() {
            "CONNECTED"
        } else if self.open {
            "STALE / LEVEL INPUT"
        } else {
            "DISCONNECTED"
        }
    }
}
#[derive(Resource)]
pub struct SerialFeed {
    pub snapshot: Snapshot,
    shared: Arc<Mutex<Snapshot>>,
    stop: Arc<AtomicBool>,
}
impl Drop for SerialFeed {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

pub fn ports() -> Result<Vec<serialport::SerialPortInfo>, serialport::Error> {
    serialport::available_ports()
}
fn choose_port() -> Result<String, String> {
    let available = ports().map_err(|e| e.to_string())?;
    let candidates: Vec<_> = available
        .into_iter()
        .filter(|p| {
            p.port_name.starts_with("/dev/cu.")
                && matches!(p.port_type, serialport::SerialPortType::UsbPort(_))
        })
        .collect();
    let st: Vec<_> = candidates
        .iter()
        .filter(
            |p| matches!(&p.port_type, serialport::SerialPortType::UsbPort(u) if u.vid == 0x0483),
        )
        .collect();
    if st.len() == 1 {
        return Ok(st[0].port_name.clone());
    }
    if candidates.len() == 1 {
        return Ok(candidates[0].port_name.clone());
    }
    Err("Connect ST-LINK USB or select --port; use --list-ports".into())
}
impl SerialFeed {
    pub fn start(manual: Option<String>, enabled: bool) -> Self {
        Self::start_with_baud(manual, enabled, 115_200)
    }
    fn start_with_baud(manual: Option<String>, enabled: bool, baud: u32) -> Self {
        let shared = Arc::new(Mutex::new(Snapshot::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let feed = Self {
            snapshot: Snapshot::default(),
            shared: shared.clone(),
            stop: stop.clone(),
        };
        if !enabled {
            return feed;
        }
        thread::spawn(move || {
            let mut state = Snapshot::default();
            let publish = |state: &Snapshot| {
                if let Ok(mut slot) = shared.lock() {
                    *slot = state.clone();
                }
            };
            while !stop.load(Ordering::Relaxed) {
                let selected = manual.clone().map(Ok).unwrap_or_else(choose_port);
                let opened = selected.and_then(|name| {
                    state.port = name.clone();
                    serialport::new(name, baud)
                        .data_bits(serialport::DataBits::Eight)
                        .parity(serialport::Parity::None)
                        .stop_bits(serialport::StopBits::One)
                        .flow_control(serialport::FlowControl::None)
                        .timeout(Duration::from_millis(100))
                        .open()
                        .map_err(|e| e.to_string())
                });
                match opened {
                    Ok(mut port) => {
                        state.open = true;
                        state.last = None;
                        state.error.clear();
                        publish(&state);
                        let mut decoder = Decoder::default();
                        let mut bytes = [0; 512];
                        while !stop.load(Ordering::Relaxed) {
                            match port.read(&mut bytes) {
                                Ok(0) => {
                                    state.error = "Serial stream closed".into();
                                    break;
                                }
                                Ok(n) => {
                                    for &b in &bytes[..n] {
                                        if let Some(packet) = decoder.push(b) {
                                            if let Some(raw) = packet {
                                                state.raw = raw;
                                                state.sequence += 1;
                                                state.last = Some(Instant::now());
                                            } else {
                                                state.invalid += 1;
                                            }
                                        }
                                    }
                                    publish(&state);
                                }
                                Err(e)
                                    if matches!(
                                        e.kind(),
                                        ErrorKind::TimedOut
                                            | ErrorKind::WouldBlock
                                            | ErrorKind::Interrupted
                                    ) =>
                                {
                                    ()
                                }
                                Err(e) => {
                                    state.error = e.to_string();
                                    break;
                                }
                            }
                        }
                    }
                    Err(e) => state.error = e,
                }
                state.open = false;
                publish(&state);
                for _ in 0..10 {
                    if stop.load(Ordering::Relaxed) {
                        return;
                    }
                    thread::sleep(Duration::from_millis(100));
                }
            }
        });
        feed
    }
}

#[cfg(all(test, unix))]
#[path = "serial_pty_test.rs"]
mod pty_test;
pub fn receive(mut feed: ResMut<SerialFeed>) {
    let snapshot = feed.shared.try_lock().ok().map(|s| s.clone());
    if let Some(snapshot) = snapshot {
        feed.snapshot = snapshot;
    }
}

/// Bounded terminal-only diagnostic through the production UART worker.
pub fn diagnose(feed: &SerialFeed) {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(6) {
        let s = feed.shared.lock().unwrap().clone();
        println!(
            "{} port={} packets={} invalid={} roll={:.2} pitch={:.2} error={}",
            s.status(),
            s.port,
            s.sequence,
            s.invalid,
            s.raw.roll,
            s.raw.pitch,
            s.error
        );
        thread::sleep(Duration::from_millis(500));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn protocol_and_invalid_packets() {
        assert_eq!(
            parse("ROLL:12.3,PITCH:-5.4\r\n"),
            Some(Attitude {
                roll: 12.3,
                pitch: -5.4
            })
        );
        for line in [
            "ROLL:NaN,PITCH:2",
            "ROLL:1,PITCH:inf",
            "ROLL:181,PITCH:0",
            "ROLL:0,PITCH:91",
            "ROLL:1",
            "ROLL:1,ROLL:2,PITCH:3",
            "booting",
            "ROLL:1,PITCH:2,X:3",
        ] {
            assert!(parse(line).is_none(), "{line}");
        }
    }
    #[test]
    fn framing_is_bounded_and_recovers() {
        let mut d = Decoder::default();
        for _ in 0..10000 {
            d.push(b'x');
        }
        assert!(d.bytes.len() <= 256);
        assert_eq!(d.push(b'\n'), Some(None));
        for b in b"ROLL:1," {
            assert!(d.push(*b).is_none());
        }
        // A read timeout deliberately does not reset partial packets.
        for b in b"PITCH:2\r" {
            assert!(d.push(*b).is_none());
        }
        assert_eq!(
            d.push(b'\n'),
            Some(Some(Attitude {
                roll: 1.,
                pitch: 2.
            }))
        );
        d.push(255);
        assert_eq!(d.push(b'\n'), Some(None));
    }
    #[test]
    fn stale_or_closed_is_not_connected() {
        let now = Instant::now();
        let mut s = Snapshot {
            open: true,
            last: Some(now),
            ..Default::default()
        };
        assert!(s.fresh_at(now));
        assert!(!s.fresh_at(now + Duration::from_millis(751)));
        s.open = false;
        assert!(!s.fresh_at(now));
    }
}
