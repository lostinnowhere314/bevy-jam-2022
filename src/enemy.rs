
use bevy::{
	prelude::*,
	utils::Duration
};
use bevy_turborand::*;
use std::f32::consts::PI;
use super::{player, physics, sprite, ui, spells, collapse_vec3, expand_vec2};

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
	fn build(&self, app: &mut App) {
		app
			.add_system(enemy_ai_general_update)
			.add_system_to_stage(CoreStage::PreUpdate, knockback_pre_update)
			.add_system(
				knockback_post_update
					.before(physics::update_movement)
					.after(spells::process_spell_enemy_collisions)
			)
			.add_system(do_enemy_ai::<AIPeriodicCharge>.before(knockback_post_update))
			.add_system(clean_dead_enemies)
			// TEST SYSTEM
			.add_startup_system(enemy_test_system);
	}	
}

pub trait EnemyAIState: Component+Default {
	fn update(
		&mut self,
		general_state: &AIGeneralState, 
		speed: &mut physics::Speed, 
		own_pos: &mut Transform,
		player_pos: Vec2,
		time_delta: Duration,
		rng: &mut RngComponent,
	);
}

#[derive(Bundle)]
pub struct EnemyBundle<T: EnemyAIState> {
	ai_state: T,
	ai_data: AIGeneralState,
	health: EnemyHealth,
	speed: physics::Speed,
	own_damage_collider: physics::CollisionRecipient<physics::DamagesEnemies>,
	player_damage_collider: physics::CollisionSource<physics::DamagesPlayer>,
	player_space_collider: physics::SymmetricCollisionSource<physics::TakesSpace>,
	wall_collider: physics::CollisionRecipient<physics::WallCollidable>,
	#[bundle]
	spatial: SpatialBundle,
	rng: RngComponent,
	knockback: EnemyKnockbackComponent,
}

impl<T: EnemyAIState> EnemyBundle<T> {
	pub fn new(
		max_health: i32, 
		collider: physics::Collider, 
		spatial: SpatialBundle, 
		global_rng: &mut GlobalRng,
	) -> Self {
		EnemyBundle::<T>::with_state(
			T::default(),
			max_health, 
			collider, 
			spatial, 
			global_rng,
		)
	}
	pub fn with_state(
		ai_state: T,
		max_health: i32, 
		collider: physics::Collider, 
		spatial: SpatialBundle, 
		global_rng: &mut GlobalRng,
	) -> Self {
		EnemyBundle {
			ai_state,
			ai_data: AIGeneralState {
				has_noticed_player: false,
				view_radius: 200.0,
			},
			health: EnemyHealth(max_health),
			speed: physics::Speed(Vec2::ZERO),
			own_damage_collider: physics::CollisionRecipient::<physics::DamagesEnemies>::new(collider.clone()),
			player_damage_collider: physics::CollisionSource::<physics::DamagesPlayer>::new(collider.clone()),
			player_space_collider: physics::SymmetricCollisionSource::<physics::TakesSpace>::new(collider.clone()),
			wall_collider: physics::CollisionRecipient::<physics::WallCollidable>::new(collider),
			spatial,
			rng: RngComponent::with_seed(global_rng.u64(0..=u64::MAX)),
			knockback: EnemyKnockbackComponent::new(),
		}
	}
}

// Knockback handling ////////////////////////////////
#[derive(Debug, Component)]
pub struct EnemyKnockbackComponent(pub Vec2);

impl EnemyKnockbackComponent {
	fn new() -> Self {
		Self(Vec2::ZERO)
	}
}

fn knockback_pre_update(
	mut query: Query<(&mut EnemyKnockbackComponent, &mut physics::Speed)>,
	time: Res<Time>,
    spell_ui_active: Res<ui::SpellUiActive>,
) {
	if spell_ui_active.0 {
		return;
	}
	
	let base = 0.1f32;
	let decay_factor = base.powf(time.delta_seconds());
	
	for (mut knockback, mut speed) in query.iter_mut() {
		knockback.0 *= decay_factor;
		
		speed.0 -= knockback.0;
	}
}

fn knockback_post_update(
	mut query: Query<(&EnemyKnockbackComponent, &mut physics::Speed)>,
    spell_ui_active: Res<ui::SpellUiActive>,
) {
	if spell_ui_active.0 {
		return;
	}
	for (knockback, mut speed) in query.iter_mut() {
		speed.0 += knockback.0;
	}
}

// State information common to all AI types
#[derive(Component, Debug)]
pub struct AIGeneralState {
	has_noticed_player: bool,
	view_radius: f32,
}

