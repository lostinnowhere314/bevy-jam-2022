#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use bevy::{
		prelude::*,
		render::{
			camera::ScalingMode,
			texture::ImageSettings,
		},
};
//use bevy_simple_state_machine::SimpleStateMachinePlugin;

mod player;
mod spells;

// theme = combine
fn main() {
    App::new()
		.insert_resource(WindowDescriptor {
			width: 640.0, height: 400.0,
			..default()
		})
        .insert_resource(ImageSettings::default_nearest())
		.insert_resource(Msaa {samples: 1})
        .add_plugins(DefaultPlugins)
		//.add_plugin(SimpleStateMachinePlugin::new())
		.add_plugin(player::PlayerPlugin)
        .add_startup_system(setup)
        .run();
}

const HALF_SIZE: f32 = 10.0;

fn setup(
	mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
	asset_server: Res<AssetServer>,
) {
	let orthographic_projection = OrthographicProjection {
		scale: 0.5,
		scaling_mode: ScalingMode::Auto {
			min_width: 640.0,
			min_height: 400.0,
		},
		..default()
	};
	
    commands.spawn_bundle(Camera2dBundle {
		projection: orthographic_projection.into(),
        transform: Transform::from_xyz(0.0, 100., 200.0).looking_at(Vec3::new(0., 0., 0.), Vec3::Y),
        ..default()
    });
}