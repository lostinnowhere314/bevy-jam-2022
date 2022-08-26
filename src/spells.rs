use super::{physics, sprite, ui, enemy, expand_vec2};
use bevy::{prelude::*, utils::HashMap};

pub struct SpellPlugin;

impl Plugin for SpellPlugin {
    fn build(&self, app: &mut App) {
        // TODO temp for testing
        let mut equipped_runes = EquippedRunes::new();
        equipped_runes.set(0, Some(Rune::ElementRune(SpellElement::Fire)));
        equipped_runes.set(1, Some(Rune::ElementRune(SpellElement::Earth)));
        equipped_runes.set(2, Some(Rune::ElementRune(SpellElement::Water)));
        equipped_runes.set(3, Some(Rune::ElementRune(SpellElement::Air)));

        app.insert_resource(equipped_runes)
			.add_event::<SpellDespawnEvent>()
			.add_event::<CreateSpellEvent>()
            .add_startup_system(setup_spell_sprites)
			.add_startup_system(setup_rune_sprites)
			.add_system(process_spell_enemy_collisions)
			.add_system_to_stage(
				CoreStage::PostUpdate,
				despawn_spells
					.before(physics::resolve_collisions::<physics::DamagesEnemies>)
			)
			.add_system(create_spells_from_events
				.after(process_spell_enemy_collisions));
    }
}

// Define rune info ///////
#[derive(Component, Debug, Deref, DerefMut)]
pub struct RuneCastQueue(Vec<Rune>);

impl RuneCastQueue {
    pub fn new() -> RuneCastQueue {
        RuneCastQueue(Vec::<Rune>::new())
    }

    pub fn generate_spell(&self) -> Option<SpellData> {
        create_spell_recursive(&self.0[..], 1.0)
    }
}

