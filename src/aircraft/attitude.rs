use crate::config::Config;
use bevy::prelude::*;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Attitude {
    pub roll: f32,
    pub pitch: f32,
}

pub fn wrap(degrees: f32) -> f32 {
    (degrees + 180.).rem_euclid(360.) - 180.
}
/// Sensor -> body: invert first, then additive mounting/calibration offsets.
pub fn sensor_to_body(raw: Attitude, config: &Config) -> Attitude {
    Attitude {
        roll: wrap(raw.roll * if config.invert_roll { -1. } else { 1. } + config.roll_offset),
        pitch: wrap(raw.pitch * if config.invert_pitch { -1. } else { 1. } + config.pitch_offset),
    }
}
/// Body uses +X starboard, +Y up, -Z nose. Positive bank lowers the right wing;
/// positive pitch raises the nose. Intrinsic pitch then roll: q_pitch * q_roll.
/// Yaw input is always zero (Euler heading convention, YXZ).
pub fn body_to_bevy(a: Attitude) -> Quat {
    Quat::from_rotation_x(a.pitch.to_radians()) * Quat::from_rotation_z(-a.roll.to_radians())
}
pub fn smoothing_alpha(response: f32, dt: f32) -> f32 {
    1. - (-response * dt).exp()
}

#[derive(Resource, Default)]
pub struct AttitudeState {
    pub raw: Attitude,
    pub accepted: Attitude,
    pub display: Quat,
    pub rejected: u64,
    candidate: Option<(Attitude, u8)>,
    initialized: bool,
}
impl AttitudeState {
    pub fn ingest(&mut self, a: Attitude) {
        self.raw = a;
        if !a.roll.is_finite() || !a.pitch.is_finite() || a.roll.abs() > 180. || a.pitch.abs() > 90.
        {
            self.rejected += 1;
            return;
        }
        // A one-frame jump over 60 degrees is held until three consistent updates confirm it.
        if self.initialized
            && (wrap(a.roll - self.accepted.roll).abs() > 60.
                || (a.pitch - self.accepted.pitch).abs() > 60.)
        {
            let count = self
                .candidate
                .filter(|(v, _)| {
                    wrap(v.roll - a.roll).abs() < 10. && (v.pitch - a.pitch).abs() < 10.
                })
                .map_or(1, |(_, n)| n + 1);
            if count < 3 {
                self.candidate = Some((a, count));
                self.rejected += 1;
                return;
            }
        }
        self.accepted = a;
        self.initialized = true;
        self.candidate = None;
    }
    pub fn displayed_angles(&self) -> Attitude {
        let (_, pitch, bank) = self.display.to_euler(EulerRot::YXZ);
        Attitude {
            roll: -bank.to_degrees(),
            pitch: pitch.to_degrees(),
        }
    }
}
pub fn process(time: Res<Time>, config: Res<Config>, mut state: ResMut<AttitudeState>) {
    let target = body_to_bevy(sensor_to_body(state.accepted, &config));
    state.display = state
        .display
        .slerp(target, smoothing_alpha(config.response, time.delta_secs()));
}
pub fn calibration(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<AttitudeState>,
    mut config: ResMut<Config>,
) {
    if keys.just_pressed(KeyCode::KeyI) {
        config.invert_roll = !config.invert_roll;
    }
    if keys.just_pressed(KeyCode::KeyK) {
        config.invert_pitch = !config.invert_pitch;
    }
    if keys.just_pressed(KeyCode::KeyZ) {
        config.roll_offset = -state.raw.roll * if config.invert_roll { -1. } else { 1. };
        config.pitch_offset = -state.raw.pitch * if config.invert_pitch { -1. } else { 1. };
    }
    if keys.just_pressed(KeyCode::KeyC) {
        config.roll_offset = 0.;
        config.pitch_offset = 0.;
    }
    let step = if keys.pressed(KeyCode::ShiftLeft) {
        0.1
    } else {
        1.
    };
    if keys.just_pressed(KeyCode::BracketLeft) {
        config.roll_offset -= step;
    }
    if keys.just_pressed(KeyCode::BracketRight) {
        config.roll_offset += step;
    }
    if keys.just_pressed(KeyCode::Minus) {
        config.pitch_offset -= step;
    }
    if keys.just_pressed(KeyCode::Equal) {
        config.pitch_offset += step;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn near(a: Vec3, b: Vec3) {
        assert!((a - b).length() < 0.0001, "{a:?} != {b:?}");
    }
    #[test]
    fn body_axes_and_rotation_order() {
        near(body_to_bevy(Attitude::default()) * Vec3::NEG_Z, Vec3::NEG_Z);
        for r in [-30_f32, 30.] {
            let q = body_to_bevy(Attitude { roll: r, pitch: 0. });
            near(
                q * Vec3::X,
                Vec3::new(r.to_radians().cos(), -r.to_radians().sin(), 0.),
            );
            near(q * Vec3::NEG_Z, Vec3::NEG_Z);
        }
        for p in [-20_f32, 20.] {
            near(
                body_to_bevy(Attitude { roll: 0., pitch: p }) * Vec3::NEG_Z,
                Vec3::new(0., p.to_radians().sin(), -p.to_radians().cos()),
            );
        }
        let q = body_to_bevy(Attitude {
            roll: 30.,
            pitch: 20.,
        });
        near(q * Vec3::X, Vec3::new(0.8660254, -0.4698463, -0.1710101));
        near(q * Vec3::NEG_Z, Vec3::new(0., 0.34202015, -0.9396926));
    }
    #[test]
    fn smoothing_is_frame_independent() {
        let target = body_to_bevy(Attitude {
            roll: 30.,
            pitch: 20.,
        });
        let integrate = |hz| {
            let mut q = Quat::IDENTITY;
            for _ in 0..hz {
                q = q.slerp(target, smoothing_alpha(12., 1. / hz as f32));
            }
            q
        };
        assert!(integrate(30).angle_between(integrate(144)) < 0.001);
    }
    #[test]
    fn spikes_and_roll_wrap() {
        let mut s = AttitudeState::default();
        s.ingest(Attitude::default());
        s.ingest(Attitude {
            roll: 170.,
            pitch: 0.,
        });
        assert_eq!(s.accepted.roll, 0.);
        s.ingest(Attitude {
            roll: 171.,
            pitch: 0.,
        });
        s.ingest(Attitude {
            roll: 172.,
            pitch: 0.,
        });
        assert_eq!(s.accepted.roll, 172.);
        s.ingest(Attitude {
            roll: -179.,
            pitch: 0.,
        });
        assert_eq!(s.accepted.roll, -179.);
        s.ingest(Attitude {
            roll: 1000.,
            pitch: 0.,
        });
        assert_eq!(s.raw.roll, 1000.);
        assert_eq!(s.accepted.roll, -179.);
    }
    #[test]
    fn mounting_and_zero() {
        let c = Config {
            port: None,
            smoke_test: false,
            invert_roll: true,
            invert_pitch: false,
            roll_offset: 12.,
            pitch_offset: -5.,
            response: 12.,
        };
        assert_eq!(
            sensor_to_body(
                Attitude {
                    roll: 12.,
                    pitch: 5.
                },
                &c
            ),
            Attitude::default()
        );
    }
}
