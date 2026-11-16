use bevy::{
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll},
    prelude::*,
    window::{MonitorSelection, PrimaryWindow, WindowMode},
};

#[derive(Resource)]
pub struct View {
    pub mode: usize,
    pub azimuth: f32,
    pub elevation: f32,
    pub distance: f32,
}
impl Default for View {
    fn default() -> Self {
        Self {
            mode: 0,
            azimuth: -0.65,
            elevation: 0.42,
            distance: 15.5,
        }
    }
}
impl View {
    pub fn name(&self) -> &'static str {
        ["PERSPECTIVE", "TOP", "FRONT", "SIDE", "FREE ORBIT"][self.mode]
    }
}
#[derive(Component)]
pub struct AircraftCamera;
pub fn setup(mut commands: Commands) {
    commands.spawn((
        AircraftCamera,
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: 45_f32.to_radians(),
            near: 0.1,
            far: 300.,
            ..default()
        }),
        Transform::from_xyz(-8., 6., -12.).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
pub fn control(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    mut view: ResMut<View>,
    mut cameras: Query<(&mut Transform, &mut Projection), With<AircraftCamera>>,
    mut window: Query<&mut Window, With<PrimaryWindow>>,
) {
    for (i, key) in [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
    ]
    .iter()
    .enumerate()
    {
        if keys.just_pressed(*key) {
            view.mode = i;
        }
    }
    if keys.just_pressed(KeyCode::Tab) {
        view.mode = (view.mode + 1) % 5;
    }
    if view.mode == 4 {
        if buttons.pressed(MouseButton::Right) {
            view.azimuth -= motion.delta.x * 0.006;
            view.elevation = (view.elevation + motion.delta.y * 0.006).clamp(-1.45, 1.45);
        }
        view.distance = (view.distance * (-scroll.delta.y * 0.08).exp()).clamp(12., 40.);
    }
    if let Ok(mut w) = window.single_mut() {
        if keys.just_pressed(KeyCode::F11) {
            w.mode = if matches!(w.mode, WindowMode::Windowed) {
                WindowMode::BorderlessFullscreen(MonitorSelection::Current)
            } else {
                WindowMode::Windowed
            };
        }
        if keys.just_pressed(KeyCode::Escape) {
            w.mode = WindowMode::Windowed;
        }
        // Reserve space for instruments on both sides, keeping a 4m bounding sphere visible.
        let panel_fraction =
            (520. * (w.width() / 1280.).min(w.height() / 800.).min(1.) / w.width()).min(0.65);
        let aspect = w.width() / w.height().max(1.);
        let d = view.distance.max(
            4.1 / ((22.5_f32.to_radians().tan() * aspect * (1. - panel_fraction))
                .atan()
                .sin()),
        );
        let (pos, up) = match view.mode {
            1 => (Vec3::Y * d, Vec3::NEG_Z),
            2 => (Vec3::NEG_Z * d, Vec3::Y),
            3 => (Vec3::X * d, Vec3::Y),
            _ => {
                let (s, c) = view.azimuth.sin_cos();
                let (e, h) = view.elevation.sin_cos();
                (Vec3::new(s * h, e, -c * h) * d, Vec3::Y)
            }
        };
        for (mut t, mut projection) in &mut cameras {
            *t = Transform::from_translation(pos).looking_at(Vec3::ZERO, up);
            if let Projection::Perspective(p) = projection.as_mut() {
                p.aspect_ratio = aspect;
            }
        }
    }
}
