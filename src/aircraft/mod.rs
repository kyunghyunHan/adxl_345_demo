pub mod attitude;
use attitude::AttitudeState;
use bevy::{
    asset::{LoadState, RecursiveDependencyLoadState},
    prelude::*,
};

#[derive(Component)]
pub struct Aircraft;
#[derive(Resource)]
pub struct ModelStatus {
    pub scene: Handle<WorldAsset>,
    pub message: String,
}

/// Original GLB: nose +X, starboard +Z, up +Y, fuselage axis at Y=4.15m.
/// Translate AFTER scale/rotation so the attitude pivot is on the fuselage axis.
pub fn model_correction() -> Transform {
    Transform::from_xyz(0., -4.15 * 0.18, 0.)
        .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2))
        .with_scale(Vec3::splat(0.18))
}
pub fn setup(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let scene = server.load(GltfAssetLabel::Scene(0).from_asset("models/aircraft.glb"));
    commands.insert_resource(ModelStatus {
        scene: scene.clone(),
        message: "LOADING AIRCRAFT…".into(),
    });
    commands
        .spawn((Aircraft, Transform::default(), Visibility::default()))
        .with_children(|p| {
            p.spawn((WorldAssetRoot(scene), model_correction()));
        });
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.7, 0.82, 1.),
        brightness: 300.,
        ..default()
    });
    commands.spawn((
        DirectionalLight {
            illuminance: 14000.,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(-6., 10., -7.).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        PointLight {
            intensity: 350000.,
            color: Color::srgb(0.25, 0.65, 1.),
            range: 30.,
            ..default()
        },
        Transform::from_xyz(5., 4., 3.),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(200., 200.))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.002, 0.004, 0.008),
            perceptual_roughness: 0.9,
            ..default()
        })),
        Transform::from_xyz(0., -4.5, 0.),
    ));
}
pub fn rotate(state: Res<AttitudeState>, mut planes: Query<&mut Transform, With<Aircraft>>) {
    for mut plane in &mut planes {
        plane.rotation = state.display;
    }
}
pub fn loading(server: Res<AssetServer>, mut status: ResMut<ModelStatus>) {
    status.message = if server.is_loaded_with_dependencies(status.scene.id()) {
        "AIRCRAFT / GLB READY".into()
    } else if let Some(LoadState::Failed(error)) = server.get_load_state(status.scene.id()) {
        format!("MODEL ERROR: {error} — restore assets/models/aircraft.glb")
    } else if let Some(RecursiveDependencyLoadState::Failed(error)) =
        server.get_recursive_dependency_load_state(status.scene.id())
    {
        format!("MODEL ERROR: {error} — restore assets/models/aircraft.glb")
    } else {
        "LOADING AIRCRAFT…".into()
    };
}
pub fn guides(mut gizmos: Gizmos) {
    let grid = Color::srgba(0.12, 0.30, 0.40, 0.5);
    for n in -10..=10 {
        let n = n as f32;
        gizmos.line(Vec3::new(n, -4.48, -10.), Vec3::new(n, -4.48, 10.), grid);
        gizmos.line(Vec3::new(-10., -4.48, n), Vec3::new(10., -4.48, n), grid);
    }
    // Stationary world references, separate from the moving body.
    gizmos.line(
        Vec3::NEG_X * 4.,
        Vec3::X * 4.,
        Color::srgb(0.55, 0.23, 0.22),
    );
    gizmos.line(Vec3::ZERO, Vec3::Y * 3., Color::srgb(0.2, 0.55, 0.36));
    gizmos.line(Vec3::Z * 4., Vec3::NEG_Z * 4., Color::srgb(0.2, 0.48, 0.7));
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn model_axis_and_center() {
        let t = model_correction();
        assert!((t.rotation * Vec3::X - Vec3::NEG_Z).length() < 1e-5);
        assert!((t.rotation * Vec3::Z - Vec3::X).length() < 1e-5);
        assert!(t.transform_point(Vec3::new(0., 4.15, 0.)).length() < 1e-5);
    }
}
