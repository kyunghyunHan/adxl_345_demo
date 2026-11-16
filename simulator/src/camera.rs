use crate::{physics::Flight, terrain};
use bevy::prelude::*;
#[derive(Component)]
pub struct ChaseCamera;
pub fn setup(mut commands: Commands) {
    let pos = Flight::default().position;
    commands.spawn((
        ChaseCamera,
        Camera3d::default(),
        DistanceFog {
            color: Color::srgb(0.42, 0.69, 0.87),
            falloff: FogFalloff::Linear {
                start: 4500.,
                end: 11000.,
            },
            ..default()
        },
        Projection::Perspective(PerspectiveProjection {
            fov: 55_f32.to_radians(),
            near: 0.5,
            far: 20000.,
            ..default()
        }),
        Transform::from_translation(pos + Vec3::new(0., 8., 28.))
            .looking_at(pos + Vec3::new(0., 1., -14.), Vec3::Y),
    ));
}
pub fn follow(
    time: Res<Time>,
    f: Res<Flight>,
    mut cameras: Query<&mut Transform, With<ChaseCamera>>,
) {
    let yaw = Quat::from_rotation_y(-f.heading);
    let mut desired = f.position + yaw * Vec3::new(0., 8., 28.);
    desired.y = desired
        .y
        .max(terrain::surface_height(desired.x, desired.z) + 5.);
    let target = f.position + f.orientation * Vec3::new(0., 1., -16.);
    for mut camera in &mut cameras {
        // Reset/teleport snaps immediately; ordinary flight uses exponential damping.
        if camera.translation.distance(desired) > 150. {
            camera.translation = desired;
        }
        let alpha = 1. - (-4. * time.delta_secs()).exp();
        camera.translation = camera.translation.lerp(desired, alpha);
        camera.translation.y = camera
            .translation
            .y
            .max(terrain::surface_height(camera.translation.x, camera.translation.z) + 4.);
        let rotation = Transform::from_translation(camera.translation)
            .looking_at(target, Vec3::Y)
            .rotation;
        camera.rotation = camera.rotation.slerp(rotation, alpha);
    }
}
