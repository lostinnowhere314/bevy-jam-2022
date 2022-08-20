#![forbid(unsafe_code)]
#![allow(clippy::type_complexity)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use bevy::{
		prelude::*,
		render::{
			camera::ScalingMode,
			texture::ImageSettings,
		},
};

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
		.add_plugin(player::PlayerPlugin)
        .add_startup_system(setup)
        .run();
}

fn setup(
	mut commands: Commands,
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
		projection: orthographic_projection,
        transform: Transform::from_xyz(0.0, 100., 200.0).looking_at(Vec3::new(0., 0., 0.), Vec3::Y),
        ..default()
    });
}
