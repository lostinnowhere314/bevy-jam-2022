use super::ui;
use bevy::{prelude::*, transform::transform_propagate_system, utils::Duration};

#[derive(Component, Debug, Default)]
pub struct FacingSpriteMarker;
#[derive(Component, Debug)]
pub struct SpriteOffset(pub Vec3);

/// Sets up system for making sprites face the camera properly.
/// Note that they need a FacingSpriteMarker for this.
pub struct FacingSpritePlugin;
impl Plugin for FacingSpritePlugin {
    fn build(&self, app: &mut App) {
        app
			.add_startup_system_to_stage(StartupStage::PreStartup, shadow_setup)
			.add_system_to_stage(
				CoreStage::PostUpdate,
				facing_sprite_update.before(transform_propagate_system),
			)
			.add_system(simple_animation_update)
			.add_system(hover_update)
			.add_system_to_stage(CoreStage::PreUpdate, pause_animation_timers);
    }
}


// Shadows
pub struct ShadowTexture(Handle<TextureAtlas>);
#[derive(Bundle)]
pub struct ShadowTextureBundle {
	marker: FacingSpriteMarker,
	sprite_offset: SpriteOffset,
	#[bundle]
	sprite_bundle: SpriteSheetBundle,
}

impl ShadowTexture {
	pub fn get_shadow_bundle(&self, index: usize) -> ShadowTextureBundle {
		ShadowTextureBundle {
			marker: FacingSpriteMarker,
			sprite_offset: SpriteOffset(Vec3::new(0.0, 0.0, -16.0)),
			sprite_bundle: SpriteSheetBundle {
				texture_atlas: self.0.clone(),
				sprite: TextureAtlasSprite {
					index,
					..default()
				},
				..default()
			}
		}
	}
}

fn shadow_setup(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
	let texture_handle = asset_server.load("shadows.png");
    let texture_atlas = texture_atlases.add(TextureAtlas::from_grid(
        texture_handle,
        Vec2::new(32.0, 16.0),
        3,
        2,
    ));
	
	commands.insert_resource(ShadowTexture(texture_atlas));
}

// Make sprites look nice in our sort-of-3d environment
pub fn facing_sprite_update(
    parent_query: Query<&Transform, (Without<FacingSpriteMarker>, Without<Camera>)>,
    mut sprite_query: Query<
        (&mut Transform, &Parent, Option<&SpriteOffset>),
        (With<FacingSpriteMarker>, Without<Camera>),
    >,
    camera_query: Query<&Transform, (With<Camera>, Without<FacingSpriteMarker>)>,
) {
    let camera_transform = camera_query.single();
    let camera_inverse = Transform::from_matrix(camera_transform.compute_matrix().inverse());

    for (mut sprite_transform, parent, maybe_offset) in sprite_query.iter_mut() {
        if let Ok(parent_transform) = parent_query.get(parent.get()) {
            let parent_position = parent_transform.translation;
            let sprite_offset = match maybe_offset {
                Some(SpriteOffset(o)) => *o,
                None => Vec3::ZERO,
            };

            // First we need to transform everything w.r.t the camera
            let parent_camera_loc =
                camera_inverse * (parent_position + camera_transform.rotation * sprite_offset);

            // Then, we want to set the sprite to be pixel-aligned
            let target_position = parent_camera_loc.round();

            // Then we adjust sprite positioning as needed
            sprite_transform.rotation = camera_transform.rotation;
            sprite_transform.translation = (*camera_transform * target_position) - parent_position;
        }
    }
}

#[derive(Component, Deref, DerefMut)]
pub struct AnimationTimer(pub Timer);

// Hovering up and down
#[derive(Component)]
pub struct SpriteHover {
	pub frequency: f32,
	pub amplitude: f32,
	time_elapsed: Duration,
}
impl SpriteHover {
	pub fn new(period: f32, amplitude: f32) -> Self {
		Self {
			frequency: 1.0 / period,
			amplitude,
			time_elapsed: Duration::from_secs(0)
		}
	}
}

