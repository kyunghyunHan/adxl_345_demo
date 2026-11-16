//! SI units, +X east, +Y altitude, -Z north; heading clockwise from north.
use crate::{controls::Controls, serial::Attitude, terrain};
use bevy::prelude::*;
#[derive(Resource)]
pub struct Flight {
    pub position: Vec3,
    pub orientation: Quat,
    pub velocity: Vec3,
    pub speed: f32,
    pub command_speed: f32,
    pub heading: f32,
    pub bank: f32,
    pub pitch: f32,
    pub crashed: bool,
    pub resets: u32,
}
impl Default for Flight {
    fn default() -> Self {
        Self {
            position: Vec3::new(0., 650., 1800.),
            orientation: Quat::IDENTITY,
            velocity: Vec3::NEG_Z * 50.,
            speed: 50.,
            command_speed: 50.,
            heading: 0.,
            bank: 0.,
            pitch: 0.,
            crashed: false,
            resets: 0,
        }
    }
}
impl Flight {
    pub fn step(&mut self, dt: f32, target: Attitude) {
        if self.crashed {
            return;
        }
        let desired = Quat::from_rotation_y(-self.heading)
            * Quat::from_rotation_x(target.pitch.clamp(-25., 25.).to_radians())
            * Quat::from_rotation_z(-target.roll.clamp(-55., 55.).to_radians());
        self.orientation = self
            .orientation
            .slerp(desired, 1. - (-3.5 * dt).exp())
            .normalize();
        let (_, pitch, roll) = self.orientation.to_euler(EulerRot::YXZ);
        self.bank = -roll;
        self.pitch = pitch;
        // Coordinated turn: yaw is derived from bank, never supplied by ADXL345.
        self.heading = (self.heading + 9.81 * self.bank.tan() / self.speed.max(20.) * dt)
            .rem_euclid(std::f32::consts::TAU);
        let forward = self.orientation * Vec3::NEG_Z;
        let acceleration = (self.command_speed - self.speed) * 0.6 - 9.81 * forward.y;
        self.speed = (self.speed + acceleration * dt).clamp(20., 110.);
        // Lift scales with v², bank reduces its vertical component. Below cruise speed
        // the aircraft sinks even at zero pitch; elevator controls flight-path target.
        let lift = 9.81 * (self.speed / 50.).powi(2).min(1.6) * self.bank.cos();
        self.velocity.y += ((forward.y * self.speed - self.velocity.y) * 1.8 + lift - 9.81) * dt;
        self.velocity.x = forward.x * self.speed;
        self.velocity.z = forward.z * self.speed;
        self.position += self.velocity * dt;
        let floor = terrain::surface_height(self.position.x, self.position.z) + 2.5;
        if self.position.y <= floor {
            self.position.y = floor;
            self.crashed = true;
            self.velocity = Vec3::ZERO;
            self.speed = 0.;
        }
        // The mesh has finite coverage; a visible HUD message explains the automatic reset.
        if self.position.x.abs() > 7600. || self.position.z.abs() > 7600. || self.position.y > 4500.
        {
            let resets = self.resets + 1;
            *self = Self::default();
            self.resets = resets;
        }
    }
}
pub fn update(time: Res<Time<Fixed>>, mut c: ResMut<Controls>, mut f: ResMut<Flight>) {
    if c.reset {
        let resets = f.resets + 1;
        *f = Flight::default();
        f.resets = resets;
        c.reset = false;
    }
    f.step(time.delta_secs(), c.target);
}
#[cfg(test)]
mod tests {
    use super::*;
    fn run(target: Attitude, hz: u32) -> Flight {
        let mut f = Flight::default();
        for _ in 0..hz * 5 {
            f.step(1. / hz as f32, target);
        }
        f
    }
    #[test]
    fn level_cruise_and_banked_turn() {
        let f = run(Attitude::default(), 120);
        assert!((f.position.y - 650.).abs() < 0.1);
        assert!(f.position.z < 1600.);
        let right = run(
            Attitude {
                roll: 30.,
                pitch: 0.,
            },
            120,
        );
        let left = run(
            Attitude {
                roll: -30.,
                pitch: 0.,
            },
            120,
        );
        assert!(right.position.x > 20. && left.position.x < -20.);
        assert!(right.heading > 0.1 && left.heading > 5.);
    }
    #[test]
    fn climb_descent_collision_and_frame_rate() {
        let target = Attitude {
            roll: 15.,
            pitch: 12.,
        };
        let a = run(target, 60);
        let b = run(target, 120);
        assert!(a.position.y > 680.);
        assert!(a.position.distance(b.position) < 2.);
        assert!(
            run(
                Attitude {
                    roll: 0.,
                    pitch: -15.
                },
                120
            )
            .position
            .y < 620.
        );
        let mut f = Flight {
            position: Vec3::new(0., -10., 0.),
            ..Default::default()
        };
        f.step(1. / 120., Attitude::default());
        assert!(f.crashed);
        let pos = f.position;
        f.step(1., target);
        assert_eq!(f.position, pos);
        let mut slow = Flight {
            speed: 25.,
            command_speed: 25.,
            ..Default::default()
        };
        for _ in 0..120 {
            slow.step(1. / 120., Attitude::default());
        }
        assert!(slow.position.y < 649.);
    }
}
