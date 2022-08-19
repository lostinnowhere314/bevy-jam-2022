#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use bevy::{
		prelude::*,
		render::camera::ScalingMode
};

mod palletize;

// theme = combine
fn main() {
    App::new()
		.insert_resource(WindowDescriptor {
			width: 640.0, height: 400.0,
			..default()
		})
		.insert_resource(Msaa {samples: 1})
        .add_plugins(DefaultPlugins)
		.add_plugin(palletize::Palletize {
			palette_source: palletize::PaletteSource::Filepath("palette/apollo.txt".into())
		})
        .add_startup_system(setup)
        .run();
}

const HALF_SIZE: f32 = 10.0;

fn setup(
	mut commands: Commands,
	postprocess_target: Res<palletize::PostprocessRenderTarget>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
	
	let orthographic_projection = OrthographicProjection {
		scale: 3.0,
		scaling_mode: ScalingMode::Auto{
			min_width: 16.0,
			min_height: 12.0,
		},
		..default()
	};
    commands.spawn_bundle(Camera3dBundle {
		projection: orthographic_projection.into(),
        transform: Transform::from_xyz(0.0, 6., 12.0).looking_at(Vec3::new(0., 1., 0.), Vec3::Y),
        camera: postprocess_target.get_default_camera(),
		..default()
    });
	
	
	// Testing cube
	let cube_mesh = meshes.add(shape::Cube::default().into());
	let cube_material = materials.add(Color::rgb(0.8, 0.7, 0.6).into());
	commands
		.spawn_bundle(PbrBundle {
			mesh: cube_mesh.clone(),
			material: cube_material.clone(),
			transform: Transform::from_xyz(0.0, 0.0, 0.0),
			..default()
		});
		
	// directional 'sun' light
    commands.spawn_bundle(DirectionalLightBundle {
        directional_light: DirectionalLight {
            // Configure the projection to better fit the scene
            shadow_projection: OrthographicProjection {
                left: -HALF_SIZE,
                right: HALF_SIZE,
                bottom: -HALF_SIZE,
                top: HALF_SIZE,
                near: -10.0 * HALF_SIZE,
                far: 10.0 * HALF_SIZE,
                ..default()
            },
            shadows_enabled: true,
			illuminance: 100000.0,
            ..default()
        },
        transform: Transform {
            translation: Vec3::new(0.0, 2.0, 0.0),
            rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_4),
            ..default()
        },
        ..default()
    });
}
