
use bevy::{
	prelude::*,
	utils::{Duration, HashMap},
};
use bevy_turborand::*;
use std::f32::consts::PI;
use super::{player, physics, ui, spells, collapse_vec3, expand_vec2, levels};

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
	fn build(&self, app: &mut App) {
		app
			.add_startup_system_to_stage(StartupStage::PreStartup, load_enemy_sprites)
			.add_system(enemy_ai_general_update)
			.add_system_to_stage(CoreStage::PreUpdate, knockback_pre_update)
			.add_system(
				knockback_post_update
					.before(physics::update_movement)
					.after(spells::process_spell_enemy_collisions)
			)
			.add_system(do_enemy_ai::<NoAI>.before(knockback_post_update))
			.add_system(do_enemy_ai::<AIPeriodicCharge>.before(knockback_post_update))
			.add_system(do_enemy_ai::<AIRotateAround>.before(knockback_post_update))
			.add_system(clean_dead_enemies);
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

#[derive(Component)]
pub struct EnemyMarker;
#[derive(Bundle)]
pub struct EnemyBundle<T: EnemyAIState> {
	marker: EnemyMarker,
	ai_state: T,
	ai_data: AIGeneralState,
	health: EnemyHealth,
	collide_damage: DamagePlayerComponent,
	speed: physics::Speed,
	own_damage_collider: physics::CollisionRecipient<physics::InteractsWithEnemies>,
	player_damage_collider: physics::CollisionSource<physics::InteractsWithPlayer>,
	player_space_collider: physics::SymmetricCollisionSource<physics::TakesSpace>,
	wall_collider: physics::CollisionRecipient<physics::WallCollidable>,
	#[bundle]
	spatial: SpatialBundle,
	rng: RngComponent,
	knockback: EnemyKnockbackComponent,
	cleanup: levels::CleanUpOnRoomLoad,
}

impl<T: EnemyAIState> EnemyBundle<T> {
	pub fn new(
		max_health: i32, 
		contact_damage: i32,
		collider: physics::Collider, 
		spatial: SpatialBundle, 
		global_rng: &mut GlobalRng,
	) -> Self {
		EnemyBundle::<T>::with_state(
			T::default(),
			max_health, 
			contact_damage,
			1.0,
			collider, 
			spatial, 
			global_rng,
		)
	}
	pub fn with_state(
		ai_state: T,
		max_health: i32, 
		contact_damage: i32,
		knockback_factor: f32,
		collider: physics::Collider, 
		spatial: SpatialBundle, 
		global_rng: &mut GlobalRng,
	) -> Self {
		EnemyBundle {
			marker: EnemyMarker,
			ai_state,
			ai_data: AIGeneralState {
				has_noticed_player: false,
				view_radius: 130.0,
			},
			health: EnemyHealth(max_health, max_health),
			collide_damage: DamagePlayerComponent(contact_damage),
			speed: physics::Speed(Vec2::ZERO),
			own_damage_collider: physics::CollisionRecipient::<physics::InteractsWithEnemies>::new(collider.clone()),
			player_damage_collider: physics::CollisionSource::<physics::InteractsWithPlayer>::new(collider.clone()),
			player_space_collider: physics::SymmetricCollisionSource::<physics::TakesSpace>::new(collider.clone()),
			wall_collider: physics::CollisionRecipient::<physics::WallCollidable>::new(collider),
			spatial,
			rng: RngComponent::with_seed(global_rng.u64(0..=u64::MAX)),
			knockback: EnemyKnockbackComponent(Vec2::ZERO, knockback_factor),
			cleanup: levels::CleanUpOnRoomLoad,
		}
	}
}

// Player contact damage /////////////////////////////
#[derive(Component)]
pub struct DamagePlayerComponent(pub i32);


// Knockback handling ////////////////////////////////
#[derive(Debug, Component)]
pub struct EnemyKnockbackComponent(pub Vec2, f32);

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
		
		speed.0 -= knockback.0 * knockback.1;
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
		speed.0 += knockback.0 * knockback.1;
	}
}

// State information common to all AI types
#[derive(Component, Debug)]
pub struct AIGeneralState {
	has_noticed_player: bool,
	view_radius: f32,
}

#[derive(Component, Debug)]
pub struct EnemyHealth(pub i32, pub i32);

// General systems /////////////////////////////
fn enemy_ai_general_update(
	mut query: Query<(&mut AIGeneralState, &Transform, &EnemyHealth), Without<player::Player>>,
	player_query: Query<&Transform, With<player::Player>>,
    spell_ui_active: Res<ui::SpellUiActive>,
) {
	if spell_ui_active.0 {
		return;
	}
	let player_pos = player_query.single().translation;
	
	for (mut state, transform, health) in query.iter_mut() {
		let pos = transform.translation;
		if !state.has_noticed_player && (
			pos.distance(player_pos) < state.view_radius
			|| health.0 < health.1
		) {
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
			commands.get_or_spawn(e).despawn_recursive();
		}
	}
}

