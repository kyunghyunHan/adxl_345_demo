use bevy::{
    asset::RenderAssetUsages, mesh::Indices, prelude::*, render::render_resource::PrimitiveTopology,
};
const HALF: f32 = 8000.;
const STEP: f32 = 100.;
const N: usize = 161;
fn elevation(x: f32, z: f32) -> f32 {
    let hill = |cx: f32, cz: f32, width: f32, height: f32| {
        height * (-((x - cx).powi(2) + (z - cz).powi(2)) / width.powi(2)).exp()
    };
    65. + 22. * (x / 550.).sin() * (z / 680.).cos()
        + hill(1700., -1600., 900., 1150.)
        + hill(2900., -2400., 1100., 1350.)
        + hill(-2800., -3500., 1000., 950.)
        + hill(400., -4800., 1300., 1650.)
        - hill(-1300., -1000., 1100., 180.)
}
/// Same two triangles as the rendered heightfield, so collision follows visible ground.
pub fn ground_height(x: f32, z: f32) -> f32 {
    let gx = ((x + HALF) / STEP).clamp(0., 159.9999);
    let gz = ((z + HALF) / STEP).clamp(0., 159.9999);
    let i = gx.floor();
    let j = gz.floor();
    let u = gx - i;
    let v = gz - j;
    let x = i * STEP - HALF;
    let z = j * STEP - HALF;
    let a = elevation(x, z);
    let b = elevation(x + STEP, z);
    let c = elevation(x, z + STEP);
    let d = elevation(x + STEP, z + STEP);
    if u + v <= 1. {
        a + (b - a) * u + (c - a) * v
    } else {
        d + (c - d) * (1. - u) + (b - d) * (1. - v)
    }
}
pub fn surface_height(x: f32, z: f32) -> f32 {
    ground_height(x, z).max(0.)
}
pub fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut positions = Vec::with_capacity(N * N);
    let mut colors = Vec::with_capacity(N * N);
    let mut indices = Vec::new();
    for z in 0..N {
        for x in 0..N {
            let x = x as f32 * STEP - HALF;
            let z = z as f32 * STEP - HALF;
            let y = elevation(x, z);
            positions.push([x, y, z]);
            let c = if y > 1250. {
                [0.83, 0.87, 0.88, 1.]
            } else if y > 700. {
                [0.34, 0.39, 0.37, 1.]
            } else if y < 18. {
                [0.65, 0.57, 0.36, 1.]
            } else {
                [0.19 + (y / 4000.), 0.37, 0.23, 1.]
            };
            colors.push(c);
        }
    }
    for z in 0..N - 1 {
        for x in 0..N - 1 {
            let a = (z * N + x) as u32;
            let b = a + 1;
            let c = a + N as u32;
            let d = c + 1;
            indices.extend([a, c, b, b, c, d]);
        }
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
    .with_inserted_indices(Indices::U32(indices));
    mesh.compute_smooth_normals();
    commands.spawn((
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(materials.add(StandardMaterial {
            perceptual_roughness: 1.,
            ..default()
        })),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(16000., 16000.))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.06, 0.37, 0.52),
            metallic: 0.25,
            perceptual_roughness: 0.23,
            ..default()
        })),
        Transform::from_xyz(0., 0., 0.),
    ));
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.72, 0.84, 1.),
        brightness: 350.,
        ..default()
    });
    commands.spawn((
        DirectionalLight {
            illuminance: 18000.,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(-3000., 6000., 2000.).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    let cloud_mesh = meshes.add(Sphere::new(1.).mesh().uv(16, 10));
    let cloud_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.97, 1.),
        perceptual_roughness: 1.,
        ..default()
    });
    for i in 0..32 {
        let x = ((i * 2137) % 13000) as f32 - 6500.;
        let z = ((i * 3571) % 13000) as f32 - 6500.;
        for j in 0..3 {
            commands.spawn((
                Mesh3d(cloud_mesh.clone()),
                MeshMaterial3d(cloud_mat.clone()),
                Transform::from_xyz(
                    x + j as f32 * 95.,
                    2000. + (i % 4) as f32 * 140. + (j % 2) as f32 * 30.,
                    z,
                )
                .with_scale(Vec3::new(170., 55. + j as f32 * 12., 95.)),
            ));
        }
    }
    // Runway and approach markings give speed and altitude an immediate visual reference.
    let runway = materials.add(Color::srgb(0.13, 0.16, 0.18));
    let marking = materials.add(Color::WHITE);
    let y = ground_height(0., 1000.);
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(45., 2., 650.))),
        MeshMaterial3d(runway),
        Transform::from_xyz(0., y + 2., 1000.),
    ));
    for i in 0..12 {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(2., 0.2, 22.))),
            MeshMaterial3d(marking.clone()),
            Transform::from_xyz(0., y + 3.2, 725. + i as f32 * 50.),
        ));
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mesh_vertices_and_lake_collision() {
        for (x, z) in [(0., 0.), (1700., -1600.), (-1300., -1000.)] {
            assert!((ground_height(x, z) - elevation(x, z)).abs() < 0.01);
        }
        assert_eq!(surface_height(-1300., -1000.), 0.);
    }
}
