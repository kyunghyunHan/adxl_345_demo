//! Explicit, repeatable GPU validation, invoked only by --demo --smoke-test.
use crate::{
    aircraft::{
        ModelStatus,
        attitude::{Attitude, AttitudeState},
    },
    camera::View,
    config::Config,
};
use bevy::{
    app::AppExit,
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
    window::PrimaryWindow,
};
use std::time::Instant;

#[derive(Resource, Default)]
pub struct Playback {
    loading_since: Option<Instant>,
    started: Option<Instant>,
    captured: usize,
}
const CASES: [(f32, f32, usize); 10] = [
    (0., 0., 0),
    (30., 0., 0),
    (-30., 0., 0),
    (0., 20., 0),
    (0., -20., 0),
    (30., 20., 2),
    (0., 0., 1),
    (0., 20., 3),
    (-30., -20., 4),
    (30., 20., 0),
];
pub fn playback(
    config: Res<Config>,
    mut state: ResMut<AttitudeState>,
    mut view: ResMut<View>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut playback: ResMut<Playback>,
    model: Res<ModelStatus>,
    server: Res<AssetServer>,
) {
    if !config.smoke_test {
        return;
    }
    let loading = playback.loading_since.get_or_insert_with(Instant::now);
    if !server.is_loaded_with_dependencies(model.scene.id()) {
        assert!(
            loading.elapsed().as_secs() < 45,
            "GLB failed to load: {}",
            model.message
        );
        return;
    }
    if playback.started.is_none() {
        std::fs::create_dir_all("target/validation").expect("create screenshot directory");
        playback.started = Some(Instant::now());
    }
    let phase = playback.captured;
    if let Some(&(roll, pitch, mode)) = CASES.get(phase) {
        state.ingest(Attitude { roll, pitch });
        view.mode = mode;
        if phase == 9 {
            if let Ok(mut w) = windows.single_mut() {
                w.resolution.set(1000., 700.);
            }
        }
    }
}
pub fn capture(
    mut commands: Commands,
    config: Res<Config>,
    state: Res<AttitudeState>,
    mut playback: ResMut<Playback>,
) {
    if !config.smoke_test {
        return;
    }
    let Some(start) = playback.started else {
        return;
    };
    let elapsed = start.elapsed().as_secs_f32();
    let phase = playback.captured;
    if phase >= CASES.len() {
        if elapsed > 3. {
            assert_eq!(playback.captured, CASES.len(), "Missing GPU captures");
            info!("GPU SMOKE PASS: GLB loaded; 10 attitude/view/resize captures written");
            commands.write_message(AppExit::Success);
        }
        return;
    }
    if elapsed > 2. {
        let (roll, pitch, mode) = CASES[phase];
        let shown = state.displayed_angles();
        // Shader compilation and GPU readback can stall initial frames. Bevy
        // clamps simulation delta, so wall time alone does not imply settling.
        let converged = (shown.roll - roll).abs() < 0.5 && (shown.pitch - pitch).abs() < 0.5;
        if !converged {
            assert!(
                elapsed < 45.,
                "Displayed attitude did not converge: {shown:?}"
            );
            return;
        }
        let path = format!("target/validation/demo-{phase:02}.png");
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
        info!("GPU case {phase}: roll={roll} pitch={pitch} view={mode}");
        playback.captured += 1;
        playback.started = Some(Instant::now());
    }
}
