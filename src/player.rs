use super::{physics, spells, sprite, ui, collapse_vec3};
use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(InputManagerPlugin::<Action>::default())
            .add_startup_system(player_setup)
			.add_system(update_player_state.after(update_spell_casting).before(player_movement))
            .add_system(player_movement.before(physics::update_movement))
            .add_system(update_spell_casting)
            .add_system(update_player_animation.after(player_movement).after(update_player_state));
    }
}

#[derive(Component, Debug)]
pub struct Player;
#[derive(Component, Debug)]
pub struct CurrentPlayerState(PlayerState);
#[derive(Debug, PartialEq, Eq)]
pub enum PlayerState {
	Normal,
	Casting,
	Knockback,
}
#[derive(Component, Debug)]
pub struct PlayerSpriteMarker;
#[derive(Component, Debug)]
pub struct PlayerVulnerability; // TODO keep values and a timer to determine vulnerability
// also TODO add health and mana

fn player_setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
	shadow_texture: Res<sprite::ShadowTexture>,
) {
    // Player sprite info
    let player_texture = asset_server.load("player/player.png");
    let player_texture_atlas = texture_atlases.add(TextureAtlas::from_grid(
        player_texture,
        Vec2::new(32.0, 64.0),
        8,
        8,
    ));
    let player_staff_texture = asset_server.load("player/player-staff.png");
    let player_staff_texture_atlas = texture_atlases.add(TextureAtlas::from_grid(
        player_staff_texture,
        Vec2::new(48.0, 64.0),
        8,
        8,
    ));

	let collider = physics::Collider::Circle {
		center: Vec2::ZERO,
		radius: 5.0
	};
	
    // Spawn the player
    commands
        .spawn()
        .insert(Player)
		.insert(CurrentPlayerState(PlayerState::Normal))
		.insert(PlayerVulnerability)
        .insert(physics::Speed(Vec2::ZERO))
        .insert_bundle(InputManagerBundle::<Action> {
            action_state: ActionState::default(),
            input_map: get_input_map(),
        })
        .insert_bundle(SpatialBundle::default())
        .insert(spells::RuneCastQueue::new())
		.insert(physics::CollisionRecipient::<physics::WallCollidable>::new(collider.clone()))
		.insert(physics::CollisionRecipient::<physics::DamagesPlayer>::new(collider.clone()))
		.insert(physics::ColliderActive::<physics::DamagesPlayer>::new(true))
		.insert(physics::SymmetricCollisionSource::<physics::TakesSpace>::new(collider.clone()))
        .with_children(|parent| {
            // Manage the sprite properly
            parent
                .spawn()
                .insert(PlayerSpriteMarker)
                .insert(sprite::FacingSpriteMarker)
                .insert(sprite::AnimationTimer(Timer::from_seconds(1.0 / 7.0, true)))
                .insert(PlayerAnimationState {
                    anim_state: AnimationState::Idle,
                    facing_dir: FacingDir::Right,
                    index: 0,
                })
                .insert(sprite::SpriteOffset(Vec3::new(0.0, 22.0, 0.0)))
                .insert_bundle(SpriteSheetBundle {
                    texture_atlas: player_staff_texture_atlas,
                    ..default()
                });
			parent.spawn_bundle(shadow_texture.get_shadow_bundle(2));
        });
}

fn update_player_state(
    mut query: Query<(&mut CurrentPlayerState, &mut PlayerVulnerability, &spells::RuneCastQueue), With<Player>>,
) {
	let (mut player_state, player_vulnerable, spell_queue) = query.single_mut();
	
	player_state.0 = match player_state.0 {
		PlayerState::Normal | PlayerState::Casting => {
			if spell_queue.is_empty() {
				PlayerState::Normal
			} else {
				PlayerState::Casting
			}
		}
		PlayerState::Knockback => {
			//TODO use a timer
			PlayerState::Normal
		}
	};
}

