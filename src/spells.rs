use bevy::{
	prelude::*,
	utils::HashMap,
};

pub struct SpellPlugin;

impl Plugin for SpellPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(EquippedRunes::new())
			.add_startup_system(setup_spell_sprites);
    }
}

// Define rune info ///////
#[derive(Component, Debug, Deref)]
pub struct RuneCastQueue(Vec<Rune>);

impl RuneCastQueue {
    pub fn new() -> RuneCastQueue {
        RuneCastQueue(Vec::<Rune>::new())
    }

    pub fn add_rune(&mut self, rune: Rune) {
        self.0.push(rune)
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
                        spell_effects.push(SpellEffect::CreateOnImpact(sub_spell));
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

            // Create the actual spell
            /*Some(SpellData {
                element: SpellElement,
                size: SpellSpriteSize,
                shape: SpellShape,
                effects: Vec<SpellEffect>,
            })*/
            None
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
}

// Rune inventory will be components(?)
#[derive(Component, Debug)]
pub struct RuneInventorySlot(pub Option<Rune>);

// Defining spell info ///////
#[derive(Debug)]
struct SpellData {
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
}

// Resource for spell sprites
#[derive(Debug)]
struct AllSpellSprites(HashMap<(SpellElement, SpellSpriteSize), SpellSpriteData>);
// Store the small amount of needed animation info
#[derive(Debug)]
struct SpellSpriteData(Handle<TextureAtlas>);

fn setup_spell_sprites(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
	let sprite_data = [
		(SpellElement::Neutral, SpellSpriteSize::Tiny, "", 16, 16, 1, 1),
        (SpellElement::Neutral, SpellSpriteSize::Small, "", 16, 16, 1, 1),
        (SpellElement::Neutral, SpellSpriteSize::Normal, "", 16, 16, 1, 1),
        (SpellElement::Neutral, SpellSpriteSize::Large, "", 16, 16, 1, 1),
        (SpellElement::Fire, SpellSpriteSize::Tiny, "", 16, 16, 1, 1),
        (SpellElement::Fire, SpellSpriteSize::Small, "", 16, 16, 1, 1),
        (SpellElement::Fire, SpellSpriteSize::Normal, "", 16, 16, 1, 1),
        (SpellElement::Fire, SpellSpriteSize::Large, "", 16, 16, 1, 1),
        (SpellElement::Water, SpellSpriteSize::Tiny, "", 16, 16, 1, 1),
        (SpellElement::Water, SpellSpriteSize::Small, "", 16, 16, 1, 1),
        (SpellElement::Water, SpellSpriteSize::Normal, "", 16, 16, 1, 1),
        (SpellElement::Water, SpellSpriteSize::Large, "", 16, 16, 1, 1),
        (SpellElement::Earth, SpellSpriteSize::Tiny, "", 16, 16, 1, 1),
        (SpellElement::Earth, SpellSpriteSize::Small, "", 16, 16, 1, 1),
        (SpellElement::Earth, SpellSpriteSize::Normal, "", 16, 16, 1, 1),
        (SpellElement::Earth, SpellSpriteSize::Large, "", 16, 16, 1, 1),
        (SpellElement::Air, SpellSpriteSize::Tiny, "", 16, 16, 1, 1),
        (SpellElement::Air, SpellSpriteSize::Small, "", 16, 16, 1, 1),
        (SpellElement::Air, SpellSpriteSize::Normal, "", 16, 16, 1, 1),
        (SpellElement::Air, SpellSpriteSize::Large, "", 16, 16, 1, 1),
        (SpellElement::Metal, SpellSpriteSize::Tiny, "", 16, 16, 1, 1),
        (SpellElement::Metal, SpellSpriteSize::Small, "", 16, 16, 1, 1),
        (SpellElement::Metal, SpellSpriteSize::Normal, "", 16, 16, 1, 1),
        (SpellElement::Metal, SpellSpriteSize::Large, "", 16, 16, 1, 1),
        (SpellElement::Plant, SpellSpriteSize::Tiny, "", 16, 16, 1, 1),
        (SpellElement::Plant, SpellSpriteSize::Small, "", 16, 16, 1, 1),
        (SpellElement::Plant, SpellSpriteSize::Normal, "", 16, 16, 1, 1),
        (SpellElement::Plant, SpellSpriteSize::Large, "", 16, 16, 1, 1),
        (SpellElement::Electric, SpellSpriteSize::Tiny, "", 16, 16, 1, 1),
        (SpellElement::Electric, SpellSpriteSize::Small, "", 16, 16, 1, 1),
        (SpellElement::Electric, SpellSpriteSize::Normal, "", 16, 16, 1, 1),
        (SpellElement::Electric, SpellSpriteSize::Large, "", 16, 16, 1, 1),
        (SpellElement::Ice, SpellSpriteSize::Tiny, "", 16, 16, 1, 1),
        (SpellElement::Ice, SpellSpriteSize::Small, "", 16, 16, 1, 1),
        (SpellElement::Ice, SpellSpriteSize::Normal, "", 16, 16, 1, 1),
        (SpellElement::Ice, SpellSpriteSize::Large, "", 16, 16, 1, 1),
        (SpellElement::Light, SpellSpriteSize::Tiny, "", 16, 16, 1, 1),
        (SpellElement::Light, SpellSpriteSize::Small, "", 16, 16, 1, 1),
        (SpellElement::Light, SpellSpriteSize::Normal, "", 16, 16, 1, 1),
        (SpellElement::Light, SpellSpriteSize::Large, "", 16, 16, 1, 1),
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
