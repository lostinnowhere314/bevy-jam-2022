
use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
	fn build(&self, app: &mut App) {
		app
			.add_plugin(InputManagerPlugin::<Action>::default())
			.add_startup_system(player_setup)
			.add_system(player_movement)
			.add_system(player_sprite_update.after(player_movement))
			.add_system(update_player_animation.after(player_movement));
	}
}


#[derive(Component, Debug)]
pub struct Player;
#[derive(Component, Debug)]
pub struct PlayerSpriteMarker;

fn player_setup(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
	// Player sprite info
	let texture_handle = asset_server.load("player/player.png");
	let texture_atlas = texture_atlases.add(
		TextureAtlas::from_grid(
			texture_handle,
			Vec2::new(32.0, 64.0),
			8, 8
		)
	);
	
	
	
	// Spawn the player
	commands
		.spawn()
		.insert(Player)
		.insert(PlayerSpeed(Vec3::ZERO))
		.insert_bundle(InputManagerBundle::<Action> {
			action_state: ActionState::default(),
			input_map: get_input_map(),
		})
		.insert_bundle(SpatialBundle::default())
		.with_children(|parent| {
			// Create something to manage the sprite properly
			parent
				.spawn()
				.insert(PlayerSpriteMarker)
				.insert(AnimationTimer(Timer::from_seconds(1.0 / 7.0, true)))
				.insert(AnimationNextState{
					state: AnimationGeneralState::Idle,
					facing_dir: FacingDir::Right,
				})
				.insert(AnimationCurrentState {
					state: AnimationState::IdleRight, 
					index:0,
				})
				.insert(SpriteOffset(Vec3::new(0.0, 24.0, 0.0)))
				.insert_bundle(SpriteSheetBundle {
					texture_atlas,
					..default()
				});
		});
}

#[derive(Component, Debug)]
struct SpriteOffset(Vec3);


// Make sprites look nice in our sort-of-3d environment
fn player_sprite_update(
	player_query: Query<&Transform, 
		(With<Player>, Without<PlayerSpriteMarker>, Without<Camera>)>,
	mut sprite_query: Query<(&mut Transform, Option<&SpriteOffset>),
		(With<PlayerSpriteMarker>, Without<Player>, Without<Camera>)>,
	camera_query: Query<&Transform, 
		(With<Camera>, Without<Player>, Without<PlayerSpriteMarker>)>,
) {
	let player_position = player_query.single().translation;
	let (mut sprite_transform, maybe_offset) = sprite_query.single_mut();
	let sprite_offset = match maybe_offset {
		Some(SpriteOffset(o)) => *o,
		None => Vec3::ZERO,
	};
	
	let camera_transform = camera_query.iter().next().expect("no camera found!");

	// First we need to transform everything w.r.t the camera
	let camera_inverse = Transform::from_matrix(
		camera_transform.compute_matrix().inverse()
	);
	let player_camera_loc = camera_inverse * (player_position + camera_transform.rotation * sprite_offset);
	
	// Then, we want to set the sprite to be pixel-aligned
	let target_position = player_camera_loc.round();
	
	// Then we adjust sprite positioning as needed
	sprite_transform.rotation = camera_transform.rotation;
	sprite_transform.translation = (*camera_transform * target_position) - player_position;
}


// Animation stuffs
// NOTE: to generalize: turn this into a trait?
#[derive(Debug, Clone, Copy, Hash, PartialEq)]
pub enum AnimationState {
	IdleRight,
	IdleLeft,
	WalkRight,
	WalkLeft,
}
impl AnimationState {	
	/// Returns the start offset of the animation
	fn offset(&self) -> usize {
		match self {
			AnimationState::IdleRight => 0,
			AnimationState::IdleLeft => 2,
			AnimationState::WalkRight => 8,
			AnimationState::WalkLeft => 16,
		}
	}
	
	fn length(&self) -> usize {
		match self {
			AnimationState::IdleRight => 1,
			AnimationState::IdleLeft => 1,
			AnimationState::WalkRight => 8,
			AnimationState::WalkLeft => 8,
		}
	}
	