/// Turns a rune queue into SpellData, spawned recursively.
/// Returns None if the runes evaluate to a spell with no effect.
fn create_spell_recursive(runes: &[Rune], power_factor: f32) -> Option<SpellData> {
    if runes.is_empty() {
		return None;
	}
	
	let mut runes_iter = runes.iter().enumerate().peekable();

	// Get current-layer shape data
	let layer_shape = if let Some((_, &Rune::ShapeRune(s))) = runes_iter.peek() {
		runes_iter.next();
		s
	} else {
		SpellShape::NoShape
	};

	let mut fire_ct: u32 = 0;
	let mut water_ct: u32 = 0;
	let mut earth_ct: u32 = 0;
	let mut air_ct: u32 = 0;
	
	let mut maybe_on_impact = None::<Box<SpellData>>;
	let mut maybe_on_disappear = None::<Box<SpellData>>;

	for (i, rune) in runes_iter {
		match rune {
			Rune::ShapeRune(_) => {
				// Recursively determine the rest of the spell
				// Result is only None if the rest of it does not evaluate to a spell with a proper effect
				let sub_spell_power_factor = power_factor * layer_shape.get_power_multiplier();
				if let Some(sub_spell) = create_spell_recursive(&runes[i..], sub_spell_power_factor) {
					match layer_shape {
						SpellShape::NoShape | SpellShape::Line => {
							maybe_on_impact = Some(Box::new(sub_spell));
						}
						SpellShape::Orb => {
							maybe_on_disappear = Some(Box::new(sub_spell));
						},
					}
				}
				break;
			}
			Rune::ElementRune(e) => {
				match e {
					SpellElement::Fire => {
						fire_ct += 1;
					}
					SpellElement::Water => {
						water_ct += 1;
					}
					SpellElement::Earth => {
						earth_ct += 1;
					}
					SpellElement::Air => {
						air_ct += 1;
					}
					_ => {
						panic!("unexpected element rune encountered");
					}
				}
			}
		}
	}

	let total_runes = fire_ct + water_ct + earth_ct + air_ct;

	if total_runes == 0 && maybe_on_impact.is_none() && maybe_on_disappear.is_none() {
		// This spell doesn't actually do anything
		return None;
	}
	
	// Assembing the spell /////////////////////////////////////////////
	// Determine element ///////////////////////////////////////////////
	let element = SpellElement::from_counts(fire_ct, water_ct, earth_ct, air_ct);
	
	// Determine this layer's attack power /////////////////////////////
	let spell_magnitude = match element {
		SpellElement::Light => total_runes as f32 / 2.5,
		SpellElement::Neutral => 0.0,
		_ => Vec2::new((fire_ct - water_ct) as f32, (earth_ct - air_ct) as f32).length()
	};
	
	let damage = spell_magnitude 
		* layer_shape.get_damage_multiplier() 
		* element.get_damage_multiplier()
		* power_factor.sqrt();
	
	// Determine mana cost //////////////////////////////////////////////
	// Get sublayer mana cost
	let sub_cost = if let Some(ref spell_data) = maybe_on_impact {
		spell_data.mana_cost
	} else if let Some(ref spell_data) = maybe_on_disappear {
		spell_data.mana_cost
	} else {
		0.0
	};
	let mana_cost = total_runes as f32
		+ layer_shape.get_cost_multiplier() * sub_cost;
	
	// Determine spell size //////////////////////////////////////////////
	let size_factor = spell_magnitude * power_factor;
	let spell_size = SpellSize::from_size_factor(size_factor);
	
	// Speed //////////////////////////////////////////////
	let speed = layer_shape.get_base_speed() * element.get_speed_multiplier();
	
	// Knockback //////////////////////////////////////////////
	let knockback = layer_shape.get_base_knockback() * element.get_knockback_multiplier();
	
	// Assemble everything together //////////////////////////////////////
	Some(SpellData {
		element,
		shape: layer_shape,
		size: spell_size,
		damage,
		mana_cost,
		knockback,
		speed,
		on_collide: maybe_on_impact,
		on_end: maybe_on_disappear,
	})
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Rune {
    ElementRune(SpellElement),
    ShapeRune(SpellShape),
}

// Runes //////////////////////////////////////////////////////////////////////////////////////
// Resource for holding equipped runes
#[derive(Debug)]
pub struct EquippedRunes(pub Vec<Option<Rune>>);
impl EquippedRunes {
    pub fn new() -> EquippedRunes {
        let mut vals = Vec::<Option<Rune>>::with_capacity(5);
        for _ in 0..5 {
            vals.push(None);
        }
        EquippedRunes(vals)
    }

    pub fn set(&mut self, index: usize, rune: Option<Rune>) {
        if let Some(item) = self.0.get_mut(index) {
            *item = rune
        }
    }
}

// Rune inventory will be components(?)
#[derive(Component, Debug)]
pub struct RuneInventorySlot(pub Option<Rune>);

// Spells //////////////////////////////////////////////////////////////////////////////////////
#[derive(Component, Debug)]
pub struct SpellMarker;

// TODO maybe move component aspects into individual pieces
#[derive(Debug, Component, Clone)]
pub struct SpellData {
    element: SpellElement,
    shape: SpellShape,
    size: SpellSize,
	damage: f32,
	mana_cost: f32,
	speed: f32,
	knockback: f32,
	on_collide: Option<Box<SpellData>>,
	on_end: Option<Box<SpellData>>,
}

impl SpellData {
	fn get_damage(&self) -> i32 {
		if self.damage > 0.0 {
			self.damage.round() as i32
		} else {
			0
		}
	}
	
	fn get_mana_cost(&self) -> i32 {
		if self.mana_cost > 1.0 {
			self.mana_cost.round() as i32
		} else {
			1
		}
	}
}

#[derive(Debug)]
pub struct SpellDespawnEvent(Entity);
#[derive(Debug)]
pub struct CreateSpellEvent {
	pub spell_data: SpellData, 
	pub position: Vec2,
	pub move_direction: Vec2,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum SpellElement {
    Neutral,
    Fire,
    Water,
    Earth,
    Air,
    Metal,
    Plant,
    Electric,
    Ice,
    Light,
}

impl SpellElement {
	fn get_speed_multiplier(&self) -> f32 {
		unimplemented!()
	}
	
	fn get_damage_multiplier(&self) -> f32 {
		unimplemented!()
	}
	
	fn get_knockback_multiplier(&self) -> f32 {
		unimplemented!()
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpellShape {
    NoShape,
    Orb,
    Line,
}

impl SpellShape {
	/// Applies to the current layer
	fn get_damage_multiplier(&self) -> f32 {
		unimplemented!()
	}
	
	/// Applies to the layer below
	fn get_cost_multiplier(&self) -> f32 {
		unimplemented!()
	}
	
	/// Applies to the layer below
	fn get_power_multiplier(&self) -> f32 {
		unimplemented!()
	}
	
	/// Applies to the current layer
	fn get_base_speed(&self) -> f32 {
		unimplemented!()
	}
	
	/// Applies to the current layer
	fn get_base_knockback(&self) -> f32 {
		unimplemented!()
	}
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
enum SpellSize {
    Tiny,
    Small,
    Normal,
    Large,
}

impl SpellSize {
	fn from_size_factor(size_factor: f32) -> Self {
		match size_factor {
			x if x < 0.2 => SpellSize::Tiny,
			x if x < 0.9 => SpellSize::Small,
			x if x < 4.0 => SpellSize::Normal,
			_ => SpellSize::Large,
		}
	}
}

impl SpellElement {
    fn as_vec(&self) -> Vec2 {
        match self {
            Self::Neutral => Vec2::new(0.0, 0.0),
            Self::Light => Vec2::new(0.0, 0.0),
            Self::Fire => Vec2::new(-1.0, 0.0),
            Self::Water => Vec2::new(1.0, 0.0),
            Self::Earth => Vec2::new(0.0, -1.0),
            Self::Air => Vec2::new(0.0, 1.0),
            Self::Metal => Vec2::new(-1.0, -1.0),
            Self::Plant => Vec2::new(1.0, -1.0),
            Self::Electric => Vec2::new(-1.0, 1.0),
            Self::Ice => Vec2::new(1.0, 1.0),
        }
        .normalize_or_zero()
    }
	
	fn from_counts(fire_ct: u32, water_ct: u32, earth_ct: u32, air_ct: u32) -> Self {
		if fire_ct > 0 && water_ct > 0 && earth_ct > 0 && air_ct > 0 {
			return Self::Light;
		}

		let fire_water = water_ct - fire_ct;
		let earth_air = air_ct - earth_ct;
		
		let element_vec = Vec2::new(fire_water as f32, earth_air as f32).normalize_or_zero();
		SpellElement::from_element_vec(element_vec)
	}		

    fn from_element_vec(vec: Vec2) -> Self {
        let vec = vec.normalize_or_zero();
        let mut closest = Self::Neutral;
        let mut closest_dist = f32::INFINITY;

        // Find closest
        for e in ALL_ELEMENTS {
            let dist = vec.distance(e.as_vec());

            if dist < closest_dist {
                closest_dist = dist;
                closest = e;
            }
        }

        // Special case for (0,0); always return Neutral here
        if closest == Self::Neutral || closest == Self::Light {
            closest = Self::Neutral;
        }

        closest
    }
}

/// Resolve spell-enemy collisions
pub fn process_spell_enemy_collisions(
	spell_query: Query<&SpellData, With<SpellMarker>>,
	mut enemy_query: Query<&mut enemy::EnemyHealth>,
	collisions: Res<physics::ActiveCollisions<physics::DamagesEnemies>>,
	mut spell_despawn_events: EventWriter<SpellDespawnEvent>,
	mut _create_spell_events: EventWriter<CreateSpellEvent>,
) {
	for collision in collisions.iter() {
		if let (Ok(spell_data), Ok(mut enemy_health)) = (spell_query.get(collision.source_entity), enemy_query.get_mut(collision.recip_entity)) {
			println!("Collided with an enemy! :)");
			
			if spell_data.get_damage() > 0 {
				enemy_health.0 -= spell_data.get_damage();
			}
			println!("{:?}", enemy_health);
			
			if let Some(new_spell_data) = &spell_data.on_collide {
				unimplemented!()
			}
			
			spell_despawn_events.send(SpellDespawnEvent(collision.source_entity));
		}
	}
}

fn despawn_spells(
	mut commands: Commands,
	mut despawn_events: EventReader<SpellDespawnEvent>,
	spell_query: Query<&SpellData>,
	mut _create_spell_events: EventWriter<CreateSpellEvent>,
) {
	for event in despawn_events.iter() {
		if let Ok(spell_data) = spell_query.get(event.0) {
			if let Some(new_spell_data) = &spell_data.on_end {
				unimplemented!()
			}
		}
		
		commands.entity(event.0).despawn_recursive();
	}
}

fn create_spells_from_events(
    mut commands: Commands,
    all_spell_sprites: Res<AllSpellSprites>,
	mut create_events: EventReader<CreateSpellEvent>,
	shadow_texture: Res<sprite::ShadowTexture>,
) {
	for event in create_events.iter() {
		let texture_data = all_spell_sprites
			.get(&event.spell_data)
			.expect("failed to get spell projectile sprite");

		let movement_direction = match event.move_direction.try_normalize() {
			Some(m) => m,
			None => continue,
		};

		// TODO determine dynamically
		let speed = 100.0;

		commands
			.spawn()
			.insert(SpellMarker)
			.insert(event.spell_data.clone())
			.insert(physics::CollisionSource::<physics::DamagesEnemies>::new(
				physics::Collider::Circle {
					center: Vec2::ZERO,
					// TODO determine dynamically. prob best related to spell size
					radius: 8.0
				}
			))
			.insert_bundle(SpatialBundle {
				transform: Transform::from_translation(expand_vec2(event.position)),
				..default()
			})
			.insert(physics::Speed(movement_direction * speed))
			.with_children(|parent| {
				parent
					.spawn()
					.insert(sprite::FacingSpriteMarker)
					.insert(sprite::SimpleAnimationMarker(true))
					.insert(sprite::AnimationTimer(Timer::from_seconds(1.0 / 7.0, true)))
					.insert(sprite::SpriteOffset(Vec3::Y * texture_data.y_offset))
					.insert_bundle(SpriteSheetBundle {
						texture_atlas: texture_data.texture_atlas.clone(),
						..default()
					});
				// TODO determine index from spell size
				parent.spawn_bundle(shadow_texture.get_shadow_bundle(1));
			});
	}
}

// Resource for spell sprites
#[derive(Debug)]
pub struct AllSpellSprites(HashMap<(SpellElement, SpellSize), SpellSpriteData>);
// Store the small amount of needed animation info
#[derive(Debug)]
struct SpellSpriteData {
	texture_atlas: Handle<TextureAtlas>, 
	y_offset: f32
}

impl AllSpellSprites {
    fn get(&self, spell_data: &SpellData) -> Option<&SpellSpriteData> {
        self.0.get(&(spell_data.element, spell_data.size))
    }
}

/// Load spell rune sprites (dealt with in ui.rs)
fn setup_rune_sprites(
	mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
	let sprite_data = [
		(Rune::ShapeRune(SpellShape::Orb), "no-sprite.png"),
		(Rune::ShapeRune(SpellShape::Line), "no-sprite.png"),
		(Rune::ElementRune(SpellElement::Fire), "ui/rune-fire.png"),
		(Rune::ElementRune(SpellElement::Water), "ui/rune-water.png"),
		(Rune::ElementRune(SpellElement::Earth), "ui/rune-earth.png"),
		(Rune::ElementRune(SpellElement::Air), "ui/rune-air.png"),
	];
	
    let mut sprite_map = HashMap::<Rune, Handle<Image>>::new();
	
	for (rune, path) in sprite_data.iter() {
		let texture_handle = asset_server.load(*path);
		sprite_map.insert(*rune, texture_handle);
	}
	
    commands.insert_resource(ui::RuneUiSprites(sprite_map));
}

struct SpellSpriteDimensions(usize,usize,usize,usize);

// TODO fill out
fn get_spell_sprite_dimensions(element: SpellElement, size: SpellSize) -> SpellSpriteDimensions {
	match (element, size) {
		(SpellElement::Neutral, SpellSize::Tiny) => SpellSpriteDimensions(16,16,4,1),	
		(SpellElement::Neutral, SpellSize::Small) => SpellSpriteDimensions(16,16,4,1),	
		(SpellElement::Neutral, SpellSize::Normal) => SpellSpriteDimensions(16,16,4,1),	
		(SpellElement::Neutral, SpellSize::Large) => SpellSpriteDimensions(16,16,4,1),
		(SpellElement::Fire, SpellSize::Tiny) => SpellSpriteDimensions(8,8,4,1),
		(SpellElement::Fire, SpellSize::Small) => SpellSpriteDimensions(16,16,4,1),
		(SpellElement::Fire, SpellSize::Normal) => SpellSpriteDimensions(16,24,4,1),
		(SpellElement::Fire, SpellSize::Large) => SpellSpriteDimensions(16,32,4,1),
		(SpellElement::Water, SpellSize::Tiny) => SpellSpriteDimensions(16,16,4,1),
		(SpellElement::Water, SpellSize::Small) => SpellSpriteDimensions(16,16,4,1),
		(SpellElement::Water, SpellSize::Normal) => SpellSpriteDimensions(16,16,4,1),
		(SpellElement::Water, SpellSize::Large) => SpellSpriteDimensions(16,16,4,1),
		(SpellElement::Earth, SpellSize::Tiny) => SpellSpriteDimensions(16,16,4,1),
		(SpellElement::Earth, SpellSize::Small) => SpellSpriteDimensions(16,16,4,1),
		(SpellElement::Earth, SpellSize::Normal) => SpellSpriteDimensions(16,16,4,1),
		(SpellElement::Earth, SpellSize::Large) => SpellSpriteDimensions(16,16,4,1),
		(SpellElement::Air, SpellSize::Tiny) => SpellSpriteDimensions(16,16,4,1),
		(SpellElement::Air, SpellSize::Small) => SpellSpriteDimensions(16,16,4,1),
		(SpellElement::Air, SpellSize::Normal) => SpellSpriteDimensions(16,16,4,1),
		(SpellElement::Air, SpellSize::Large) => SpellSpriteDimensions(16,16,4,1),
		(SpellElement::Metal, SpellSize::Tiny) => SpellSpriteDimensions(16,16,4,1),
		(SpellElement::Metal, SpellSize::Small) => SpellSpriteDimensions(16,16,4,1),
		(SpellElement::Metal, SpellSize::Normal) => SpellSpriteDimensions(16,16,2,1),
		(SpellElement::Metal, SpellSize::Large) => SpellSpriteDimensions(16,16,4,1),
		(SpellElement::Plant, SpellSize::Tiny) => SpellSpriteDimensions(16,16,4,1),
		(SpellElement::Plant, SpellSize::Small) => SpellSpriteDimensions(16,16,4,1),
		(SpellElement::Plant, SpellSize::Normal) => SpellSpriteDimensions(20,20,4,1),
		(SpellElement::Plant, SpellSize::Large) => SpellSpriteDimensions(16,16,4,1),
		(SpellElement::Electric, SpellSize::Tiny) => SpellSpriteDimensions(16,16,4,1),
		(SpellElement::Electric, SpellSize::Small) => SpellSpriteDimensions(16,16,4,1),
		(SpellElement::Electric, SpellSize::Normal) => SpellSpriteDimensions(16,16,4,1),
		(SpellElement::Electric, SpellSize::Large) => SpellSpriteDimensions(16,16,4,1),
		(SpellElement::Ice, SpellSize::Tiny) => SpellSpriteDimensions(16,16,4,1),
		(SpellElement::Ice, SpellSize::Small) => SpellSpriteDimensions(16,16,4,1),
		(SpellElement::Ice, SpellSize::Normal) => SpellSpriteDimensions(16,16,4,1),
		(SpellElement::Ice, SpellSize::Large) => SpellSpriteDimensions(16,16,4,1),
		(SpellElement::Light, SpellSize::Tiny) => SpellSpriteDimensions(16,16,4,1),
		(SpellElement::Light, SpellSize::Small) => SpellSpriteDimensions(16,16,4,1),
		(SpellElement::Light, SpellSize::Normal) => SpellSpriteDimensions(20,20,4,1),
		(SpellElement::Light, SpellSize::Large) => SpellSpriteDimensions(16,16,4,1),
	}
}

fn get_sprite_asset_path(element: SpellElement, size: SpellSize) -> String {
	let element_str = match element {
		SpellElement::Neutral => "neutral",
		SpellElement::Fire => "fire",
		SpellElement::Water => "water",
		SpellElement::Earth => "earth",
		SpellElement::Air => "air",
		SpellElement::Metal => "metal",
		SpellElement::Plant => "plant",
		SpellElement::Electric => "electricity",
		SpellElement::Ice => "ice",
		SpellElement::Light => "light",
	};
	let size_str = match size {
		SpellSize::Tiny => "tiny",
		SpellSize::Small => "small",
		SpellSize::Normal => "normal",
		SpellSize::Large => "large",
	};
	
	String::from("spells/".to_owned() + element_str + "-" + size_str + ".png")
}

/// Load spell sprites (for projectiles)
fn setup_spell_sprites(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
    
    let mut sprite_map = HashMap::<(SpellElement, SpellSize), SpellSpriteData>::new();
	
	for element in ALL_ELEMENTS.iter() {
		for size in ALL_SIZES.iter() {
			let path = get_sprite_asset_path(*element, *size);
			let sprite_data = get_spell_sprite_dimensions(*element, *size);
			
			// TODO update
			let texture_handle = asset_server.load(&path);
			let texture_atlas = texture_atlases.add(TextureAtlas::from_grid(
				texture_handle,
				Vec2::new(sprite_data.0 as f32, sprite_data.1 as f32),
				sprite_data.2,
				sprite_data.3,
			));
			sprite_map.insert((*element, *size), SpellSpriteData {
				texture_atlas,
				y_offset: 32.0
			});
		}
	}
	
    commands.insert_resource(AllSpellSprites(sprite_map));
}

const ALL_ELEMENTS: [SpellElement; 10] = [
    SpellElement::Neutral,
    SpellElement::Fire,
    SpellElement::Water,
    SpellElement::Earth,
    SpellElement::Air,
    SpellElement::Metal,
    SpellElement::Plant,
    SpellElement::Electric,
    SpellElement::Ice,
    SpellElement::Light,
];
const ALL_SIZES: [SpellSize; 4] = [
    SpellSize::Tiny,
    SpellSize::Small,
    SpellSize::Normal,
    SpellSize::Large,
];