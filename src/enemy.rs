
use bevy::prelude::*;
use super::{player, physics, sprite};


pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
	fn build(&self, app: &mut App) {
		app
			.add_system(do_enemy_ai)
			// TEST SYSTEM
			.add_startup_system(enemy_test_system);
	}	
}

#[derive(Bundle)]
pub struct EnemyBundle {
	ai_state: AIState,
	health: EnemyHealth,
	speed: physics::Speed,
	own_damage_collider: physics::CollisionRecipient<physics::DamagesEnemies>,
	player_damage_collider: physics::CollisionSource<physics::DamagesPlayer>,
	player_space_collider: physics::SymmetricCollisionSource<physics::TakesSpace>,
	wall_collider: physics::CollisionRecipient<physics::WallCollidable>,
	#[bundle]
	spatial: SpatialBundle,
}

impl EnemyBundle {
	pub fn new(ai_type: AIType, max_health: i32, collider: physics::Collider, spatial: SpatialBundle) -> Self {
		EnemyBundle {
			ai_state: ai_type.to_default_ai_state(),
			health: EnemyHealth(max_health),
			speed: physics::Speed(Vec2::ZERO),
			own_damage_collider: physics::CollisionRecipient::<physics::DamagesEnemies>::new(collider.clone()),
			player_damage_collider: physics::CollisionSource::<physics::DamagesPlayer>::new(collider.clone()),
			player_space_collider: physics::SymmetricCollisionSource::<physics::TakesSpace>::new(collider.clone()),
			wall_collider: physics::CollisionRecipient::<physics::WallCollidable>::new(collider),
			spatial,
		}
	}
}

pub enum AIType {
	PeriodicCharge
}

impl AIType {
	fn to_default_ai_state(&self) -> AIState {
		match self {
			Self::PeriodicCharge => AIState::PeriodicCharge(Vec2::ZERO)
		}
	}
}

#[derive(Component, Debug)]
enum AIState {
	PeriodicCharge(Vec2)
}

#[derive(Component, Debug)]
pub struct EnemyHealth(pub i32);

fn do_enemy_ai(
	query: Query<(&mut AIState, &mut physics::Speed, &Transform)>,
	player_query: Query<&Transform, With<player::Player>>,
) {
	// TODO
}

fn enemy_test_system(
	mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
	let texture_handle = asset_server.load("no-sprite.png");
    let texture_atlas = texture_atlases.add(TextureAtlas::from_grid(
        texture_handle,
        Vec2::new(16.0, 16.0),
        1,
        1,
    ));
	
	commands
		.spawn_bundle(EnemyBundle::new(
			AIType::PeriodicCharge, 
			100, 
			physics::Collider::Circle {
				center: Vec2::ZERO,
				radius: 16.0
			}, 
			SpatialBundle {
				transform: Transform::from_translation(Vec3::new(60.0, 0.0, 0.0)),
				..default()
			}
		)).with_children(|parent| {
			parent
				.spawn_bundle(SpriteSheetBundle {
                    texture_atlas,
                    ..default()
                })
				.insert(sprite::SpriteOffset(Vec3::new(0.0, 20.0, 0.0)))
				.insert(sprite::FacingSpriteMarker);
		});
}