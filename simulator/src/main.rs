mod aircraft;
mod camera;
mod controls;
mod hud;
mod physics;
mod serial;
mod terrain;
mod validation;

use bevy::prelude::*;
#[derive(Default)]
struct Options {
    port: Option<String>,
    sensor: bool,
    no_serial: bool,
    smoke: bool,
    invert_roll: bool,
    invert_pitch: bool,
    diagnose: bool,
}
fn options() -> Result<Options, String> {
    let mut o = Options::default();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--port" => o.port = Some(args.next().ok_or("--port requires a device path")?),
            "--sensor" => o.sensor = true,
            "--keyboard" => o.sensor = false,
            "--no-serial" => o.no_serial = true,
            "--smoke-test" => o.smoke = true,
            "--invert-roll" => o.invert_roll = true,
            "--invert-pitch" => o.invert_pitch = true,
            "--diagnose-serial" => o.diagnose = true,
            "--list-ports" => {
                for p in serial::ports().map_err(|e| e.to_string())? {
                    println!("{}  {:?}", p.port_name, p.port_type);
                }
                std::process::exit(0);
            }
            "--help" | "-h" => {
                println!(
                    "ADXL345 Flight Simulator\n--sensor | --keyboard (default)\n--port /dev/cu.usbmodemXXXX\n--list-ports  --no-serial\n--diagnose-serial (6 seconds, no window)\n--invert-roll  --invert-pitch\n--smoke-test (keyboard, no UART)\n\nW/S pitch, A/D roll, Up/Down speed, Tab input mode, R reset, C zero, I/K invert, Esc quit"
                );
                std::process::exit(0);
            }
            _ => return Err(format!("Unknown option: {arg}; use --help")),
        }
    }
    if o.sensor && o.no_serial {
        return Err("--sensor needs UART; remove --no-serial".into());
    }
    if o.smoke {
        o.sensor = false;
        o.no_serial = true;
    }
    Ok(o)
}
fn main() {
    let o = options().unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2)
    });
    if o.diagnose {
        let feed = serial::SerialFeed::start(o.port, true);
        serial::diagnose(&feed);
        return;
    }
    // An absolute asset root makes direct binary launches independent of the current directory.
    let asset_path = std::env::var("FLIGHT_ASSET_ROOT")
        .unwrap_or_else(|_| format!("{}/assets", env!("CARGO_MANIFEST_DIR")));
    let mut controls = controls::Controls::default();
    controls.sensor_mode = o.sensor;
    controls.invert_roll = o.invert_roll;
    controls.invert_pitch = o.invert_pitch;
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: asset_path,
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "SKYBOUND | ADXL345 Flight Simulator".into(),
                        resolution: (1440, 900).into(),
                        present_mode: bevy::window::PresentMode::AutoVsync,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .insert_resource(ClearColor(Color::srgb(0.42, 0.69, 0.87)))
        .insert_resource(Time::<Fixed>::from_hz(120.))
        .insert_resource(serial::SerialFeed::start(o.port, !o.no_serial))
        .insert_resource(controls)
        .init_resource::<physics::Flight>()
        .insert_resource(validation::Smoke::new(o.smoke))
        .add_systems(
            Startup,
            (terrain::setup, aircraft::setup, camera::setup, hud::setup),
        )
        .add_systems(FixedUpdate, physics::update)
        .add_systems(
            Update,
            (
                serial::receive,
                controls::input,
                validation::drive,
                aircraft::animate,
                camera::follow,
                hud::update,
                validation::capture,
            )
                .chain(),
        )
        .run();
}
