//! Opt-in GPU playback. Does not claim to test physical keys or real STM32 hardware.
use crate::{controls::Controls, physics::Flight, serial::Attitude};
use bevy::{
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};
#[derive(Resource)]
pub struct Smoke {
    enabled: bool,
    phase: usize,
    elapsed: f32,
    frames: u32,
}
impl Smoke {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            phase: 0,
            elapsed: 0.,
            frames: 0,
        }
    }
}
pub fn drive(s: Res<Smoke>, mut c: ResMut<Controls>) {
    if !s.enabled {
        return;
    }
    c.target = match s.phase {
        1 => Attitude {
            roll: 30.,
            pitch: 0.,
        },
        2 => Attitude {
            roll: -25.,
            pitch: 18.,
        },
        _ => Attitude::default(),
    };
}
pub fn capture(time: Res<Time>, mut s: ResMut<Smoke>, f: Res<Flight>, mut commands: Commands) {
    if !s.enabled {
        return;
    }
    s.elapsed += time.delta_secs();
    s.frames += 1;
    assert!(
        f.position.is_finite() && f.orientation.is_finite(),
        "Non-finite flight state"
    );
    if s.elapsed < 4. || s.frames < 90 {
        return;
    }
    if s.phase < 4 {
        assert!(!f.crashed, "Unexpected ground impact in smoke flight");
        match s.phase {
            0 => assert!(f.position.z < 1750.),
            1 => assert!(f.bank.to_degrees() > 20.),
            2 => assert!(f.pitch.to_degrees() > 12.),
            _ => {}
        }
        std::fs::create_dir_all("validation").expect("create screenshot directory");
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(format!("validation/flight-{}.png", s.phase)));
        info!(
            "GPU smoke phase {}: position {:?}, bank {:.1}, pitch {:.1}",
            s.phase,
            f.position,
            f.bank.to_degrees(),
            f.pitch.to_degrees()
        );
        s.phase += 1;
        s.elapsed = 0.;
        s.frames = 0;
    } else {
        info!("GPU SMOKE PASS: forward flight, bank, climb, recovery; four screenshots");
        commands.write_message(bevy::app::AppExit::Success);
    }
}
