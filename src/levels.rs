
use bevy::prelude::*;
use bevy_turborand::*;
use super::{enemy, sprite, physics};

pub struct LevelsPlugin;
impl Plugin for LevelsPlugin {
	fn build(&self, app: &mut App) {
		app
			.insert_resource(CurrentRoom(None))
			.add_event::<RoomTransitionEvent>()
			.add_system(transition_to_room);
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

// Transition function.
// Soon will be absolutely atrociously long.
fn transition_to_room(
	mut commands: Commands,
	mut transition_events: EventReader<RoomTransitionEvent>,
	mut current_room: ResMut<CurrentRoom>,
	cleanup_query: Query<Entity, With<CleanUpOnRoomLoad>>,
	// Things needed to spawn the destination room
	mut global_rng: ResMut<GlobalRng>,
	enemy_textures: Res<enemy::EnemySprites>,
	shadow_texture: Res<sprite::ShadowTexture>
) {
	if let Some(transition_event) = transition_events.iter().next() {	
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
		
		
		// Do spawning
		// Might be nicer to serialize
		match room_index {
			0 => {
				// starting room
				// should have staff
				//unimplemented!()
			}
			1 => {
				// testing-ish
				commands
					.spawn_bundle(enemy::EnemyBundle::<enemy::AIPeriodicCharge>::new(
						100, 
						2, 
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
						parent.spawn_bundle(sprite::SimpleAnimationBundle::new(
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
