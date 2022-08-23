use bevy::{
	prelude::*,
	utils::HashMap,
};
use super::{sprite, physics};

pub struct SpellPlugin;

impl Plugin for SpellPlugin {
    fn build(&self, app: &mut App) {
		// temp for testing
		let mut equipped_runes = EquippedRunes::new();
		equipped_runes.set(0, Some(Rune::ElementRune(SpellElement::Fire)));
		
        app.insert_resource(equipped_runes)
			.add_startup_system(setup_spell_sprites);
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
        create_spell_recursive(&self.0[..])
    }
}

fn create_spell_recursive(runes: &[Rune]) -> Option<SpellData> {
    if !runes.is_empty() {
        let mut runes_iter = runes.iter().enumerate().peekable();
        let mut spell_effects = Vec::<SpellEffect>::with_capacity(2);

        // Get current-layer shape data
        let layer_shape = if let Some((_, &Rune::ShapeRune(s))) = runes_iter.peek() {
            runes_iter.next();
            s
        } else {
            SpellShape::NoShape
        };

        let mut fire_water = 0;
        let mut earth_air = 0;
        let mut element_runes = 0;
        let mut has_sub_spell = false;

        for (i, rune) in runes_iter {
            match rune {
                Rune::ShapeRune(_) => {
                    // Recursively determine the rest of the spell
                    // Only None if the rest of it does not evaluate to a spell with a proper effect
                    if let Some(sub_spell) = create_spell_recursive(&runes[i..]) {
                        spell_effects.push(match layer_shape {
							SpellShape::NoShape
							| SpellShape::Line => SpellEffect::CreateOnImpact(sub_spell),
							SpellShape::Orb => SpellEffect::CreateOnDisappear(sub_spell),
						});
                        has_sub_spell = true;
                    }
                    break;
                }
                Rune::ElementRune(e) => {
                    match e {
                        SpellElement::Fire => {
                            fire_water -= 1;
                        }
                        SpellElement::Water => {
                            fire_water += 1;
                        }
                        SpellElement::Earth => {
                            earth_air -= 1;
                        }
                        SpellElement::Air => {
                            earth_air += 1;
                        }
                        _ => {}
                    }
                    element_runes += 1;
                }
            }
        }

        if element_runes == 0 && !has_sub_spell {
            // This spell doesn't actually do anything
            None
        } else {
            // Determine element
            let element_vec = Vec2::new(fire_water as f32, earth_air as f32).normalize_or_zero();
            let element = SpellElement::from_element_vec(element_vec, element_runes > 0);

            // TODO pass recursively
            let multiplier = 1.0;
            let damage = 1
                + (match element {
                    SpellElement::Neutral => 0.0,
                    SpellElement::Fire => 6.0,
                    SpellElement::Water | SpellElement::Earth | SpellElement::Air => 5.0,
                    SpellElement::Metal
                    | SpellElement::Plant
                    | SpellElement::Electric
                    | SpellElement::Ice => 9.0,
                    SpellElement::Light => 16.0,
                } * multiplier
                    * (element_runes as f32).sqrt())
                .round() as i32;
			spell_effects.push(SpellEffect::Damage(damage));

            // Create the actual spell
            Some(SpellData {
				element: element,
				// TODO
                size: SpellSpriteSize::Normal,
                shape: layer_shape,
                effects: spell_effects,
            })
        }
    } else {
        None
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Rune {
    ElementRune(SpellElement),
    ShapeRune(SpellShape),
}

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

// Defining spell info ///////
#[derive(Debug)]
pub struct SpellData {
    element: SpellElement,
    size: SpellSpriteSize,
    shape: SpellShape,
    effects: Vec<SpellEffect>,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
enum SpellElement {
    Neutral,
    Fire,
    Water,
    Earth,
    Air,
    // remove if you don't have enough time
    Metal,
    Plant,
    Electric,
    Ice,
    Light,
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

impl SpellElement {
    fn as_vec(&self) -> Vec2 {
        match self {
            SpellElement::Neutral => Vec2::new(0.0, 0.0),
            SpellElement::Light => Vec2::new(0.0, 0.0),
            SpellElement::Fire => Vec2::new(-1.0, 0.0),
            SpellElement::Water => Vec2::new(1.0, 0.0),
            SpellElement::Earth => Vec2::new(0.0, -1.0),
            SpellElement::Air => Vec2::new(0.0, 1.0),
            SpellElement::Metal => Vec2::new(-1.0, -1.0),
            SpellElement::Plant => Vec2::new(1.0, -1.0),
            SpellElement::Electric => Vec2::new(-1.0, 1.0),
            SpellElement::Ice => Vec2::new(1.0, 1.0),
        }
        .normalize_or_zero()
    }

    fn from_element_vec(vec: Vec2, is_nonempty: bool) -> SpellElement {
        let vec = vec.normalize_or_zero();
        let mut closest = SpellElement::Neutral;
        let mut closest_dist = f32::INFINITY;

        // Find closest
        for e in ALL_ELEMENTS {
            let dist = vec.distance(e.as_vec());

            if dist < closest_dist {
                closest_dist = dist;
                closest = e;
            }
        }

        // Special case for (0,0)
        if closest == SpellElement::Neutral || closest == SpellElement::Light {
            closest = if is_nonempty {
                SpellElement::Light
            } else {
                SpellElement::Neutral
            };
        }

        closest
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
enum SpellSpriteSize {
    Tiny,
    Small,
    Normal,
    Large,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpellShape {
    NoShape,
    Orb,
    Line,
}

#[derive(Debug)]
enum SpellEffect {
    Damage(i32),
    CreateOnImpact(SpellData),
	CreateOnDisappear(SpellData),
}

/// Function for spawning spells as projectiles
/// NOT A SYSTEM
/// Does nothing if movement_direction is zero-length
pub fn create_spell(
	commands: &mut Commands, 
	spell_data: SpellData,
	spawn_position: Vec3,
	movement_direction: Vec3,
	sprite_offset: Vec3,
	all_spell_sprites: Res<AllSpellSprites>
) {
	let texture_atlas = all_spell_sprites.get(&spell_data).expect("failed to get spell projectile sprite").0.clone();
	
	let movement_direction = match movement_direction.try_normalize() {
		Some(m) => m,
		None => return
	};
	
	// TODO temp
	let speed = 300.0;
	
	commands
		.spawn()
		.insert_bundle(SpatialBundle {
			transform: Transform::from_translation(spawn_position),
			..default()
		})
		.insert(physics::Speed(movement_direction * speed))
		.with_children(|parent| {
			parent.spawn()
                .insert(sprite::FacingSpriteMarker)
				.insert(sprite::SimpleAnimationMarker(true))
                .insert(sprite::AnimationTimer(Timer::from_seconds(1.0 / 7.0, true)))
                .insert(sprite::SpriteOffset(Vec3::new(0.0, 24.0, 0.0)))
                .insert_bundle(SpriteSheetBundle {
                    texture_atlas,
                    ..default()
                });
		});
}

// Resource for spell sprites
#[derive(Debug)]
pub struct AllSpellSprites(HashMap<(SpellElement, SpellSpriteSize), SpellSpriteData>);
// Store the small amount of needed animation info
#[derive(Debug)]
struct SpellSpriteData(Handle<TextureAtlas>);

impl AllSpellSprites {
	fn get(&self, spell_data: &SpellData) -> Option<&SpellSpriteData> {
		self.0.get(&(spell_data.element, spell_data.size))
	}
}

fn setup_spell_sprites(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
	let sprite_data = [
		(SpellElement::Neutral, SpellSpriteSize::Tiny, "no-sprite.png", 16, 16, 1, 1),
        (SpellElement::Neutral, SpellSpriteSize::Small, "no-sprite.png", 16, 16, 1, 1),
        (SpellElement::Neutral, SpellSpriteSize::Normal, "no-sprite.png", 16, 16, 1, 1),
        (SpellElement::Neutral, SpellSpriteSize::Large, "no-sprite.png", 16, 16, 1, 1),
        (SpellElement::Fire, SpellSpriteSize::Tiny, "no-sprite.png", 16, 16, 1, 1),
        (SpellElement::Fire, SpellSpriteSize::Small, "no-sprite.png", 16, 16, 1, 1),
        (SpellElement::Fire, SpellSpriteSize::Normal, "no-sprite.png", 16, 16, 1, 1),
        (SpellElement::Fire, SpellSpriteSize::Large, "no-sprite.png", 16, 16, 1, 1),
        (SpellElement::Water, SpellSpriteSize::Tiny, "no-sprite.png", 16, 16, 1, 1),
        (SpellElement::Water, SpellSpriteSize::Small, "no-sprite.png", 16, 16, 1, 1),
        (SpellElement::Water, SpellSpriteSize::Normal, "no-sprite.png", 16, 16, 1, 1),
        (SpellElement::Water, SpellSpriteSize::Large, "no-sprite.png", 16, 16, 1, 1),
        (SpellElement::Earth, SpellSpriteSize::Tiny, "no-sprite.png", 16, 16, 1, 1),
        (SpellElement::Earth, SpellSpriteSize::Small, "no-sprite.png", 16, 16, 1, 1),
        (SpellElement::Earth, SpellSpriteSize::Normal, "no-sprite.png", 16, 16, 1, 1),
        (SpellElement::Earth, SpellSpriteSize::Large, "no-sprite.png", 16, 16, 1, 1),
        (SpellElement::Air, SpellSpriteSize::Tiny, "no-sprite.png", 16, 16, 1, 1),
        (SpellElement::Air, SpellSpriteSize::Small, "no-sprite.png", 16, 16, 1, 1),
        (SpellElement::Air, SpellSpriteSize::Normal, "no-sprite.png", 16, 16, 1, 1),
        (SpellElement::Air, SpellSpriteSize::Large, "no-sprite.png", 16, 16, 1, 1),
        (SpellElement::Metal, SpellSpriteSize::Tiny, "no-sprite.png", 16, 16, 1, 1),
        (SpellElement::Metal, SpellSpriteSize::Small, "no-sprite.png", 16, 16, 1, 1),
        (SpellElement::Metal, SpellSpriteSize::Normal, "no-sprite.png", 16, 16, 1, 1),
        (SpellElement::Metal, SpellSpriteSize::Large, "no-sprite.png", 16, 16, 1, 1),
        (SpellElement::Plant, SpellSpriteSize::Tiny, "no-sprite.png", 16, 16, 1, 1),
        (SpellElement::Plant, SpellSpriteSize::Small, "no-sprite.png", 16, 16, 1, 1),
        (SpellElement::Plant, SpellSpriteSize::Normal, "no-sprite.png", 16, 16, 1, 1),
        (SpellElement::Plant, SpellSpriteSize::Large, "no-sprite.png", 16, 16, 1, 1),
        (SpellElement::Electric, SpellSpriteSize::Tiny, "no-sprite.png", 16, 16, 1, 1),
        (SpellElement::Electric, SpellSpriteSize::Small, "no-sprite.png", 16, 16, 1, 1),
        (SpellElement::Electric, SpellSpriteSize::Normal, "no-sprite.png", 16, 16, 1, 1),
        (SpellElement::Electric, SpellSpriteSize::Large, "no-sprite.png", 16, 16, 1, 1),
        (SpellElement::Ice, SpellSpriteSize::Tiny, "no-sprite.png", 16, 16, 1, 1),
        (SpellElement::Ice, SpellSpriteSize::Small, "no-sprite.png", 16, 16, 1, 1),
        (SpellElement::Ice, SpellSpriteSize::Normal, "no-sprite.png", 16, 16, 1, 1),
        (SpellElement::Ice, SpellSpriteSize::Large, "no-sprite.png", 16, 16, 1, 1),
        (SpellElement::Light, SpellSpriteSize::Tiny, "no-sprite.png", 16, 16, 1, 1),
        (SpellElement::Light, SpellSpriteSize::Small, "no-sprite.png", 16, 16, 1, 1),
        (SpellElement::Light, SpellSpriteSize::Normal, "no-sprite.png", 16, 16, 1, 1),
        (SpellElement::Light, SpellSpriteSize::Large, "no-sprite.png", 16, 16, 1, 1),
	];
	
	let mut sprite_map = HashMap::<(SpellElement, SpellSpriteSize), SpellSpriteData>::new();
	
	for (element, size, path, w, h, nx, ny) in sprite_data.iter() {
		let texture_handle = asset_server.load(*path);
		let texture_atlas = texture_atlases.add(TextureAtlas::from_grid(
			texture_handle,
			Vec2::new(*w as f32, *h as f32),
			*nx,
			*ny,
		));
		sprite_map.insert((*element, *size), SpellSpriteData(texture_atlas));
	}
	
	commands.insert_resource(AllSpellSprites(sprite_map));
}