// Animation stuffs
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum FacingDir {
    Left,
    Right,
}
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AnimationState {
    Idle,
    Walk,
    Casting,
	Knockback,
}
struct AnimationDescription {
	start_index: usize,
	length: usize,
	reversed: bool,
	priority: i32, // knockback > casting > walk = idle
}
// and a function that turns one into the other
fn get_animation_description(facing_dir: FacingDir, anim_state: AnimationState) -> AnimationDescription {
	let priority = match anim_state {
		AnimationState::Idle | AnimationState::Walk => 0,
		AnimationState::Casting => 5,
		AnimationState::Knockback => 10,
	};
	
	let reversed = match facing_dir {
		FacingDir::Left => true,
		FacingDir::Right => false,
	};
	
	let length = match anim_state {
		AnimationState::Idle | AnimationState::Knockback => 1,
		AnimationState::Walk => 8,
		AnimationState::Casting => 3,
	};
	
	let start_index = match (facing_dir, anim_state) {
		(FacingDir::Left, AnimationState::Idle) => 2,
		(FacingDir::Left, AnimationState::Walk) => 16,
		(FacingDir::Left, AnimationState::Casting) => 29,
		(FacingDir::Left, AnimationState::Knockback) => 0,
		(FacingDir::Right, AnimationState::Idle) => 0,
		(FacingDir::Right, AnimationState::Walk) => 8,
		(FacingDir::Right, AnimationState::Casting) => 24,
		(FacingDir::Right, AnimationState::Knockback) => 2,
	};
	
	AnimationDescription {
		start_index,
		length,
		reversed,
		priority,
	}
}

#[derive(Component)]
pub struct PlayerAnimationState {
	facing_dir: FacingDir,
	anim_state: AnimationState,
	index: usize,
}

fn update_player_animation (
    time: Res<Time>,
    mut anim_query: Query<
		(&mut sprite::AnimationTimer, &mut TextureAtlasSprite, &mut PlayerAnimationState),
        With<PlayerSpriteMarker>>,
	player_query: Query<(&CurrentPlayerState, &physics::Speed), With<Player>>,
    spell_ui_active: Res<ui::SpellUiActive>,
) {
	if spell_ui_active.0 {
		return;
	}
	
	let (player_state, player_speed) = player_query.single();
	let (mut timer, mut sprite, mut current_anim_state) = anim_query.single_mut();
	
	timer.tick(time.delta());
	
	let anim_desc = get_animation_description(
		current_anim_state.facing_dir,
		current_anim_state.anim_state,
	);
	
	// Sort of trying to deal with the next state
	// What the facing direction should be
	let next_facing_dir = if player_speed.0.x > 0.1 {
		FacingDir::Right
	} else if player_speed.0.x < -0.1 {
		FacingDir::Left
	} else {
		current_anim_state.facing_dir
	};
	
	// What the animation state should be
	let next_anim_state = match player_state.0 {
		PlayerState::Normal => if player_speed.0.length() > 10.0 {
				AnimationState::Walk
			} else {
				AnimationState::Idle
			}
		PlayerState::Casting => AnimationState::Casting,
		PlayerState::Knockback => AnimationState::Knockback,
	};
	
	let next_anim_desc = get_animation_description(
		next_facing_dir,
		next_anim_state,
	);
	
	// Determine if we need to change animation, and if we should without waiting for the next timer tick
	let change_animation = current_anim_state.facing_dir != next_facing_dir 
		|| current_anim_state.anim_state != next_anim_state;
	let interrupt_immediate = change_animation && anim_desc.priority < next_anim_desc.priority;
		
	if change_animation {
		if timer.just_finished() || interrupt_immediate {
			if interrupt_immediate {
				timer.reset();
			}
			current_anim_state.index = 0;
			current_anim_state.facing_dir = next_facing_dir;
			current_anim_state.anim_state = next_anim_state;
			update_sprite(&mut sprite, current_anim_state.index, next_anim_desc);
		}
	} else {
		// Increment the index if the timer is done
		if timer.just_finished() {
			current_anim_state.index = (current_anim_state.index + 1) % anim_desc.length;
			update_sprite(&mut sprite, current_anim_state.index, anim_desc);
		}
	};
}

/// Updates the TextureAtlasSprite to have the proper texture index
fn update_sprite(sprite: &mut TextureAtlasSprite, index: usize, anim_desc: AnimationDescription) {
	sprite.index = if anim_desc.reversed {
		anim_desc.start_index
			+ (anim_desc.length - index - 1)
	} else {
		anim_desc.start_index + index
	};
}

// Movement
#[derive(Component, Default, Debug)]
struct PlayerSpeed(Vec3);

