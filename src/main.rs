#![forbid(unsafe_code)]
#![allow(
	clippy::type_complexity,
	clippy::too_many_arguments,
)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use bevy::{
    prelude::*,
    render::texture::ImageSettings,
	//diagnostic::{LogDiagnosticsPlugin, FrameTimeDiagnosticsPlugin},
};
use bevy_turborand::*;

mod physics;
mod player;
mod spells;
mod sprite;
mod ui;
mod enemy;
mod levels;

// theme = combine
fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            width: 640.0,
            height: 400.0,
			resizable: false,
            ..default()
        })
        .insert_resource(ImageSettings::default_nearest())
        .insert_resource(Msaa { samples: 1 })
        .add_plugins(DefaultPlugins)
        .add_plugin(RngPlugin::default())
        .add_plugin(player::PlayerPlugin)
        .add_plugin(sprite::FacingSpritePlugin)
        .add_plugin(spells::SpellPlugin)
        .add_plugin(physics::GeneralPhysicsPlugin)
		.add_plugin(enemy::EnemyPlugin)
        .add_plugin(ui::UIPlugin)
		.add_plugin(levels::LevelsPlugin)
        //.add_plugin(LogDiagnosticsPlugin::default())
        //.add_plugin(FrameTimeDiagnosticsPlugin::default())
        .run();
}


// Utility functions for converting between actual space and the space that we're pretending everything
// lives in for physics
fn expand_vec2(vec: Vec2) -> Vec3 {
	Vec3::new(vec.x, 0.0, vec.y)
}

fn collapse_vec3(vec: Vec3) -> Vec2 {
	Vec2::new(vec.x, vec.z)
}
