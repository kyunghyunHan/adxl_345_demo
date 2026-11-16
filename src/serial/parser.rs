use crate::aircraft::attitude::Attitude;

pub const MAX_LINE: usize = 256;

pub fn parse_attitude(line: &str) -> Option<Attitude> {
    let mut roll = None;
    let mut pitch = None;
    for field in line.trim().split(',') {
        let (name, value) = field.trim().split_once(':')?;
        let value = value.trim().parse::<f32>().ok().filter(|v| v.is_finite())?;
        match name {
            "ROLL" if roll.is_none() => roll = Some(value),
            "PITCH" if pitch.is_none() => pitch = Some(value),
            _ => return None,
        }
    }
    Some(Attitude {
        roll: roll?,
        pitch: pitch?,
    })
}

/// Keep partial packets across read timeouts. Discard an oversized packet through its newline.
#[derive(Default)]
pub struct LineDecoder {
    bytes: Vec<u8>,
    dropping: bool,
}
impl LineDecoder {
    pub fn push(&mut self, byte: u8) -> Option<Option<Attitude>> {
        if byte == b'\n' {
            let value = (!self.dropping)
                .then(|| {
                    std::str::from_utf8(&self.bytes)
                        .ok()
                        .and_then(parse_attitude)
                })
                .flatten();
            self.bytes.clear();
            self.dropping = false;
            return Some(value);
        }
        if !self.dropping {
            if self.bytes.len() == MAX_LINE {
                self.bytes.clear();
                self.dropping = true;
            } else {
                self.bytes.push(byte);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn valid_packets() {
        for (s, r, p) in [
            ("ROLL:0,PITCH:0", 0., 0.),
            ("ROLL:-12.3,PITCH:5.4\r\n", -12.3, 5.4),
            ("ROLL:45.0,PITCH:-20.0", 45., -20.),
        ] {
            assert_eq!(parse_attitude(s), Some(Attitude { roll: r, pitch: p }));
        }
    }
    #[test]
    fn rejects_malformed() {
        for s in [
            "ROLL:abc,PITCH:10",
            "ROLL:NaN,PITCH:10",
            "ROLL:inf,PITCH:10",
            "ROLL:1",
            "ROLL:1,PITCH:2,ROLL:3",
            "ROLL:1,PITCH:2,EXTRA:0",
            "ROLL:1e99,PITCH:0",
        ] {
            assert!(parse_attitude(s).is_none(), "{s}");
        }
    }
    #[test]
    fn bounded_decoder_recovers_and_keeps_fragments() {
        let mut decoder = LineDecoder::default();
        for _ in 0..10000 {
            assert!(decoder.push(b'x').is_none());
        }
        assert!(decoder.bytes.len() <= MAX_LINE);
        assert_eq!(decoder.push(b'\n'), Some(None));
        for b in b"ROLL:1,PITCH:" {
            assert!(decoder.push(*b).is_none());
        }
        for b in b"2\r" {
            assert!(decoder.push(*b).is_none());
        }
        assert_eq!(
            decoder.push(b'\n'),
            Some(Some(Attitude {
                roll: 1.,
                pitch: 2.
            }))
        );
    }
}