fn player_movement(
    action_state: Query<&ActionState<Action>, With<Player>>,
    mut player_query: Query<(&mut physics::Speed, &CurrentPlayerState), With<Player>>,
    time: Res<Time>,
    spell_ui_active: Res<ui::SpellUiActive>,
) {
    if spell_ui_active.0 {
        return;
    }
    let action_state = action_state.single();
    let (mut speed, player_state) = player_query.single_mut();

	if player_state.0 != PlayerState::Knockback {
		let mut total_offset = Vec2::splat(0.0);
		
		if player_state.0 == PlayerState::Normal {
			if action_state.pressed(Action::Up) {
				total_offset.y -= 1.0;
			}
			if action_state.pressed(Action::Down) {
				total_offset.y += 1.0;
			}
			if action_state.pressed(Action::Right) {
				total_offset.x += 1.0;
			}
			if action_state.pressed(Action::Left) {
				total_offset.x -= 1.0;
			}
		}

		// Update speed
		let target_speed = total_offset.normalize_or_zero() * SPEED;

		speed.0.x = update_speed(speed.0.x, target_speed.x, time.delta_seconds());
		speed.0.y = update_speed(speed.0.y, target_speed.y, time.delta_seconds());
	}
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

// Spellcasting
fn update_spell_casting(
    mut query: Query<(&Transform, &ActionState<Action>, &CurrentPlayerState, &mut spells::RuneCastQueue), With<Player>>,
    anim_query: Query<&PlayerAnimationState, With<PlayerSpriteMarker>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    equipped: Res<spells::EquippedRunes>,
    spell_ui_active: Res<ui::SpellUiActive>,
    ui_mouse_target: Res<ui::CurrentMouseoverTarget>,
    windows: Res<Windows>,
	mut create_spell_events: EventWriter<spells::CreateSpellEvent>,
) {
    // Don't do anything if the spell UI is open
    if spell_ui_active.0 {
        return;
    }

    let (transform, action_state, player_state, mut spell_queue) = query.single_mut();

	if player_state.0 == PlayerState::Knockback {
		spell_queue.clear();
		return;
	}
	
    // Find which ones to add
    for (idx, comp_action) in SPELL_COMP_ACTIONS.iter().enumerate() {
        if action_state.just_pressed(*comp_action) {
            if let Some(Some(rune)) = equipped.0.get(idx as usize) {
                spell_queue.push(*rune);
            } else {
                println!("No component available to add")
            }
        }
    }

    // Check if we want to cast a spell (and aren't clicking on UI)
    if ui_mouse_target.0.is_none() && action_state.just_pressed(Action::CastSpell) {
		if let Some(spell_data) = spell_queue.generate_spell() {
			// Figure out where the mouse is pointing
			let offset = Vec3::new(0.0, 16.0, 0.0);
			let (camera, camera_transform) = camera_query.single();
			let anim_state = anim_query.single();

			let maybe_world_mouse_position = ui::get_cursor_world_position(
				windows,
				camera,
				camera_transform,
				offset,
				Vec3::Y,
			);

			let maybe_aim_dir = match maybe_world_mouse_position {
				Some(mouse_pos) => {
					println!("{:?}", mouse_pos);
					(mouse_pos - transform.translation).try_normalize()
				}
				None => None,
			};

			let aim_dir = match maybe_aim_dir {
				Some(aim_dir) => collapse_vec3(aim_dir),
				None => match anim_state.facing_dir {
					FacingDir::Right => Vec2::new(1.0, 0.0),
					FacingDir::Left => Vec2::new(-1.0, 0.0),
				},
			};
			
			let start_pos = collapse_vec3(transform.translation) + 12.0 * aim_dir;
			
			create_spell_events.send(spells::CreateSpellEvent {
				spell_data,
				position: start_pos,
				move_direction: aim_dir,
			});
		}
		spell_queue.clear();
    } else if action_state.just_pressed(Action::CancelSpell) {
		spell_queue.clear();
	}
}

// Input handling
#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub enum Action {
    Left,
    Right,
    Up,
    Down,
    Run,
    Dodge,
    CastSpell,
    CancelSpell,
    OpenInventory,
    SpellComp0,
    SpellComp1,
    SpellComp2,
    SpellComp3,
    SpellComp4,
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
        // Spells
        (KeyCode::Tab, Action::OpenInventory),
        (KeyCode::Key1, Action::SpellComp0),
        (KeyCode::Key2, Action::SpellComp1),
        (KeyCode::Key3, Action::SpellComp2),
        (KeyCode::Key4, Action::SpellComp3),
        (KeyCode::E, Action::SpellComp4),
    ])
    .insert(MouseButton::Left, Action::CastSpell)
    .insert(MouseButton::Right, Action::CancelSpell)
    .build()
}

const SPELL_COMP_ACTIONS: [Action; 5] = [
    Action::SpellComp0,
    Action::SpellComp1,
    Action::SpellComp2,
    Action::SpellComp3,
    Action::SpellComp4,
];
