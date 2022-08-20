
use bevy::{
	prelude::*,
	transform::transform_propagate_system
};

#[derive(Component, Debug)]
pub struct FacingSpriteMarker;
#[derive(Component, Debug)]
pub struct SpriteOffset(pub Vec3);

/// Sets up system for making sprites face the camera properly.
/// Note that they need a FacingSpriteMarker for this.
pub struct FacingSpritePlugin;
impl Plugin for FacingSpritePlugin {
	fn build(&self, app: &mut App) {
		app.add_system_to_stage(
				CoreStage::PostUpdate, 
				facing_sprite_update.before(transform_propagate_system)
			);
	}
}


// Make sprites look nice in our sort-of-3d environment
fn facing_sprite_update(
	parent_query: Query<&Transform, (Without<FacingSpriteMarker>, Without<Camera>)>,
	mut sprite_query: Query<(&mut Transform, &Parent, Option<&SpriteOffset>),
		(With<FacingSpriteMarker>, Without<Camera>)>,
	camera_query: Query<&Transform, 
		(With<Camera>, Without<FacingSpriteMarker>)>,
) {
	let camera_transform = camera_query.single();
	let camera_inverse = Transform::from_matrix(
		camera_transform.compute_matrix().inverse()
	);
	
	for (mut sprite_transform, parent, maybe_offset) in sprite_query.iter_mut() {
		if let Ok(parent_transform) = parent_query.get(parent.get()) {
			let parent_position = parent_transform.translation;
			let sprite_offset = match maybe_offset {
				Some(SpriteOffset(o)) => *o,
				None => Vec3::ZERO,
			};
			
			// First we need to transform everything w.r.t the camera
			let parent_camera_loc = camera_inverse * (parent_position + camera_transform.rotation * sprite_offset);
			
			// Then, we want to set the sprite to be pixel-aligned
			let target_position = parent_camera_loc.round();
			
			// Then we adjust sprite positioning as needed
			sprite_transform.rotation = camera_transform.rotation;
			sprite_transform.translation = (*camera_transform * target_position) - parent_position;
		}
	}
}