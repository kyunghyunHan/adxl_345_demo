use crate::{controls::Controls, physics::Flight, serial::SerialFeed, terrain};
use bevy::prelude::*;
#[derive(Component)]
pub enum Readout {
    Flight,
    Sensor,
    Status,
    Notice,
}
fn font(handle: &Handle<Font>, size: f32) -> TextFont {
    TextFont {
        font: handle.clone().into(),
        font_size: bevy::text::FontSize::Px(size),
        ..default()
    }
}
pub fn setup(mut commands: Commands, server: Res<AssetServer>) {
    let typeface = server.load("fonts/NotoSans-Regular.ttf");
    let white = Color::srgb(0.88, 0.95, 0.96);
    let cyan = Color::srgb(0.30, 0.88, 0.88);
    commands.spawn(Node {width:percent(100),height:percent(100),padding:UiRect::all(px(24)),flex_direction:FlexDirection::Column,justify_content:JustifyContent::SpaceBetween,..default()}).with_children(|root| {
        root.spawn((Node{padding:UiRect::all(px(16)),justify_content:JustifyContent::SpaceBetween,align_items:AlignItems::Center,..default()},BackgroundColor(Color::srgba(0.02,0.06,0.09,0.87)))).with_children(|bar| {
            bar.spawn((Text::new("SKYBOUND  /  ADXL345\nSTM32F439ZI  >  USB UART  >  RUST / BEVY"),font(&typeface,17.),TextColor(white)));
            bar.spawn((Readout::Status,Text::new("KEYBOARD"),font(&typeface,19.),TextColor(cyan)));
        });
        root.spawn(Node{justify_content:JustifyContent::SpaceBetween,align_items:AlignItems::Center,..default()}).with_children(|row| {
            row.spawn((Node{padding:UiRect::all(px(20)),min_width:px(245),..default()},BackgroundColor(Color::srgba(0.02,0.06,0.09,0.8)))).with_children(|p| {
                p.spawn((Readout::Flight,Text::new("FLIGHT"),font(&typeface,21.),TextColor(white)));
            });
            row.spawn((Text::new("-  +  -"),font(&typeface,25.),TextColor(Color::srgba(0.9,1.,0.9,0.75))));
            row.spawn((Node{padding:UiRect::all(px(18)),max_width:px(320),..default()},BackgroundColor(Color::srgba(0.02,0.06,0.09,0.8)))).with_children(|p| {
                p.spawn((Readout::Sensor,Text::new("SENSOR"),font(&typeface,14.),TextColor(cyan)));
            });
        });
        root.spawn((Node{padding:UiRect::all(px(16)),flex_direction:FlexDirection::Column,row_gap:px(9),..default()},BackgroundColor(Color::srgba(0.02,0.06,0.09,0.90)))).with_children(|bar| {
            bar.spawn((Readout::Notice,Text::new(""),font(&typeface,17.),TextColor(Color::srgb(1.,0.78,0.37))));
            bar.spawn((Text::new("W / S  Nose down / up     A / D  Bank     UP / DOWN  Speed     TAB  Input mode\nR  Reset     C  Calibrate (hold still)     I / K  Invert roll / pitch     ESC  Quit"),font(&typeface,14.),TextColor(white)));
        });
    });
}
pub fn update(
    f: Res<Flight>,
    c: Res<Controls>,
    feed: Res<SerialFeed>,
    time: Res<Time>,
    mut labels: Query<(&Readout, &mut Text, &mut TextColor)>,
) {
    for (kind, mut text, mut color) in &mut labels {
        text.0 = match kind {
            Readout::Flight => format!(
                "SIMULATED FLIGHT\n\nROLL       {:>7.1}°\nPITCH      {:>7.1}°\nHEADING    {:>7.1}°\n\nSPEED      {:>5.0} km/h\nALTITUDE   {:>5.0} m\nAGL        {:>5.0} m\nV/S        {:>5.1} m/s\nSET SPEED  {:>5.0} km/h",
                f.bank.to_degrees(),
                f.pitch.to_degrees(),
                f.heading.to_degrees(),
                f.speed * 3.6,
                f.position.y,
                f.position.y - terrain::surface_height(f.position.x, f.position.z),
                f.velocity.y,
                f.command_speed * 3.6
            ),
            Readout::Status => {
                color.0 = if c.sensor_mode && !feed.snapshot.fresh() {
                    Color::srgb(1., 0.63, 0.3)
                } else {
                    Color::srgb(0.3, 0.92, 0.72)
                };
                format!(
                    "{}\nSENSOR: {}",
                    if c.sensor_mode {
                        "SENSOR CONTROL"
                    } else {
                        "KEYBOARD CONTROL"
                    },
                    feed.snapshot.status()
                )
            }
            Readout::Sensor => format!(
                "SENSOR / MEASURED\nRaw roll      {:>7.1}°\nRaw pitch     {:>7.1}°\nZero          {:>5.1} / {:.1}°\n\nCONTROL / TARGET\nBank          {:>7.1}°\nPitch         {:>7.1}°\nInvert R / P  {} / {}\nDead zone     {:.1}°\nGain          {:.1}x\n\n{}\nPackets {} / bad {}\n{}\nFrame {:.1} ms",
                c.raw.roll,
                c.raw.pitch,
                c.zero.roll,
                c.zero.pitch,
                c.target.roll,
                c.target.pitch,
                c.invert_roll,
                c.invert_pitch,
                c.dead_zone,
                c.gain,
                if c.calibrating {
                    format!("ZERO: hold still ({}/25)", c.calibration_count)
                } else {
                    "ZERO: calibrated".into()
                },
                feed.snapshot.sequence,
                feed.snapshot.invalid,
                if feed.snapshot.port.is_empty() {
                    "USB auto-detect"
                } else {
                    &feed.snapshot.port
                },
                time.delta_secs() * 1000.
            ),
            Readout::Notice => {
                if f.crashed {
                    "GROUND IMPACT  /  Press R to start a new flight".into()
                } else if c.sensor_mode && !feed.snapshot.fresh() {
                    format!("SAFE LEVEL INPUT  /  {}", feed.snapshot.error)
                } else if c.sensor_mode && c.calibrating {
                    "CALIBRATION  /  Hold the sensor level and still for 25 samples".into()
                } else {
                    format!(
                        "FREE FLIGHT  /  16 km landscape  /  Resets: {}  /  Heading is simulated from bank",
                        f.resets
                    )
                }
            }
        };
    }
}