fn hover_update(
    time: Res<Time>,
    mut query: Query<(
        &mut SpriteHover,
        &mut SpriteOffset
    )>,
    spell_ui_active: Res<ui::SpellUiActive>,
) {
	if spell_ui_active.0 {
		return;
	}
	
	for (mut hover, mut offset) in query.iter_mut() {
		let prev_sine = (hover.time_elapsed.as_secs_f32() * hover.frequency * std::f32::consts::TAU).sin();
		hover.time_elapsed += time.delta();
		let new_sine = (hover.time_elapsed.as_secs_f32() * hover.frequency * std::f32::consts::TAU).sin();
		
		offset.0.y += hover.amplitude * (new_sine - prev_sine);
	}
}

/// Simple looping animations
/// bool is whether animation should go forward
#[derive(Component)]
pub struct SimpleAnimationMarker(pub bool);

#[derive(Bundle)]
pub struct SimpleAnimationBundle {
	#[bundle]
	sprite_sheet: SpriteSheetBundle,
	offset: SpriteOffset,
	facing_marker: FacingSpriteMarker,
	anim_marker: SimpleAnimationMarker,
	timer: AnimationTimer,
}
impl SimpleAnimationBundle {
	pub fn new(
		texture_atlas: Handle<TextureAtlas>,
		y_offset: f32,
		sprite_is_reversed: bool,
	) -> Self {
		Self {
			sprite_sheet: SpriteSheetBundle {
				texture_atlas,
				..default()
			},
			offset: SpriteOffset(Vec3::new(0.0, y_offset, 0.0)),
			facing_marker: FacingSpriteMarker,
			anim_marker: SimpleAnimationMarker(sprite_is_reversed),
			timer: AnimationTimer(Timer::from_seconds(1.0 / 7.0, true)),
		}
	}
}

#[derive(Bundle)]
pub struct FacingSpriteBundle {
	#[bundle]
	pub sprite: SpriteBundle,
	pub offset: SpriteOffset,
	pub facing_marker: FacingSpriteMarker,
}
impl FacingSpriteBundle {
	pub fn new(
		texture: Handle<Image>,
		y_offset: f32,
	) -> Self {
		Self::new_vec(texture, Vec3::new(0.0, y_offset, 0.0))
	}
	pub fn new_vec(
		texture: Handle<Image>,
		offset: Vec3,
	) -> Self {
		Self {
			sprite: SpriteBundle {
				texture,
				..default()
			},
			offset: SpriteOffset(offset),
			facing_marker: FacingSpriteMarker,
		}
	}
}


fn simple_animation_update(
    time: Res<Time>,
    mut query: Query<(
        &SimpleAnimationMarker,
        &mut AnimationTimer,
        &mut TextureAtlasSprite,
        &Handle<TextureAtlas>,
    )>,
    spell_ui_active: Res<ui::SpellUiActive>,
    texture_atlases: Res<Assets<TextureAtlas>>,
) {
    if spell_ui_active.0 {
        return;
    }

    for (marker, mut timer, mut sprite, handle) in query.iter_mut() {
        timer.tick(time.delta());
        if timer.just_finished() {
            if let Some(texture_atlas) = texture_atlases.get(handle) {
                sprite.index = if marker.0 {
                    (sprite.index + 1) % texture_atlas.textures.len()
                } else {
                    let index = sprite.index as i64 - 1;
                    if index >= 0 {
                        sprite.index - 1
                    } else {
                        texture_atlas.textures.len() - 1
                    }
                }
            }
        }
    }
}

fn pause_animation_timers(
    mut query: Query<&mut AnimationTimer>,
    spell_ui_active: Res<ui::SpellUiActive>,
) {
    if spell_ui_active.0 {
        for mut timer in query.iter_mut() {
            timer.pause();
        }
    } else {
        for mut timer in query.iter_mut() {
            timer.unpause();
        }
    }
}
