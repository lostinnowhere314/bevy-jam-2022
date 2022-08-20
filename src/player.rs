
use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

use super::palletize;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
	fn build(&self, app: &mut App) {
		app
			.add_plugin(InputManagerPlugin::<Action>::default())
			.add_startup_system(player_setup)
			.add_system(player_movement)
			.add_system(player_sprite_update.after(player_movement));
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
	
	// Animation stuff
	/*let starting_state = "idle";
	let states_map = HashMap::from([
		("idle".to_string(), AnimationState{
			name: "idle".to_string(),
			clip: idle_clip_handle,
			interruptible: true,
		}),
		("walk".to_string(), AnimationState{
			name: "walk".to_string(),
			clip: walk_clip_handle,
			interruptible: true,
		}),
	]);
	let state_transitions = vec![
		StateMachineTransition {
			
		}
	];*/
	
	
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
				.insert_bundle(SpriteSheetBundle {
					texture_atlas: texture_atlas,
					..default()
				});
		});
}

fn player_sprite_update(
	player_query: Query<&Transform, 
		(With<Player>, Without<PlayerSpriteMarker>, Without<Camera>)>,
	mut sprite_query: Query<&mut Transform, 
		(With<PlayerSpriteMarker>, Without<Player>, Without<Camera>)>,
	camera_query: Query<&Transform, 
		(With<Camera>, Without<Player>, Without<PlayerSpriteMarker>, Without<palletize::PostProcessCameraMarker>)>,
) {
	let player_position = player_query.single().translation;
	let mut sprite_transform = sprite_query.single_mut();
	
	let camera_transform = camera_query.iter().next().expect("no camera found!");

	// First we need to transform everything w.r.t the camera
	let camera_inverse = Transform::from_matrix(
		camera_transform.compute_matrix().inverse()
	);
	let player_camera_loc = camera_inverse * player_position;
	
	// Then, we want to set the sprite to be pixel-aligned
	let target_position = player_camera_loc.round();
	
	// Then we adjust sprite positioning as needed
	sprite_transform.rotation = camera_transform.rotation;
	sprite_transform.translation = (*camera_transform * target_position) - player_position;
}


#[derive(Component, Default, Debug)]
struct PlayerSpeed(Vec3);

fn player_movement(
	action_state: Query<&ActionState<Action>, With<Player>>,
	mut player_query: Query<(&mut Transform, &mut PlayerSpeed), With<Player>>,
	time: Res<Time>,
) {
	let action_state = action_state.single();
	let (mut transform, mut speed) = player_query.single_mut();
	
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
}

const SPEED: f32 = 60.0;
const ACCEL_FORWARD: f32 = 480.0;
const ACCEL_NEUTRAL: f32 = 240.0;
const ACCEL_DECEL: f32 = 360.0;

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