use super::{ui, levels, expand_vec2, collapse_vec3};
use bevy::{
	prelude::*,
	transform::transform_propagate_system,
};
use std::marker::{Send, Sync, PhantomData};
use std::ops::{Deref, DerefMut};
use itertools::Itertools;

pub struct GeneralPhysicsPlugin;

impl Plugin for GeneralPhysicsPlugin {
    fn build(&self, app: &mut App) {
        app
			.add_system(update_movement)
			.add_system(do_takes_space_collisions.after(update_movement))
			.add_system(do_wall_collisions.after(do_takes_space_collisions))
			.add_plugin(CollisionPlugin::<WallCollidable>::default())
			.add_plugin(CollisionPlugin::<InteractsWithPlayer>::default())
			.add_plugin(CollisionPlugin::<InteractsWithEnemies>::default())
			.add_plugin(SymmetricCollisionPlugin::<TakesSpace>::default());
    }
}



// Specific collision categories
#[derive(Default)]
pub struct WallCollidable;
#[derive(Default)]
pub struct InteractsWithPlayer;
#[derive(Default)]
pub struct InteractsWithEnemies;
// symmetric for player and enemies; prevents occupying same space
#[derive(Default)]
pub struct TakesSpace;

// Walls and TakesSpace collisions
#[derive(Bundle)]
pub struct Wall {
	collision: CollisionSource<WallCollidable>,
	in_direction: WallInsideDirection,
	marker: levels::CleanUpOnRoomLoad,
}
// Normal vector of the wall
#[derive(Component)]
struct WallInsideDirection(Vec2);
impl Wall {
	pub fn new(point1: Vec2, point2: Vec2, rhs_inside: bool) -> Self {
		let wall_tangent = (point2 - point1).try_normalize().expect("degenerate wall attempted to be created");
		
		let wall_normal = wall_tangent.perp() * (if rhs_inside {
			1.0
		} else {
			-1.0
		});
		
		Self {
			collision: CollisionSource::<WallCollidable>::new(Collider::LineSegment(
				point1,
				point2
			)),
			in_direction: WallInsideDirection(wall_normal),
			marker: levels::CleanUpOnRoomLoad
		}
	}
}

fn do_wall_collisions (
	wall_query: Query<&WallInsideDirection, Without<CollisionRecipient<WallCollidable>>>,
	mut recip_query: Query<&mut Transform, With<CollisionRecipient<WallCollidable>>>,
	collisions: Res<ActiveCollisions<WallCollidable>>
) {
	for collision in collisions.iter() {
		// Get collidees from queries
		if let (Ok(wall), Ok(mut other_transform)) = (
			wall_query.get(collision.source_entity),
			recip_query.get_mut(collision.recip_entity),
		) {
			// Get transformed collider; recipient must be a circle, source is always LineSegment
			if let (
				Collider::Circle {center: real_center, radius},
				Collider::LineSegment(point1, _point2)
			) = (
				&collision.recip_collider,
				&collision.source_collider,
			) {
				// Check the current projected distance
				let recip_rel_pos = *real_center - *point1;
				let dist = wall.0.dot(recip_rel_pos);
				if dist < *radius {
					other_transform.translation += expand_vec2(wall.0 * (*radius - dist));
				}
			} else {
				panic!("invalid WallCollidable recipient Collider; must be Collider::Circle");
			}
		}
	}
}


