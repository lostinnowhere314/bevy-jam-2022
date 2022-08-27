#![forbid(unsafe_code)]
#![allow(
	clippy::type_complexity,
	clippy::too_many_arguments,
)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use bevy::{
    prelude::*,
    render::{camera::ScalingMode, texture::ImageSettings},
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
        .add_startup_system(setup)
		// TODO debug
		.add_startup_system(test_setup)
        //.add_plugin(LogDiagnosticsPlugin::default())
        //.add_plugin(FrameTimeDiagnosticsPlugin::default())
        .run();
}

use ui::{MessageTrigger, MessageEvent, MessageSource, MessageTriggerType};

fn test_setup(mut commands: Commands) {
	commands.spawn().insert(MessageTrigger {
			message_event: MessageEvent {
				message: Some("Good job, you opened the inventory".to_string()),
				source: MessageSource::Tutorial0
			},
			trigger_type: MessageTriggerType::OnSpellUi(true),
			next_message: Some(Box::new(MessageTrigger {
				message_event: MessageEvent {
					message: None,
					source: MessageSource::Tutorial0
				},
				trigger_type: MessageTriggerType::OnTimer(Timer::from_seconds(4.0, false)),
				next_message: None
			}))
		});
}

fn setup(mut commands: Commands) {
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

// Utility functions for converting between actual space and the space that we're pretending everything
// lives in for physics
fn expand_vec2(vec: Vec2) -> Vec3 {
	Vec3::new(vec.x, 0.0, vec.y)
}

fn collapse_vec3(vec: Vec3) -> Vec2 {
	Vec2::new(vec.x, vec.z)
}