	fn is_interruptible(&self) -> bool {
		match self {
			AnimationState::IdleRight => true,
			AnimationState::IdleLeft => true,
			AnimationState::WalkRight => true,
			AnimationState::WalkLeft => true,
		}
	}
	
	fn get_priority(&self) -> i32 {
		self.get_general_state().get_priority()
	}
	
	fn order_is_reversed(&self) -> bool {
		match self {
			AnimationState::IdleRight => false,
			AnimationState::IdleLeft => false,
			AnimationState::WalkRight => false,
			AnimationState::WalkLeft => true,
		}
	}
	
	fn get_general_state(&self) -> AnimationGeneralState {
		match self {
			AnimationState::IdleRight => AnimationGeneralState::Idle,
			AnimationState::IdleLeft => AnimationGeneralState::Idle,
			AnimationState::WalkRight => AnimationGeneralState::Walk,
			AnimationState::WalkLeft => AnimationGeneralState::Walk,
		}
	}
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum FacingDir {
	Left,
	Right
}
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum AnimationGeneralState {
	Idle,
	Walk,
	//Casting,
}

impl AnimationGeneralState {
	fn get_default_next_state(&self) -> AnimationGeneralState {
		match self {
			AnimationGeneralState::Idle => AnimationGeneralState::Idle,
			AnimationGeneralState::Walk => AnimationGeneralState::Walk,
		}
	}
	