fn do_takes_space_collisions(
	mut pos_query: Query<&mut Transform, With<SymmetricCollisionSource<TakesSpace>>>,
	collisions: Res<ActiveCollisions<TakesSpace>>,
) {
	for collision in collisions.iter() {
		// Get the colliders
		if let (Collider::Circle {
			center: _,
			radius: radius1,
		}, Collider::Circle {
			center: _,
			radius: radius2,
		}) = (&collision.source_collider, &collision.recip_collider) {
			// Get the transforms
			if let Ok([mut transform1, mut transform2]) = 
				pos_query.get_many_mut([collision.source_entity, collision.recip_entity])
			 {
				let pos_diff_vec = collapse_vec3(transform1.translation)
					- collapse_vec3(transform2.translation);
				let rad_sum = radius1 + radius2;
				if rad_sum > pos_diff_vec.length() {
					// They're (still) too close; move them apart
					let move_dist = (rad_sum - pos_diff_vec.length()) / 2.0;
					let dir = if let Some(d) = pos_diff_vec.try_normalize() {
						d
					} else {
						Vec2::X
					};
					
					let move_vec = expand_vec2(move_dist * dir);
					
					transform1.translation += move_vec;
					transform2.translation -= move_vec;
				}
			}
		} else {
			panic!("TakesSpace collision type must use Circle type collider")
		}
	}
}

#[derive(Component, Deref, DerefMut, Debug)]
pub struct Speed(pub Vec2);

// Movement should only be updated if menu is not open
pub fn update_movement(
    time: Res<Time>,
    mut query: Query<(&Speed, &mut Transform)>,
    spell_ui_active: Res<ui::SpellUiActive>,
) {
    if spell_ui_active.0 {
        return;
    }
    for (speed, mut transform) in query.iter_mut() {
        // Update position
        transform.translation += expand_vec2(speed.0 * time.delta_seconds());
    }
}

// Collision stuff
#[derive(Debug, Clone)]
pub enum Collider {
	Circle {
		center: Vec2,
		radius: f32,
	},
	LineSegment(Vec2, Vec2),
	LineRay {
		anchor: Vec2,
		direction: Vec2,
	},
	ThickLineSegment {
		point1: Vec2, 
		point2: Vec2, 
		thickness:f32
	},
}

fn transform_in_plane(point: Vec2, transform: &Transform) -> Vec2 {
	let pos_3d = Vec3::new(point.x, 0.0, point.y);
	let tr_pos = *transform * pos_3d;
	Vec2::new(tr_pos.x, tr_pos.z)
}
fn rotate_in_plane(point: Vec2, transform: &Transform) -> Vec2 {
	let pos_3d = Vec3::new(point.x, 0.0, point.y);
	let tr_pos = transform.rotation * pos_3d;
	Vec2::new(tr_pos.x, tr_pos.z)
}

impl Collider {
	/// Applies a transformation to the given collider.
	/// Scaling is not supported (although it probably behaves somewhat ok sometimes)
	fn with_transform(&self, transform: &Transform) -> Self {
		match *self {
			Self::Circle { center, radius } =>
				Self::Circle {
					center: transform_in_plane(center, transform),
					radius,
				},
			Self::LineSegment(point1, point2) => 
				Self::LineSegment(
					transform_in_plane(point1, transform),
					transform_in_plane(point2, transform)
				),
			Self::LineRay {anchor, direction} => 
				Self::LineRay {
					anchor: transform_in_plane(anchor, transform),
					direction: rotate_in_plane(direction, transform)
				},
			Self::ThickLineSegment { point1, point2, thickness } => 
				Self::ThickLineSegment{
					point1: transform_in_plane(point1, transform),
					point2: transform_in_plane(point2, transform),
					thickness 
				},
		}
	}
	
