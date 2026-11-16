use crate::physics::Flight;
use bevy::prelude::*;
#[derive(Component)]
pub struct Aircraft;
#[derive(Component)]
pub struct Propeller;

pub fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let white = materials.add(Color::srgb(0.91, 0.92, 0.84));
    let red = materials.add(Color::srgb(0.83, 0.12, 0.07));
    let dark = materials.add(Color::srgb(0.045, 0.08, 0.12));
    let glass = materials.add(StandardMaterial {
        base_color: Color::srgb(0.12, 0.35, 0.45),
        metallic: 0.5,
        perceptual_roughness: 0.15,
        ..default()
    });
    let fuselage = meshes.add(Sphere::new(1.).mesh().uv(24, 16));
    let wing = meshes.add(Cuboid::new(1., 1., 1.));
    let flight = Flight::default();
    commands
        .spawn((
            Aircraft,
            Transform::from_translation(flight.position),
            Visibility::default(),
        ))
        .with_children(|p| {
            p.spawn((
                Mesh3d(fuselage.clone()),
                MeshMaterial3d(white.clone()),
                Transform::from_xyz(0., 0., 0.).with_scale(Vec3::new(0.8, 0.85, 4.2)),
            ));
            p.spawn((
                Mesh3d(fuselage.clone()),
                MeshMaterial3d(red.clone()),
                Transform::from_xyz(0., 0., -3.1).with_scale(Vec3::new(0.78, 0.78, 1.05)),
            ));
            p.spawn((
                Mesh3d(fuselage.clone()),
                MeshMaterial3d(glass),
                Transform::from_xyz(0., 0.7, -0.7).with_scale(Vec3::new(0.69, 0.65, 1.35)),
            ));
            // Separate left/right wing panels, tip stripes, tailplane and vertical fin.
            for side in [-1., 1.] {
                p.spawn((
                    Mesh3d(wing.clone()),
                    MeshMaterial3d(white.clone()),
                    Transform::from_xyz(side * 3., 0., -0.4)
                        .with_scale(Vec3::new(5.4, 0.15, 1.55))
                        .with_rotation(Quat::from_rotation_z(side * 0.035)),
                ));
                p.spawn((
                    Mesh3d(wing.clone()),
                    MeshMaterial3d(red.clone()),
                    Transform::from_xyz(side * 5.45, 0.09, -0.4)
                        .with_scale(Vec3::new(0.5, 0.16, 1.56)),
                ));
                p.spawn((
                    Mesh3d(wing.clone()),
                    MeshMaterial3d(white.clone()),
                    Transform::from_xyz(side * 1.35, 0.22, 3.)
                        .with_scale(Vec3::new(2.5, 0.13, 0.9)),
                ));
                // Landing gear legs and tires.
                p.spawn((
                    Mesh3d(wing.clone()),
                    MeshMaterial3d(dark.clone()),
                    Transform::from_xyz(side * 0.95, -0.95, 0.3)
                        .with_scale(Vec3::new(0.12, 1.2, 0.12)),
                ));
                p.spawn((
                    Mesh3d(fuselage.clone()),
                    MeshMaterial3d(dark.clone()),
                    Transform::from_xyz(side * 0.95, -1.55, 0.3)
                        .with_scale(Vec3::new(0.15, 0.37, 0.37)),
                ));
            }
            p.spawn((
                Mesh3d(wing.clone()),
                MeshMaterial3d(red.clone()),
                Transform::from_xyz(0., 1.05, 2.95)
                    .with_scale(Vec3::new(0.13, 1.7, 1.25))
                    .with_rotation(Quat::from_rotation_x(-0.2)),
            ));
            p.spawn((
                Mesh3d(fuselage.clone()),
                MeshMaterial3d(dark.clone()),
                Transform::from_xyz(0., 0., -4.1).with_scale(Vec3::splat(0.3)),
            ));
            p.spawn((
                Propeller,
                Transform::from_xyz(0., 0., -4.22),
                Visibility::default(),
            ))
            .with_children(|prop| {
                for a in [0., std::f32::consts::FRAC_PI_2] {
                    prop.spawn((
                        Mesh3d(wing.clone()),
                        MeshMaterial3d(dark.clone()),
                        Transform::from_rotation(Quat::from_rotation_z(a))
                            .with_scale(Vec3::new(0.16, 3.1, 0.07)),
                    ));
                }
            });
        });
}
pub fn animate(
    time: Res<Time>,
    flight: Res<Flight>,
    mut plane: Query<&mut Transform, (With<Aircraft>, Without<Propeller>)>,
    mut prop: Query<&mut Transform, (With<Propeller>, Without<Aircraft>)>,
) {
    for mut t in &mut plane {
        t.translation = flight.position;
        t.rotation = flight.orientation;
    }
    if !flight.crashed {
        for mut t in &mut prop {
            t.rotate_local_z(time.delta_secs() * 65.);
        }
    }
}