	fn get_priority(&self) -> i32 {
		match self {
			AnimationGeneralState::Idle => 0,
			AnimationGeneralState::Walk => 0,
		}
	}
}

fn get_animation_state(facing_dir: FacingDir, animation_gen_state: AnimationGeneralState) -> AnimationState {
	match animation_gen_state {
		AnimationGeneralState::Idle => match facing_dir {
			FacingDir::Left => AnimationState::IdleLeft,
			FacingDir::Right => AnimationState::IdleRight,
		},
		AnimationGeneralState::Walk => match facing_dir {
			FacingDir::Left => AnimationState::WalkLeft,
			FacingDir::Right => AnimationState::WalkRight,
		}
	}
}

// All of these are needed for it to function
#[derive(Component, Deref, DerefMut)]
struct AnimationTimer(Timer);
// TODO make this one have a vec of the next states to look at
#[derive(Component, Debug)]
pub struct AnimationNextState {
	state: AnimationGeneralState, 
	facing_dir: FacingDir
}
#[derive(Component, Debug)]
struct AnimationCurrentState {
	state: AnimationState, 
	index: usize,
}

impl AnimationNextState {
	pub fn set_state_if_priority(&mut self, new_state: AnimationGeneralState) {
		if new_state.get_priority() >= self.state.get_priority() {
			self.state = new_state;
		}
	}
}

// Handles updating player animations
fn update_player_animation(
	time: Res<Time>,
	mut query: Query<
		(&mut AnimationTimer, &mut AnimationCurrentState, &mut AnimationNextState, &mut TextureAtlasSprite), 
		With<PlayerSpriteMarker>
	>,
) {
	for (mut timer, mut current_state, mut next_state, mut sprite) in &mut query {
		// update timer
		timer.tick(time.delta());
		// Determine if the next animation should interrupt the current one
		let next_transition_state = get_animation_state(next_state.facing_dir, next_state.state);
		let interrupt = 
			next_transition_state != current_state.state 
			&& current_state.state.is_interruptible() 
			&& next_transition_state.get_priority() >= current_state.state.get_priority();
		let interrupt_immediate = interrupt
			&& next_transition_state.get_priority() > current_state.state.get_priority();
		
		if timer.just_finished() || interrupt_immediate {
			if interrupt_immediate {
				timer.reset();
			}
			
			current_state.index += 1;
			
			// Figure out if we need to transition
			if current_state.index >= current_state.state.length() || interrupt {
				// Reset index
				current_state.index = 0;
				// Find the default state to transition to after this one
				let default_next_state = next_state.state.get_default_next_state();
				// Do the transition
				current_state.state = next_transition_state;
				
				next_state.state = default_next_state;
			}
			
			// Update sprite index
			sprite.index = if current_state.state.order_is_reversed() {
				current_state.state.offset() + (current_state.state.length() - current_state.index - 1)
			} else {
				current_state.state.offset() + current_state.index
			};
		}
	}
}


// Movement
#[derive(Component, Default, Debug)]
struct PlayerSpeed(Vec3);

fn player_movement(
	action_state: Query<&ActionState<Action>, With<Player>>,
	mut player_query: Query<(&mut Transform, &mut PlayerSpeed), With<Player>>,
	mut anim_query: Query<&mut AnimationNextState, With<PlayerSpriteMarker>>,
	time: Res<Time>,
) {
	let action_state = action_state.single();
	let (mut transform, mut speed) = player_query.single_mut();
	let mut anim_next_state = anim_query.single_mut();
	
	let mut total_offset = Vec3::splat(0.0);
	
	if action_state.pressed(Action::Up) {
		total_offset.z -= 1.0;
	}
	if action_state.pressed(Action::Down) {
		total_offset.z += 1.0;
	}
	if action_state.pressed(Action::Right) {
		total_offset.x += 1.0;
	}
	if action_state.pressed(Action::Left) {
		total_offset.x -= 1.0;
	}
	
	// Update speed
	let target_speed = total_offset.normalize_or_zero() * SPEED;
	
	
	speed.0.x = update_speed(speed.0.x, target_speed.x, time.delta_seconds());
	speed.0.z = update_speed(speed.0.z, target_speed.z, time.delta_seconds());
	
	// Update position
	transform.translation += speed.0 * time.delta_seconds();
	
	// Update animation info
	if speed.0.x > 0.1 {
		anim_next_state.facing_dir = FacingDir::Right;
	} else if speed.0.x < -0.1 {
		anim_next_state.facing_dir = FacingDir::Left;
	}
	anim_next_state.set_state_if_priority(if speed.0.length() > 1.0 {
		AnimationGeneralState::Walk
	} else {
		AnimationGeneralState::Idle
	});
}

const SPEED: f32 = 70.0;
const ACCEL_FORWARD: f32 = 560.0;
const ACCEL_NEUTRAL: f32 = 360.0;
const ACCEL_DECEL: f32 = 480.0;

/// Moves from start towards limit up to amt.
fn move_not_past(start: f32, amt: f32, limit: f32) -> f32 {
	let diff = limit - start;
	let diff_amt = diff.abs();
	let sign = diff.signum();
	let amt = amt.abs();
	
	if diff_amt < 1e-4 || diff_amt < amt {
		// If we're at the limit or would overshoot
		limit
	} else {
		// Just move
		start + amt * sign
	}
}

fn update_speed(current_speed: f32, target_speed: f32, delta: f32) -> f32 {
	// Determine which acceleration to use
	let accel = if target_speed.abs() < 1e-4 {
		// Target speed is zero; decelerate w/ neutral amount
		ACCEL_NEUTRAL
	} else {
		// Figure out 
		if current_speed.signum() == target_speed.signum() {
			if current_speed.abs() >= target_speed.abs() {
				// we're going too fast
				ACCEL_NEUTRAL
			} else {
				// speed up
				ACCEL_FORWARD
			}
		} else {
			// we're going in the wrong direction
			ACCEL_DECEL
		}
	};
	
	move_not_past(current_speed, accel * delta, target_speed)
}

// Input handling
#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug)]
enum Action  {
	Left,
	Right,
	Up,
	Down,
	Run,
	Dodge,
	CastSpell,
	QueueComp1,
	QueueComp2,
	QueueComp3,
	QueueComp4,
	QueueComp5,
}


fn get_input_map() -> InputMap<Action> {
	InputMap::new([
		(KeyCode::W, Action::Up),
		(KeyCode::A, Action::Left),
		(KeyCode::S, Action::Down),
		(KeyCode::D, Action::Right),
		(KeyCode::Up, Action::Up),
		(KeyCode::Left, Action::Left),
		(KeyCode::Down, Action::Down),
		(KeyCode::Right, Action::Right),
		(KeyCode::LShift, Action::Run),
	])
}