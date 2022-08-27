
use bevy::{
	prelude::*,
	utils::HashMap
};
use bevy_turborand::*;
use super::{enemy, sprite, spells, physics, player, ui, expand_vec2};

pub struct LevelsPlugin;
impl Plugin for LevelsPlugin {
	fn build(&self, app: &mut App) {
		app
			.insert_resource(CurrentRoom(None))
			.add_startup_system(load_level_sprites)
			.add_event::<RoomTransitionEvent>()
			.add_system(transition_to_room)
			.add_system(update_gate)
			.add_system(do_player_interaction);
	}
}

#[derive(Component)]
pub struct CleanUpOnRoomLoad;
// Event for triggering a room transition
pub struct RoomTransitionEvent(pub DestinationRoom);
pub enum DestinationRoom {
	NextRoom,
	TargetRoom {
		target: usize,
		respawn: bool,
	},
}
// Resource to store the current room
struct CurrentRoom(Option<usize>);

// Things in the game environment that can interact with the player.
#[derive(Component)]
pub enum PlayerInteraction {
	GiveStaff,
	RoomTransition,
}

fn do_player_interaction(
	mut commands: Commands,
	interact_query: Query<(Entity, &PlayerInteraction)>,
	collisions: Res<physics::ActiveCollisions<physics::InteractsWithPlayer>>,
	// For doing the interactions
	mut staff_events: EventWriter<player::GiveStaffEvent>,
	mut transition_events: EventWriter<RoomTransitionEvent>,
) {	
	for collision in collisions.iter() {
		if let Ok((e, interaction)) = interact_query.get(collision.source_entity) {
			match interaction {
				PlayerInteraction::GiveStaff => {
					staff_events.send(player::GiveStaffEvent);
					commands.entity(e).despawn_recursive();
				},
				PlayerInteraction::RoomTransition => {
					transition_events.send(RoomTransitionEvent(DestinationRoom::NextRoom));
					commands.entity(e).despawn_recursive();
				}
			}
		}
	}
}

// Gate that opens if there are no enemies
#[derive(Component)]
pub struct GateMarker;

fn update_gate(
	mut commands: Commands,
	gate_query: Query<Entity, With<GateMarker>>,
	enemy_query: Query<(), With<enemy::EnemyMarker>>,
) {
	if enemy_query.is_empty() {
		for e in gate_query.iter() {
			commands.entity(e).despawn_recursive();
		}
	}
}


#[derive(Deref, DerefMut)]
pub struct LevelSprites(pub HashMap<String, Handle<Image>>);
impl LevelSprites {
	pub fn get_sprite(&self, key: &str) -> Handle<Image> {
		self.0.get(&key.to_string()).expect("invalid enemy sprite key encountered").clone()
	}
}
fn load_level_sprites(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
) {
	let handles = [
		("gate", "level/gate.png"),
	].iter()
		.map(|(key, path)| {
			(key.to_string(), asset_server.load(*path))
		})
		.collect();
	
	commands.insert_resource(LevelSprites(handles));
}

use ui::{MessageTrigger, MessageEvent, MessageSource, MessageTriggerType};
use sprite::*;
use enemy::*;
use physics::*;
use spells::*;
use player::*;

fn at_location(x: f32, y: f32) -> SpatialBundle {
	at_location_vec(Vec2::new(x,y))
}
fn at_location_vec(loc: Vec2) -> SpatialBundle {
	SpatialBundle {
		transform: Transform::from_translation(expand_vec2(loc)),
		..default()
	}
}

