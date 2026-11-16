pub mod horizon;
use crate::{
    aircraft::{
        ModelStatus,
        attitude::{AttitudeState, sensor_to_body},
    },
    camera::View,
    config::Config,
    serial::SerialFeed,
};
use bevy::{prelude::*, window::PrimaryWindow};
use std::time::Instant;

pub const MUTED: Color = Color::srgb(0.48, 0.62, 0.71);
const CYAN: Color = Color::srgb(0.30, 0.85, 0.96);
const PANEL: Color = Color::srgba(0.025, 0.055, 0.085, 0.96);
#[derive(Component, Clone, Copy)]
pub enum Readout {
    Angles,
    Raw,
    Status,
    Telemetry,
    Calibration,
    Footer,
    Camera,
    Model,
}
pub fn font(size: f32) -> TextFont {
    TextFont {
        font_size: bevy::text::FontSize::Px(size),
        ..default()
    }
}
fn label(p: &mut ChildSpawnerCommands, text: &str, size: f32, color: Color) {
    p.spawn((Text::new(text), font(size), TextColor(color)));
}
fn dynamic(p: &mut ChildSpawnerCommands, kind: Readout, size: f32, color: Color) {
    p.spawn((kind, Text::new("—"), font(size), TextColor(color)));
}
fn panel() -> Node {
    Node {
        width: px(260),
        padding: UiRect::all(px(16)),
        flex_direction: FlexDirection::Column,
        row_gap: px(14),
        border: UiRect::all(px(1)),
        border_radius: BorderRadius::all(px(8)),
        ..default()
    }
}
#[derive(Resource)]
pub struct UiFont(Handle<Font>);

pub fn apply_font(font: Res<UiFont>, mut texts: Query<&mut TextFont, Added<TextFont>>) {
    for mut text in &mut texts {
        text.font = font.0.clone().into();
    }
}