	/// Tests if two colliders intersect.
	fn intersects(&self, other: &Collider) -> bool {
		match self {
			Self::Circle {center: c1, radius: r1} => {
				match other {
					Self::Circle {center: c2, radius: r2} => { 
						let rad_sum_sq = (r1+r2) * (r1+r2);
						c1.distance_squared(*c2) <= rad_sum_sq
					},
					Self::LineSegment(point1, point2) => { 
						let rad_sq = r1 * r1;
						
						point1.distance_squared(*c1) <= rad_sq
						|| point2.distance_squared(*c1) <= rad_sq
						|| {
							let cir_rel_pos = *c1 - *point1;
							let line_vec = *point2 - *point1;
							let line_dir = line_vec.normalize_or_zero();
							let line_length = line_vec.length();
							
							let cir_rej = cir_rel_pos.reject_from_normalized(line_dir);
							let cir_proj_dot = cir_rel_pos.dot(line_dir);
							
							// Determine if close enough
							cir_rej.length_squared() <= rad_sq
							// Determine if the projection of the point lies on the segment
							&& cir_proj_dot >= 0.0
							&& cir_proj_dot <= line_length
						}
					},
					Self::LineRay { anchor, direction } => {
						let rad_sq = r1 * r1;
						
						anchor.distance_squared(*c1) <= rad_sq
						|| {
							let cir_rel_pos = *c1 - *anchor;
							
							let cir_rej = cir_rel_pos.reject_from(*direction);
							let cir_proj_dot = cir_rel_pos.dot(*direction);
							
							// Determine if close enough
							cir_rej.length_squared() <= rad_sq
							// Determine if the projection of the point lies on the ray
							&& cir_proj_dot >= 0.0
						}
					}
					Self::ThickLineSegment { point1, point2, thickness } => { 
						let max_dist_sq = (r1 + thickness) * (r1 + thickness);
						
						// Only test based on projection
						let cir_rel_pos = *c1 - *point1;
						let line_vec = *point2 - *point1;
						let line_dir = line_vec.normalize_or_zero();
						let line_length = line_vec.length();
						
						let cir_rej = cir_rel_pos.reject_from_normalized(line_dir);
						let cir_proj_dot = cir_rel_pos.dot(line_dir);
						
						// Determine if close enough
						cir_rej.length_squared() <= max_dist_sq
						// Determine if the projection of the point lies on the segment
						&& cir_proj_dot >= 0.0
						&& cir_proj_dot <= line_length
					},
				}
			},
			_ => match other {
				Self::Circle {..} => {
					other.intersects(self)
				},
				_ => panic!("intersection testing that does not involve a circle is unsupported")
			}
		}
	}
}

// Generics time
#[derive(Component, Debug)]
pub struct CollisionSource<T>(pub Collider, PhantomData<T>);
#[derive(Component, Debug)]
pub struct CollisionRecipient<T>(pub Collider, PhantomData<T>);
#[derive(Component, Debug)]
pub struct SymmetricCollisionSource<T>(pub Collider, PhantomData<T>);
#[derive(Component)]
pub struct ColliderActive<T>(pub bool, PhantomData<T>);

// trait to compactify some things
trait HasCollider {
	fn collider(&self) -> &Collider;
}
impl<T> HasCollider for CollisionSource<T> {
	fn collider(&self) -> &Collider {
		&self.0
	}
}
impl<T> HasCollider for CollisionRecipient<T> {
	fn collider(&self) -> &Collider {
		&self.0
	}
}
impl<T> HasCollider for SymmetricCollisionSource<T> {
	fn collider(&self) -> &Collider {
		&self.0
	}
}

// constructors
impl<T> CollisionSource<T> {
	pub fn new(collider: Collider) -> Self {
		Self(collider, Default::default())
	}
}
impl<T> CollisionRecipient<T> {
	pub fn new(collider: Collider) -> Self {
		Self(collider, Default::default())
	}
}
impl<T> SymmetricCollisionSource<T> {
	pub fn new(collider: Collider) -> Self {
		Self(collider, Default::default())
	}
}
impl<T> ColliderActive<T> {
	pub fn new(active: bool) -> Self {
		Self(active, Default::default())
	}
}

/// Stores collision data
/// Used for both directed and symmetric collision cases
/// For symmetric, ordering is arbitrary
#[derive(Debug)]
pub struct Collision {
	pub source_entity: Entity,
	pub source_collider: Collider,
	pub recip_entity: Entity,
	pub recip_collider: Collider,
}