// Transition function.
// Soon will be absolutely atrociously long.
fn transition_to_room(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut transition_events: EventReader<RoomTransitionEvent>,
	mut current_room: ResMut<CurrentRoom>,
	cleanup_query: Query<Entity, With<CleanUpOnRoomLoad>>,
	// Things needed for setup
	mut player_query: Query<&mut Transform, With<Player>>,
	mut camera_bounds: ResMut<CameraBounds>,
	mut clear_color: ResMut<ClearColor>,
	// Things needed to spawn the destination room
	mut message_events: EventWriter<MessageEvent>,
	mut global_rng: ResMut<GlobalRng>,
	level_textures: Res<LevelSprites>,
	enemy_textures: Res<EnemySprites>,
	shadow_texture: Res<ShadowTexture>,
	spell_textures: Res<AllSpellSprites>,
) {
	let maybe_transition_event = if current_room.0.is_none() {
		// Go to room 0 at the start
		Some(&RoomTransitionEvent(DestinationRoom::TargetRoom {
			target: 0,
			respawn: false
		}))
	} else {
		transition_events.iter().next()
	};
	
	if let Some(transition_event) = maybe_transition_event {	
		// Clean up from previous room 
		for entity in cleanup_query.iter() {
			commands.entity(entity).despawn_recursive();
		}
		
		// Figure out room index
		let (room_index, respawn) = match &transition_event.0 {
			DestinationRoom::NextRoom => if let Some(index) = current_room.0 {
				(index + 1, false)
			} else {
				panic!("cannot transition to next room if not in a room")
			},
			DestinationRoom::TargetRoom {target, respawn} => (*target, *respawn),
		};
		
		println!("entering room {}", room_index);
		message_events.send(MessageEvent {
			message: None,
			source: MessageSource::ForceClear,
		});
		
		// Do spawning
		// TIP: Alt+5 does a good code folding level for this to actually be readable
		// What kind of things might we want to return from this match statement?
		// - Camera boundaries (x1, x2)
		// - Player start position (x,y)
		// - screen clear color
		let (new_player_pos, (cam_min_x, cam_max_x), new_clear_color) = match room_index {
			0 => {
				// starting room
				// The staff
				commands.spawn()
					.insert(CollisionSource::<InteractsWithPlayer>::new(Collider::Circle {
						center: Vec2::ZERO,
						radius: 12.0,
					}))
					.insert(PlayerInteraction::GiveStaff)
					.insert_bundle(at_location(100.0,0.0))
					.with_children(|parent| {
						parent.spawn_bundle(FacingSpriteBundle::new(
							asset_server.load("player/staff.png"),
							22.0
						));
						parent.spawn_bundle(shadow_texture.get_shadow_bundle(0));
					});
				// Set up some tutorial messages
				message_events.send(MessageEvent {
					message: Some("You begin your quest to reach the Tower of the Moon.\nUse WASD to walk.".to_string()),
					source: MessageSource::Tutorial,
				});
				// (sorry this is mildly atrocious but I didn't want it wandering five tabs to the right)
				commands.spawn().insert(MessageTrigger {
					message_event: MessageEvent {
						message: Some("Walk to the right and pick up your staff.".to_string()),
						source: MessageSource::Tutorial,
					},
					trigger_type: MessageTriggerType::OnMove,
					next_message: Some(Box::new(MessageTrigger {
					///////////////////////////////////////////////////////////////////////
					message_event: MessageEvent {
						message: Some("With your staff, you can cast spells. But first, you must equip runes.\nPress TAB to open your inventory.".to_string()),
						source: MessageSource::Tutorial,
					},
					trigger_type: MessageTriggerType::OnCollectStaff,
					next_message: Some(Box::new(MessageTrigger {
					///////////////////////////////////////////////////////////////////////
					message_event: MessageEvent {
						message: Some("Hover the mouse over a rune and press 1-4 or E to equip.".to_string()),
						source: MessageSource::Tutorial,
					},
					trigger_type: MessageTriggerType::OnSpellUi(true),
					next_message: Some(Box::new(MessageTrigger {
					///////////////////////////////////////////////////////////////////////
					message_event: MessageEvent {
						message: Some("Once you are done selecting runes, press TAB again to close your inventory.".to_string()),
						source: MessageSource::Tutorial,
					},
					trigger_type: MessageTriggerType::OnRuneEqipped,
					next_message: Some(Box::new(MessageTrigger {
					///////////////////////////////////////////////////////////////////////
					message_event: MessageEvent {
						message: Some("Use 1-4 and E to prepare runes. When you are ready, LEFT CLICK to cast\nyour spell.\nExtinguish the flames to open the gate.".to_string()),
						source: MessageSource::Tutorial,
					},
					trigger_type: MessageTriggerType::OnSpellUi(false),
					next_message: Some(Box::new(MessageTrigger {
					///////////////////////////////////////////////////////////////////////
					message_event: MessageEvent {
						message: None,
						source: MessageSource::Tutorial,
					},
					trigger_type: MessageTriggerType::OnSpellCast,
					next_message: Some(Box::new(MessageTrigger {
					///////////////////////////////////////////////////////////////////////
					message_event: MessageEvent {
						message:  Some("Enter the gate to proceed to the next area.".to_string()),
						source: MessageSource::Tutorial,
					},
					trigger_type: MessageTriggerType::OnGateOpened,
					next_message: None
					}))
					}))
					}))
					}))
					}))
					}))
				});
				
				// Gate
				commands.spawn()
					.insert(GateMarker)
					.insert_bundle(at_location(0.0, -50.0))
					.insert(CleanUpOnRoomLoad)
					.with_children(|parent| {
						parent.spawn_bundle(FacingSpriteBundle::new(
							level_textures.get_sprite("gate"),
							32.0
						));
					});
				
				// "enemies"
				commands
					.spawn_bundle(EnemyBundle::<NoAI>::with_state(
						NoAI,
						1, 
						1, 
						0.0,
						Collider::Circle {
							center: Vec2::ZERO,
							radius: 8.0
						}, 
						at_location(-32.0,-40.0),
						&mut global_rng
					)).with_children(|parent| {
						parent.spawn_bundle(SimpleAnimationBundle::new(
							spell_textures.get_atlas_from_type(SpellElement::Fire, SpellSize::Large), 
							20.0,
							false
						));
						parent.spawn_bundle(shadow_texture.get_shadow_bundle(2));
					});
				commands
					.spawn_bundle(EnemyBundle::<NoAI>::with_state(
						NoAI,
						1, 
						1, 
						0.0,
						Collider::Circle {
							center: Vec2::ZERO,
							radius: 8.0
						}, 
						at_location(32.0,-40.0),
						&mut global_rng
					)).with_children(|parent| {
						parent.spawn_bundle(SimpleAnimationBundle::new(
							spell_textures.get_atlas_from_type(SpellElement::Fire, SpellSize::Large), 
							20.0,
							false
						));
						parent.spawn_bundle(shadow_texture.get_shadow_bundle(2));
					});
				
				// Room transitions
				commands.spawn()
					.insert(CollisionSource::<InteractsWithPlayer>::new(Collider::LineSegment(
						Vec2::new(-32.0,0.0),
						Vec2::new(32.0,0.0),
					)))
					.insert(PlayerInteraction::RoomTransition)
					.insert(CleanUpOnRoomLoad)
					.insert_bundle(at_location(0.0,-72.0));
				
				(
					Vec2::new(-50.0, 0.0),
					(0.0, 0.0),
					Color::hex("75A743").unwrap()
				)
			}
			1 => {
				// testing-ish
				commands
					.spawn_bundle(EnemyBundle::<AIPeriodicCharge>::new(
						100, 
						2, 
						Collider::Circle {
							center: Vec2::ZERO,
							radius: 8.0
						}, 
						at_location(60.0,0.0),
						&mut global_rng
					)).with_children(|parent| {
						parent.spawn_bundle(SimpleAnimationBundle::new(
							enemy_textures.get_sprite("spiky"), 
							20.0,
							false
						));
						parent.spawn_bundle(shadow_texture.get_shadow_bundle(2));
					});
				
				(
					Vec2::new(-50.0, 0.0),
					(0.0, 160.0),
					Color::hex("75A743").unwrap()
				)
			}
			_ => panic!("attempted to transition to non-existent room {}", room_index)
		};
		
		// Update player position
		let mut player_transform = player_query.single_mut();
		player_transform.translation = expand_vec2(new_player_pos);
		
		// Update camera bounds
		camera_bounds.min_x = cam_min_x;
		camera_bounds.max_x = cam_max_x;
		
		// Update clear color
		clear_color.0 = new_clear_color;
		
		current_room.0 = Some(room_index);
	}
}
