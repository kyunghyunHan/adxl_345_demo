mod aircraft;
mod camera;
mod config;
mod demo;
mod serial;
mod ui;
mod validation;

use bevy::prelude::*;
use config::Config;

fn main() {
    let config=Config::from_args().unwrap_or_else(|error| {
        eprintln!("{error}\nUsage: cargo run -- /dev/cu.usbmodemXXXX [options]\n       cargo run -- --demo\nOptions: --invert-roll --invert-pitch --roll-offset N --pitch-offset N --response N");
        std::process::exit(if error.is_empty(){0}else{2});
    });
    let feed = serial::SerialFeed::start(config.port.clone());
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "ATTITUDE LAB | STM32 · ADXL345".into(),
                resolution: (1440, 900).into(),
                resize_constraints: bevy::window::WindowResizeConstraints {
                    min_width: 800.,
                    min_height: 560.,
                    ..default()
                },
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.018, 0.033, 0.055)))
        .insert_resource(config)
        .insert_resource(feed)
        .init_resource::<aircraft::attitude::AttitudeState>()
        .init_resource::<camera::View>()
        .init_resource::<validation::Playback>()
        .add_systems(Startup, (aircraft::setup, camera::setup, ui::setup))
        .add_systems(
            Update,
            (
                serial::receive,
                demo::input,
                validation::playback,
                aircraft::attitude::calibration,
                aircraft::attitude::process,
                aircraft::rotate,
                camera::control,
                aircraft::loading,
                ui::apply_font,
                ui::horizon::update,
                ui::update,
                ui::resize,
                aircraft::guides,
                validation::capture,
            )
                .chain(),
        )
        .run();
}
