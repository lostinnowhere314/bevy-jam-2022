
use bevy::prelude::*;
use super::{player, physics};


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
	speed: physics::Speed,
	player_damage_collider: physics::CollisionSource<physics::DamagesPlayer>,
	player_space_collider: physics::SymmetricCollisionSource<physics::TakesSpace>,
	wall_collider: physics::CollisionRecipient<physics::WallCollidable>,
	#[bundle]
	spatial: SpatialBundle,
}

impl EnemyBundle {
	pub fn new(ai_state: AIState, collider: physics::Collider, spatial: SpatialBundle) -> Self {
		EnemyBundle {
			ai_state,
			speed: physics::Speed(Vec3::ZERO),
			player_damage_collider: physics::CollisionSource::<physics::DamagesPlayer>::new(collider.clone()),
			player_space_collider: physics::SymmetricCollisionSource::<physics::TakesSpace>::new(collider.clone()),
			wall_collider: physics::CollisionRecipient::<physics::WallCollidable>::new(collider),
			spatial,
		}
	}
}

#[derive(Component)]
pub enum AIState {
	PeriodicCharge
}

impl AIState {
	fn new() -> Self {
		unimplemented!()
	}
}

fn do_enemy_ai(
	query: Query<(&mut AIState, &mut physics::Speed, &Transform)>,
	player_query: Query<&Transform, With<player::Player>>,
) {
	// TODO
}

fn enemy_test_system(
	mut commands: Commands,
) {
	// TODO
}