// AI types //////////////////////////////////////////////////

#[derive(Component, Default)]
pub struct NoAI;

impl EnemyAIState for NoAI {
	fn update(
		&mut self,
		_general_state: &AIGeneralState, 
		_speed: &mut physics::Speed, 
		_own_pos: &mut Transform,
		_player_pos: Vec2,
		_time_delta: Duration,
		_rng: &mut RngComponent,
	) {
		()
	}
}

// //////////////////////////////////////////////////////////////////////
#[derive(Component)]
pub struct AIPeriodicCharge {
	pub timer: Timer,
	pub is_charging: bool,
	pub speed: f32,
	pub max_dev_angle: f32,
	pub target_speed: Vec2,
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


// //////////////////////////////////////////////////////////////////////
#[derive(Component, Debug)]
pub struct AIRotateAround {
	pub is_charging: bool,
	pub rotate_max_speed: f32,
	pub rotate_period_coef: f32,
	pub rotate_dist: f32,
	pub charge_speed: f32,
	pub time_counter: Duration,
	pub charge_decide_rate: f32,
	pub charge_timer: Timer,
}

impl Default for AIRotateAround {
	fn default() -> Self {
		Self {
			is_charging: false,
			rotate_max_speed: 100.0,
			charge_speed: 160.0,
			rotate_dist: 80.0,
			time_counter: Duration::from_secs(0),
			rotate_period_coef: std::f32::consts::TAU / 6.0,
			charge_decide_rate: 0.25,
			charge_timer: Timer::from_seconds(2.0 * 80.0 / 140.0, false),
		}
	}
}

impl AIRotateAround {
	fn new(rotate_max_speed: f32, rotate_period: f32, rotate_dist: f32, charge_speed: f32, charge_frequency: f32) -> Self {
		Self {
			rotate_max_speed,
			charge_speed,
			rotate_period_coef: std::f32::consts::TAU / rotate_period,
			rotate_dist,
			charge_decide_rate: 1.0 / charge_frequency,
			charge_timer: Timer::from_seconds(2.0 * rotate_dist / charge_speed, false),
			..default()
		}
	}
}

impl EnemyAIState for AIRotateAround {
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
		
		self.time_counter += time_delta;
		
		if !self.is_charging {
			let own_pos_2d = collapse_vec3(own_pos.translation);
			
			// decide if we should start charging
			// (this makes it exponentially distributed)
			let start_charging = rng.chance(-1.0 * (-1.0 * time_delta.as_secs_f64() * self.charge_decide_rate as f64).exp_m1());
			if start_charging {
				self.charge_timer.reset();
				self.is_charging = true;
				
				speed.0 = (player_pos - own_pos_2d).normalize_or_zero() * self.charge_speed;
			} else {
				// Determine how to move
				let current_speed = (self.time_counter.as_secs_f32() * self.rotate_period_coef).sin() * self.rotate_max_speed;
				
				let player_dist_vec = player_pos - own_pos_2d;
				let circle_inward = (player_dist_vec).normalize_or_zero();
				let circle_tangent = circle_inward.perp();
				let dist_to_player = player_dist_vec.length();
				
				// Make a vector in the direction* we want to move
				// (if speed is negative, we want to move in the opposite of this direction)
				let move_direction = (
					circle_tangent
					+ circle_inward * current_speed.signum() / (
						1.0 + 2.0 * (dist_to_player / self.rotate_dist - 1.0).abs()
					)
				).normalize();
				
				speed.0 = move_direction * current_speed;
			}
		} else {
			// We just keep moving as we were
			self.charge_timer.tick(time_delta);
			if self.charge_timer.finished() {
				self.is_charging = false;
			}
		}
	}
}

// Sprite loading
#[derive(Deref, DerefMut)]
pub struct EnemySprites(pub HashMap<String, Handle<TextureAtlas>>);
impl EnemySprites {
	pub fn get_sprite(&self, key: &str) -> Handle<TextureAtlas> {
		self.0.get(&key.to_string()).expect("invalid enemy sprite key encountered").clone()
	}
}

fn load_enemy_sprites(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
	let handles = [
		("spiky", "enemies/spiky-enemy.png", (24,24,4,2)),
		("eye", "enemies/eye-enemy.png", (20,20,8,1)),
	].iter()
		.map(|(key, path, (w,h,nx,ny))| {
			let texture_handle = asset_server.load(*path);
			(
				key.to_string(), 
				texture_atlases.add(TextureAtlas::from_grid(
					texture_handle,
					Vec2::new(*w as f32, *h as f32),
					*nx,
					*ny,
				))
			)
		})
		.collect();
	
	commands.insert_resource(EnemySprites(handles));
}
