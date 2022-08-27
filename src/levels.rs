
use bevy::prelude::*;
use bevy_turborand::*;
use super::{enemy, sprite, physics, player, ui};

pub struct LevelsPlugin;
impl Plugin for LevelsPlugin {
	fn build(&self, app: &mut App) {
		app
			.insert_resource(CurrentRoom(None))
			.add_event::<RoomTransitionEvent>()
			.add_system(transition_to_room)
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

use ui::{MessageTrigger, MessageEvent, MessageSource, MessageTriggerType};
use sprite::*;
use enemy::*;
use physics::*;

// Transition function.
// Soon will be absolutely atrociously long.
fn transition_to_room(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut transition_events: EventReader<RoomTransitionEvent>,
	mut current_room: ResMut<CurrentRoom>,
	cleanup_query: Query<Entity, With<CleanUpOnRoomLoad>>,
	// Things needed to spawn the destination room
	mut message_events: EventWriter<MessageEvent>,
	mut global_rng: ResMut<GlobalRng>,
	enemy_textures: Res<enemy::EnemySprites>,
	shadow_texture: Res<sprite::ShadowTexture>
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
				(index, false)
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
		// What kind of things might we want to return from this match statement?
		// - Camera boundaries (x1, x2)
		// - Player start position (x,y)
		// - Camera start position (x) 
		match room_index {
			0 => {
				// starting room
				// The staff
				commands.spawn()
					.insert(CollisionSource::<InteractsWithPlayer>::new(Collider::Circle {
						center: Vec2::ZERO,
						radius: 12.0,
					}))
					.insert(PlayerInteraction::GiveStaff)
					.insert_bundle(SpatialBundle {
						transform: Transform::from_translation(Vec3::new(100.0, 0.0, 0.0)),
						..default()
					})
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
				// sorry this is mildly atrocious but I didn't want it wandering five tabs to the right
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
					next_message: None
					}))
					}))
					}))
					}))
					}))
				});
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
						SpatialBundle {
							transform: Transform::from_translation(Vec3::new(60.0, 0.0, 0.0)),
							..default()
						},
						&mut global_rng
					)).with_children(|parent| {
						parent.spawn_bundle(SimpleAnimationBundle::new(
							enemy_textures.get_sprite("spiky"), 
							20.0,
							false
						));
						parent.spawn_bundle(shadow_texture.get_shadow_bundle(2));
					});
			}
			_ => panic!("attempted to transition to non-existent room {}", room_index)
		}
		
		current_room.0 = Some(room_index);
	}
}
