use crate::{
    aircraft::attitude::{Attitude, AttitudeState, wrap},
    config::Config,
};
use bevy::prelude::*;

pub fn input(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    config: Res<Config>,
    mut state: ResMut<AttitudeState>,
) {
    if !config.demo() || config.smoke_test {
        return;
    }
    let mut a = state.raw;
    let step = 35. * time.delta_secs();
    if keys.pressed(KeyCode::ArrowLeft) {
        a.roll -= step;
    }
    if keys.pressed(KeyCode::ArrowRight) {
        a.roll += step;
    }
    if keys.pressed(KeyCode::ArrowUp) {
        a.pitch += step;
    }
    if keys.pressed(KeyCode::ArrowDown) {
        a.pitch -= step;
    }
    a.roll = wrap(a.roll);
    a.pitch = a.pitch.clamp(-85., 85.);
    if keys.just_pressed(KeyCode::Space) {
        a = Attitude::default();
    }
    state.ingest(a);
}
