use crate::{
    physics::Flight,
    serial::{Attitude, SerialFeed},
};
use bevy::prelude::*;

#[derive(Resource)]
pub struct Controls {
    pub sensor_mode: bool,
    pub invert_roll: bool,
    pub invert_pitch: bool,
    pub dead_zone: f32,
    pub gain: f32,
    pub filter_hz: f32,
    pub raw: Attitude,
    pub zero: Attitude,
    pub filtered: Attitude,
    pub target: Attitude,
    pub calibrating: bool,
    pub calibration_count: u32,
    sum: Attitude,
    last_sequence: u64,
    manual_calibration: bool,
    pub reset: bool,
}
impl Default for Controls {
    fn default() -> Self {
        Self {
            sensor_mode: false,
            invert_roll: false,
            invert_pitch: false,
            dead_zone: 2.,
            gain: 1.5,
            filter_hz: 3.,
            raw: Attitude::default(),
            zero: Attitude::default(),
            filtered: Attitude::default(),
            target: Attitude::default(),
            calibrating: true,
            calibration_count: 0,
            sum: Attitude::default(),
            last_sequence: 0,
            manual_calibration: false,
            reset: false,
        }
    }
}
pub fn dead_zone(value: f32, zone: f32) -> f32 {
    value.signum() * (value.abs() - zone).max(0.)
}
fn roll_difference(value: f32, zero: f32) -> f32 {
    (value - zero + 180.).rem_euclid(360.) - 180.
}
impl Controls {
    fn sample(&mut self, raw: Attitude) {
        self.raw = raw;
        if self.calibrating {
            // Startup must not wait indefinitely for a motionless board. Use the
            // first valid packet as zero; C explicitly requests a stable average.
            if !self.manual_calibration {
                self.zero = raw;
                self.calibrating = false;
                self.filtered = Attitude::default();
                return;
            }
            // Unwrap around the previous zero before averaging across +/-180°.
            let unwrapped_roll = self.zero.roll + roll_difference(raw.roll, self.zero.roll);
            // Restart the average if the board moves by more than three degrees.
            if self.calibration_count > 0 {
                let mean = Attitude {
                    roll: self.sum.roll / self.calibration_count as f32,
                    pitch: self.sum.pitch / self.calibration_count as f32,
                };
                if roll_difference(raw.roll, mean.roll).abs() > 3.
                    || (raw.pitch - mean.pitch).abs() > 3.
                {
                    self.calibration_count = 0;
                    self.sum = Attitude::default();
                }
            }
            self.sum.roll += unwrapped_roll;
            self.sum.pitch += raw.pitch;
            self.calibration_count += 1;
            if self.calibration_count >= 25 {
                self.zero = Attitude {
                    roll: roll_difference(self.sum.roll / 25., 0.),
                    pitch: self.sum.pitch / 25.,
                };
                self.calibrating = false;
                self.filtered = Attitude::default();
            }
        }
    }
    pub fn filter(&mut self, dt: f32, fresh: bool) {
        let mapped = if fresh && !self.calibrating {
            Attitude {
                roll: dead_zone(
                    roll_difference(self.raw.roll, self.zero.roll)
                        * if self.invert_roll { -1. } else { 1. },
                    self.dead_zone,
                ) * self.gain,
                pitch: dead_zone(
                    (self.raw.pitch - self.zero.pitch) * if self.invert_pitch { -1. } else { 1. },
                    self.dead_zone,
                ) * self.gain,
            }
        } else {
            Attitude::default()
        };
        let alpha = 1. - (-std::f32::consts::TAU * self.filter_hz * dt).exp();
        self.filtered.roll += (mapped.roll.clamp(-55., 55.) - self.filtered.roll) * alpha;
        self.filtered.pitch += (mapped.pitch.clamp(-25., 25.) - self.filtered.pitch) * alpha;
        self.target = self.filtered;
    }
}
pub fn input(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    feed: Res<SerialFeed>,
    mut c: ResMut<Controls>,
    mut flight: ResMut<Flight>,
    mut commands: Commands,
) {
    let dt = time.delta_secs();
    if keys.just_pressed(KeyCode::Escape) {
        commands.write_message(bevy::app::AppExit::Success);
    }
    if keys.just_pressed(KeyCode::Tab) {
        c.sensor_mode = !c.sensor_mode;
        c.filtered = Attitude::default();
    }
    if keys.just_pressed(KeyCode::KeyI) {
        c.invert_roll = !c.invert_roll;
    }
    if keys.just_pressed(KeyCode::KeyK) {
        c.invert_pitch = !c.invert_pitch;
    }
    if keys.just_pressed(KeyCode::KeyC) {
        c.calibrating = true;
        c.manual_calibration = true;
        c.calibration_count = 0;
        c.sum = Attitude::default();
        c.filtered = Attitude::default();
    }
    if feed.snapshot.sequence != c.last_sequence {
        c.last_sequence = feed.snapshot.sequence;
        if feed.snapshot.fresh() {
            c.sample(feed.snapshot.raw);
        }
    }
    if !feed.snapshot.fresh() && c.calibrating {
        c.calibration_count = 0;
        c.sum = Attitude::default();
    }
    // No stale telemetry can command bank/pitch. Physics gently levels the plane.
    if c.sensor_mode {
        c.filter(dt, feed.snapshot.fresh());
    } else {
        let axis = |positive, negative| {
            (keys.pressed(positive) as i32 - keys.pressed(negative) as i32) as f32
        };
        c.target = Attitude {
            roll: axis(KeyCode::KeyD, KeyCode::KeyA) * 45.,
            pitch: axis(KeyCode::KeyS, KeyCode::KeyW) * 20.,
        };
    }
    let throttle =
        (keys.pressed(KeyCode::ArrowUp) as i32 - keys.pressed(KeyCode::ArrowDown) as i32) as f32;
    flight.command_speed = (flight.command_speed + throttle * 12. * dt).clamp(25., 100.);
    if keys.just_pressed(KeyCode::KeyR) {
        c.reset = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn calibration_limits_inversion_and_disconnect() {
        let mut c = Controls::default();
        for _ in 0..25 {
            c.sample(Attitude {
                roll: 10.,
                pitch: -5.,
            });
        }
        assert!(!c.calibrating);
        assert_eq!(c.zero.roll, 10.);
        c.sample(Attitude {
            roll: 180.,
            pitch: -90.,
        });
        c.invert_roll = true;
        c.filter(1., true);
        assert!((c.target.roll + 55.).abs() < 0.01);
        assert!((c.target.pitch + 25.).abs() < 0.01);
        c.filter(1., false);
        assert!(c.target.roll.abs() < 0.01);
        assert!(c.target.pitch.abs() < 0.01);
        assert_eq!(dead_zone(1.9, 2.), 0.);
        assert_eq!(dead_zone(-5., 2.), -3.);
    }
    #[test]
    fn filter_is_independent_of_frame_rate() {
        let run = |hz| {
            let mut c = Controls {
                calibrating: false,
                ..Default::default()
            };
            c.raw.roll = 20.;
            for _ in 0..hz {
                c.filter(1. / hz as f32, true);
            }
            c.filtered.roll
        };
        assert!((run(30) - run(120)).abs() < 0.001);
    }

    #[test]
    fn startup_does_not_block_tilting_and_manual_zero_averages() {
        let mut c = Controls::default();
        c.sample(Attitude {
            roll: 5.,
            pitch: 2.,
        });
        assert!(!c.calibrating);
        c.sample(Attitude {
            roll: 25.,
            pitch: 12.,
        });
        c.filter(0.25, true);
        assert!(c.target.roll > 20. && c.target.pitch > 10.);
        c.manual_calibration = true;
        c.calibrating = true;
        for _ in 0..24 {
            c.sample(Attitude {
                roll: 12.,
                pitch: 3.,
            });
        }
        assert!(c.calibrating);
        c.sample(Attitude {
            roll: 12.,
            pitch: 3.,
        });
        assert!(!c.calibrating);
        assert_eq!(
            c.zero,
            Attitude {
                roll: 12.,
                pitch: 3.
            }
        );
    }

    #[test]
    fn roll_crossing_180_does_not_reverse_the_controls() {
        let mut c = Controls::default();
        c.sample(Attitude {
            roll: 179.,
            pitch: 0.,
        });
        c.sample(Attitude {
            roll: -175.,
            pitch: 0.,
        });
        c.filter(1., true);
        assert!((c.target.roll - 6.).abs() < 0.01);
        c.manual_calibration = true;
        c.calibrating = true;
        for i in 0..25 {
            c.sample(Attitude {
                roll: if i % 2 == 0 { 179.5 } else { -179.5 },
                pitch: 0.,
            });
        }
        assert!(!c.calibrating);
        assert!(c.zero.roll.abs() > 179.);
    }

    #[test]
    fn sensor_pipeline_turns_climbs_and_levels_after_loss() {
        let mut c = Controls::default();
        for _ in 0..25 {
            c.sample(Attitude {
                roll: 7.,
                pitch: -4.,
            });
        }
        let mut f = Flight::default();
        c.sample(Attitude {
            roll: 27.,
            pitch: 6.,
        });
        for _ in 0..600 {
            c.filter(1. / 120., true);
            f.step(1. / 120., c.target);
        }
        assert!(f.heading > 0.1 && f.position.x > 20. && f.position.y > 680.);
        for _ in 0..360 {
            c.filter(1. / 120., false);
            f.step(1. / 120., c.target);
        }
        assert!(f.bank.abs() < 0.001 && f.pitch.abs() < 0.001);
        assert!(!f.crashed);
    }
}