/// Resource for holding collisions.
/// Updated each frame in CoreStage::PreUpdate
#[derive(Debug)]
pub struct ActiveCollisions<T>(pub Vec<Collision>, PhantomData<T>);

impl<T> ActiveCollisions<T> {
	fn new() -> ActiveCollisions<T> {
		ActiveCollisions::<T>(
			Vec::<Collision>::new(), 
			Default::default()
		)
	}
}

impl<T> Deref for ActiveCollisions<T> {
	type Target = Vec<Collision>;
	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<T> DerefMut for ActiveCollisions<T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}


// Plugins
#[derive(Default)]
pub struct CollisionPlugin<T>(PhantomData<T>);
impl<T: Send + Sync + 'static> Plugin for CollisionPlugin<T> {
    fn build(&self, app: &mut App) {
        app
			.insert_resource(ActiveCollisions::<T>::new())
			.add_system_to_stage(CoreStage::PostUpdate, resolve_collisions::<T>.after(transform_propagate_system));
    }
}

pub fn resolve_collisions<T: Send + Sync + 'static> (
	source_query: Query<(Entity, &CollisionSource<T>, Option<&GlobalTransform>, Option<&ColliderActive<T>>)>,
	recip_query: Query<(Entity, &CollisionRecipient<T>, Option<&GlobalTransform>, Option<&ColliderActive<T>>)>,
	mut collisions: ResMut<ActiveCollisions<T>>
) {
	collisions.clear();
	
	let source_iter = process_collision_query(source_query);
	let recip_iter = process_collision_query(recip_query);
	
	
	for (source_entity, source_collider) in source_iter {
		for (recip_entity, recip_collider) in &recip_iter {
			if source_collider.intersects(recip_collider) {
				collisions.push(Collision {
					source_entity,
					source_collider: source_collider.clone(),
					recip_entity: *recip_entity,
					recip_collider: recip_collider.clone()
				});
			}
		}
	}
}


#[derive(Default)]
pub struct SymmetricCollisionPlugin<T>(PhantomData<T>);
impl<T: Send + Sync + 'static> Plugin for SymmetricCollisionPlugin<T> {
    fn build(&self, app: &mut App) {
        app
			.insert_resource(ActiveCollisions::<T>::new())
			.add_system_to_stage(CoreStage::PostUpdate, resolve_collisions_symmetric::<T>.after(transform_propagate_system));
    }
}

fn resolve_collisions_symmetric<T: Send + Sync + 'static> (
	sources_query: Query<(Entity, &SymmetricCollisionSource<T>, Option<&GlobalTransform>, Option<&ColliderActive<T>>)>,
	mut collisions: ResMut<ActiveCollisions<T>>
) {
	collisions.clear();
	
	let sources_iter = process_collision_query(sources_query);
	
	for pair in sources_iter.iter().combinations(2) {
		if let (
			Some((entity1, collider1)), 
			Some((entity2, collider2))
		) = (pair.first(), pair.last()) {
			if collider1.intersects(collider2) {
				collisions.push(Collision {
					source_entity: *entity1,
					source_collider: collider1.clone(),
					recip_entity: *entity2,
					recip_collider: collider2.clone()
				});
			}
		}
	}
}


// Utility function for processing collision queries
fn process_collision_query<T: Send+Sync+'static, U: HasCollider+Component>(
	query: Query<(Entity, &U, Option<&GlobalTransform>, Option<&ColliderActive<T>>)>
) -> Vec<(Entity, Collider)> {
	query
		.iter()
		.filter(|(_,_,_,s)| match s {
			Some(active) => active.0,
			None => true
		})
		.map(|(e, collider_container, maybe_transform, _)| {
			let collider_orig = collider_container.collider();
			
			let collider = match maybe_transform {
				Some(transform) => collider_orig.with_transform(&transform.compute_transform()),
				None => collider_orig.clone()
			};
			
			(e, collider)
		})
		.collect()
}