#[derive(Component, Debug)]
pub struct EnemyHealth(pub i32);

// General systems /////////////////////////////
fn enemy_ai_general_update(
	mut query: Query<(&mut AIGeneralState, &Transform), Without<player::Player>>,
	player_query: Query<&Transform, With<player::Player>>,
    spell_ui_active: Res<ui::SpellUiActive>,
) {
	if spell_ui_active.0 {
		return;
	}
	let player_pos = player_query.single().translation;
	
	for (mut state, transform) in query.iter_mut() {
		let pos = transform.translation;
		if !state.has_noticed_player && pos.distance(player_pos) < state.view_radius {
			state.has_noticed_player = true;
		}
	}
}

fn do_enemy_ai<T: EnemyAIState>(
	mut query: Query<(&mut T, &AIGeneralState, &mut physics::Speed, &mut Transform, &mut RngComponent), Without<player::Player>>,
	player_query: Query<&Transform, With<player::Player>>,
	time: Res<Time>,
    spell_ui_active: Res<ui::SpellUiActive>,
) {
	if spell_ui_active.0 {
		return;
	}
	
	let player_transform = player_query.single();
	
	for (mut state, general_data, mut speed, mut transform, mut rng) in query.iter_mut() {
		state.update(
			general_data, 
			&mut speed, 
			&mut transform, 
			collapse_vec3(player_transform.translation),
			time.delta(),
			&mut rng,
		);
	}
}

fn clean_dead_enemies(
	mut commands: Commands,
	query: Query<(Entity, &EnemyHealth)>
) {
	for (e, health) in query.iter() {
		if health.0 <= 0 {
			commands.entity(e).despawn_recursive();
		}
	}
}

// AI types //////////////////////////////////////////////////

#[derive(Component)]
pub struct AIPeriodicCharge {
	timer: Timer,
	is_charging: bool,
	speed: f32,
	max_dev_angle: f32,
	target_speed: Vec2,
}

impl EnemyAIState for AIPeriodicCharge {
	fn update(
		&mut self,
		general_state: &AIGeneralState, 
		speed: &mut physics::Speed, 
		own_pos: &mut Transform,
		player_pos: Vec2,
		time_delta: Duration,
		rng: &mut RngComponent,
	) {
		if !general_state.has_noticed_player {
			return;
		}
		
		self.timer.tick(time_delta);
		if self.timer.just_finished() {
			let own_pos_2 = collapse_vec3(own_pos.translation);
			
			if self.is_charging {
				let dir_to_player = player_pos - own_pos_2;
				let angle = rng.f32_normalized() * self.max_dev_angle;
				let target_dir = collapse_vec3(
					Quat::from_rotation_y(angle) * expand_vec2(dir_to_player)
				).normalize_or_zero();
				
				self.target_speed = target_dir * self.speed;
			} else {
				self.target_speed = Vec2::ZERO; 
			}
			self.is_charging = !self.is_charging;
		}
		
		let base = 0.2f32;
		let ratio = base.powf(time_delta.as_secs_f32());
		speed.0 = speed.0 * ratio + self.target_speed * (1.0 - ratio);
	}
}

impl Default for AIPeriodicCharge {
	fn default() -> Self {
		AIPeriodicCharge {
			timer: Timer::from_seconds(1.0, true),
			is_charging: true,
			speed: 160.0,
			max_dev_angle: PI / 8.0,
			target_speed: Vec2::ZERO,
		}
	}
}





// TODO testing; remove eventually
fn enemy_test_system(
	mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
	shadow_texture: Res<sprite::ShadowTexture>,
	mut global_rng: ResMut<GlobalRng>,
) {
	let texture_handle = asset_server.load("no-sprite.png");
    let texture_atlas = texture_atlases.add(TextureAtlas::from_grid(
        texture_handle,
        Vec2::new(16.0, 16.0),
        1,
        1,
    ));
	
	commands
		.spawn_bundle(EnemyBundle::<AIPeriodicCharge>::new(
			100, 
			physics::Collider::Circle {
				center: Vec2::ZERO,
				radius: 8.0
			}, 
			SpatialBundle {
				transform: Transform::from_translation(Vec3::new(60.0, 0.0, 0.0)),
				..default()
			},
			&mut global_rng
		)).with_children(|parent| {
			parent
				.spawn_bundle(SpriteSheetBundle {
                    texture_atlas,
                    ..default()
                })
				.insert(sprite::SpriteOffset(Vec3::new(0.0, 5.0, 0.0)))
				.insert(sprite::FacingSpriteMarker);
			parent.spawn_bundle(shadow_texture.get_shadow_bundle(2));
		});
}