pub fn setup(mut commands: Commands, server: Res<AssetServer>) {
    commands.insert_resource(UiFont(server.load("fonts/NotoSans-Regular.ttf")));
    commands.spawn(Node {width:percent(100),height:percent(100),flex_direction:FlexDirection::Column,justify_content:JustifyContent::SpaceBetween,padding:UiRect::all(px(20)),..default()}).with_children(|root| {
        root.spawn((Node {width:percent(100),justify_content:JustifyContent::SpaceBetween,align_items:AlignItems::Center,padding:UiRect::bottom(px(14)),..default()},BorderColor::all(MUTED))).with_children(|bar| {
            bar.spawn(Node {flex_direction:FlexDirection::Column,row_gap:px(4),..default()}).with_children(|p| {
                label(p,"AEROSPACE  /  ATTITUDE LAB",22.,Color::WHITE);
                label(p,"STM32  ×  ADXL345     /     REAL-TIME TELEMETRY",11.,MUTED);
            });
            dynamic(bar,Readout::Status,16.,CYAN);
        });
        root.spawn(Node {width:percent(100),flex_grow:1.,justify_content:JustifyContent::SpaceBetween,align_items:AlignItems::Center,..default()}).with_children(|row| {
            row.spawn((panel(),BackgroundColor(PANEL),BorderColor::all(Color::srgb(0.12,0.24,0.31)))).with_children(|left| {
                label(left,"01  /  ATTITUDE DATA",13.,CYAN);
                label(left,"DISPLAYED / FILTERED",10.,MUTED);
                dynamic(left,Readout::Angles,30.,Color::WHITE);
                dynamic(left,Readout::Raw,12.,MUTED);
                dynamic(left,Readout::Telemetry,13.,Color::srgb(0.75,0.84,0.88));
                label(left,"MOUNTING & ZERO",11.,CYAN);
                dynamic(left,Readout::Calibration,12.,MUTED);
                label(left,"I / K   Invert roll / pitch\nZ   Zero here    C   Clear zero\n[ / ]   Roll offset\n- / =   Pitch offset\nShift   Fine adjustment",11.,MUTED);
            });
            row.spawn((panel(),BackgroundColor(PANEL),BorderColor::all(Color::srgb(0.12,0.24,0.31)))).with_children(|right| {
                label(right,"02  /  ARTIFICIAL HORIZON",13.,CYAN);
                horizon::spawn(right);
                label(right,"VIEW CONTROL",11.,CYAN);
                dynamic(right,Readout::Camera,14.,Color::WHITE);
                label(right,"1  Perspective    2  Top\n3  Front             4  Side\n5  Free orbit       Tab  Cycle\nOrbit: right drag / wheel\nF11  Fullscreen    Esc  Window",11.,MUTED);
                label(right,"DEMO CONTROLS",11.,CYAN);
                label(right,"Left / Right   Bank\nUp / Down   Pitch\nSpace   Return to level",11.,MUTED);
            });
        });
        root.spawn((Node {width:percent(100),padding:UiRect::all(px(12)),flex_direction:FlexDirection::Column,row_gap:px(5),border_radius:BorderRadius::all(px(6)),..default()},BackgroundColor(PANEL))).with_children(|bottom| {
            dynamic(bottom,Readout::Model,11.,MUTED);
            dynamic(bottom,Readout::Footer,12.,Color::srgb(0.75,0.84,0.88));
            label(bottom,"ACCELEROMETER GRAVITY ESTIMATE   /   X RIGHT · Y UP · -Z NOSE   /   REFERENCE USE ONLY",10.,MUTED);
        });
    });
}
pub fn update(
    config: Res<Config>,
    feed: Res<SerialFeed>,
    state: Res<AttitudeState>,
    view: Res<View>,
    model: Res<ModelStatus>,
    time: Res<Time>,
    mut labels: Query<(&Readout, &mut Text, &mut TextColor)>,
) {
    let now = Instant::now();
    let status = if config.demo() {
        "DEMO / SYNTHETIC INPUT"
    } else {
        feed.snapshot.status(now)
    };
    let color = if config.demo() {
        Color::srgb(0.30, 0.65, 1.)
    } else {
        match status {
            "CONNECTED" => Color::srgb(0.3, 0.92, 0.6),
            "DISCONNECTED" => Color::srgb(1., 0.32, 0.35),
            _ => Color::srgb(1., 0.78, 0.25),
        }
    };
    let age = feed
        .snapshot
        .last_received
        .map(|t| now.saturating_duration_since(t).as_secs_f32());
    let rate = if age.is_some_and(|v| v < 2.) && feed.snapshot.open {
        feed.snapshot.rate
    } else {
        0.
    };
    let displayed = state.displayed_angles();
    let target = sensor_to_body(state.accepted, &config);
    for (kind, mut text, mut text_color) in &mut labels {
        **text = match kind {
            Readout::Angles => format!(
                "ROLL   {:+.1}°\nPITCH {:+.1}°",
                displayed.roll, displayed.pitch
            ),
            Readout::Raw => format!(
                "RAW    R {:+.1}°   P {:+.1}°\nTARGET R {:+.1}°   P {:+.1}°",
                state.raw.roll, state.raw.pitch, target.roll, target.pitch
            ),
            Readout::Status => {
                text_color.0 = color;
                status.into()
            }
            Readout::Telemetry => format!(
                "UART   {}\nBAUD   115200 / 8N1\nRATE   {}\nLAST   {}\nSENSOR {}\nINVALID {}   HELD {}",
                if config.demo() { "OFF / DEMO" } else { status },
                if config.demo() {
                    "DEMO".into()
                } else {
                    format!("{rate:.0} Hz")
                },
                if config.demo() {
                    "SYNTHETIC".into()
                } else {
                    age.map_or("NEVER".into(), |v| format!("{v:.1} s ago"))
                },
                if config.demo() {
                    "SIMULATED"
                } else if status == "CONNECTED" {
                    "GRAVITY ESTIMATE"
                } else {
                    "NO FRESH DATA"
                },
                feed.snapshot.invalid,
                state.rejected
            ),
            Readout::Calibration => format!(
                "Invert R {}   P {}\nOffset R {:+.1}°   P {:+.1}°\nResponse {:.1} / s",
                config.invert_roll,
                config.invert_pitch,
                config.roll_offset,
                config.pitch_offset,
                config.response
            ),
            Readout::Camera => format!("{} / {}", view.mode + 1, view.name()),
            Readout::Model => model.message.clone(),
            Readout::Footer => format!(
                "PORT {}  |  {}  |  RAW {:+.1}° / {:+.1}°  |  {:.0} FPS{}",
                config.port.as_deref().unwrap_or("— DEMO —"),
                status,
                state.raw.roll,
                state.raw.pitch,
                1. / time.delta_secs().max(0.001),
                if feed.snapshot.error.is_empty() {
                    String::new()
                } else {
                    format!("\nERROR {} / retry every 1s", feed.snapshot.error)
                }
            ),
        };
    }
}
pub fn resize(window: Query<&Window, With<PrimaryWindow>>, mut scale: ResMut<UiScale>) {
    if let Ok(w) = window.single() {
        scale.0 = (w.width() / 1280.).min(w.height() / 800.).clamp(0.35, 1.25);
    }
}
