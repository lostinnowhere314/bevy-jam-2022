use super::{physics, sprite, ui, enemy, expand_vec2};
use bevy::{prelude::*, utils::HashMap};

pub struct SpellPlugin;

impl Plugin for SpellPlugin {
    fn build(&self, app: &mut App) {
        // temp for testing
        let mut equipped_runes = EquippedRunes::new();
        equipped_runes.set(0, Some(Rune::ElementRune(SpellElement::Fire)));

        app.insert_resource(equipped_runes)
			.add_event::<SpellDespawnEvent>()
			.add_event::<CreateSpellEvent>()
            .add_startup_system(setup_spell_sprites)
			.add_startup_system(setup_rune_sprites)
			.add_system(process_spell_enemy_collisions)
			.add_system(despawn_spells.after(process_spell_enemy_collisions))
			.add_system(create_spells_from_events.after(process_spell_enemy_collisions));
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
		
		let mut maybe_on_impact = None::<Box<SpellData>>;
		let mut maybe_on_disappear = None::<Box<SpellData>>;

        for (i, rune) in runes_iter {
            match rune {
                Rune::ShapeRune(_) => {
                    // Recursively determine the rest of the spell
                    // Only None if the rest of it does not evaluate to a spell with a proper effect
                    if let Some(sub_spell) = create_spell_recursive(&runes[i..]) {
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
                        _ => {
							panic!("unexpected element rune encountered");
						}
                    }
                    element_runes += 1;
                }
            }
        }

        if element_runes == 0 && maybe_on_impact.is_none() && maybe_on_disappear.is_none() {
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
            Some(SpellData {
                element,
				shape: layer_shape,
                // TODO determine more dynamically
                size: SpellSpriteSize::Normal,
				damage: damage,
				// TODO determine
				mana_cost: 1,
				on_collide: maybe_on_impact,
				on_end: maybe_on_disappear,
            })
        }
    } else {
        None
    }
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
    size: SpellSpriteSize,
	damage: i32,
	mana_cost: i32,
	on_collide: Option<Box<SpellData>>,
	on_end: Option<Box<SpellData>>,
}

#[derive(Debug)]
struct SpellDespawnEvent(Entity);
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
    // remove if you don't have enough time
    Metal,
    Plant,
    Electric,
    Ice,
    Light,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpellShape {
    NoShape,
    Orb,
    Line,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
enum SpellSpriteSize {
    Tiny,
    Small,
    Normal,
    Large,
}

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

/// Resolve spell-enemy collisions
// TODO update w/ on-collide
fn process_spell_enemy_collisions(
	spell_query: Query<&SpellData, With<SpellMarker>>,
	mut enemy_query: Query<&mut enemy::EnemyHealth>,
	collisions: Res<physics::ActiveCollisions<physics::DamagesEnemies>>,
	mut spell_despawn_events: EventWriter<SpellDespawnEvent>,
) {
	for collision in collisions.iter() {
		if let (Ok(spell_data), Ok(mut enemy_health)) = (spell_query.get(collision.source_entity), enemy_query.get_mut(collision.recip_entity)) {
			println!("Collided with an enemy! :)");
			
			if spell_data.damage.is_positive() {
				enemy_health.0 -= spell_data.damage;
			}
			println!("{:?}", enemy_health);
			
			spell_despawn_events.send(SpellDespawnEvent(collision.source_entity));
		}
	}
}

// TODO update to do the on-end part
fn despawn_spells(
	mut commands: Commands,
	mut despawn_events: EventReader<SpellDespawnEvent>,
) {
	for event in despawn_events.iter() {
		// TODO do stuff for spawning sub-projectiles, particles, &c
		
		commands.entity(event.0).despawn_recursive();
	}
}

fn create_spells_from_events(
    mut commands: Commands,
    all_spell_sprites: Res<AllSpellSprites>,
	mut create_events: EventReader<CreateSpellEvent>
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
					// TODO determine dynamically. prob best related to sprite data
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
			});
	}
}

// Resource for spell sprites
#[derive(Debug)]
pub struct AllSpellSprites(HashMap<(SpellElement, SpellSpriteSize), SpellSpriteData>);
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
		(Rune::ElementRune(SpellElement::Earth), "no-sprite.png"),
		(Rune::ElementRune(SpellElement::Air), "no-sprite.png"),
	];
	
    let mut sprite_map = HashMap::<Rune, Handle<Image>>::new();
	
	for (rune, path) in sprite_data.iter() {
		let texture_handle = asset_server.load(*path);
		sprite_map.insert(*rune, texture_handle);
	}
	
    commands.insert_resource(ui::RuneUiSprites(sprite_map));
}

/// Load spell sprites (for projectiles)
fn setup_spell_sprites(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
    let sprite_data = [
        (
            SpellElement::Neutral,
            SpellSpriteSize::Tiny,
            "no-sprite.png",
            16,
            16,
            1,
            1,
        ),
        (
            SpellElement::Neutral,
            SpellSpriteSize::Small,
            "no-sprite.png",
            16,
            16,
            1,
            1,
        ),
        (SpellElement::Neutral, SpellSpriteSize::Normal, "no-sprite.png", 16, 16, 1, 1),
        (
            SpellElement::Neutral,
            SpellSpriteSize::Large,
            "no-sprite.png",
            16,
            16,
            1,
            1,
        ),
        (
            SpellElement::Fire,
            SpellSpriteSize::Tiny,
            "no-sprite.png",
            16,
            16,
            1,
            1,
        ),
        (
            SpellElement::Fire,
            SpellSpriteSize::Small,
            "no-sprite.png",
            16,
            16,
            1,
            1,
        ),
        (
            SpellElement::Fire,
            SpellSpriteSize::Normal,
            "spells/fire-large.png",
            16,
            32,
            4,
            1,
        ),
        (
            SpellElement::Fire,
            SpellSpriteSize::Large,
            "no-sprite.png",
            16,
            16,
            1,
            1,
        ),
        (
            SpellElement::Water,
            SpellSpriteSize::Tiny,
            "no-sprite.png",
            16,
            16,
            1,
            1,
        ),
        (
            SpellElement::Water,
            SpellSpriteSize::Small,
            "no-sprite.png",
            16,
            16,
            1,
            1,
        ),
        (
            SpellElement::Water,
            SpellSpriteSize::Normal,
            "no-sprite.png",
            16,
            16,
            1,
            1,
        ),
        (
            SpellElement::Water,
            SpellSpriteSize::Large,
            "no-sprite.png",
            16,
            16,
            1,
            1,
        ),
        (
            SpellElement::Earth,
            SpellSpriteSize::Tiny,
            "no-sprite.png",
            16,
            16,
            1,
            1,
        ),
        (
            SpellElement::Earth,
            SpellSpriteSize::Small,
            "no-sprite.png",
            16,
            16,
            1,
            1,
        ),
        (
            SpellElement::Earth,
            SpellSpriteSize::Normal,
            "no-sprite.png",
            16,
            16,
            1,
            1,
        ),
        (
            SpellElement::Earth,
            SpellSpriteSize::Large,
            "no-sprite.png",
            16,
            16,
            1,
            1,
        ),
        (
            SpellElement::Air,
            SpellSpriteSize::Tiny,
            "no-sprite.png",
            16,
            16,
            1,
            1,
        ),
        (
            SpellElement::Air,
            SpellSpriteSize::Small,
            "no-sprite.png",
            16,
            16,
            1,
            1,
        ),
        (
            SpellElement::Air,
            SpellSpriteSize::Normal,
            "no-sprite.png",
            16,
            16,
            1,
            1,
        ),
        (
            SpellElement::Air,
            SpellSpriteSize::Large,
            "no-sprite.png",
            16,
            16,
            1,
            1,
        ),
        (
            SpellElement::Metal,
            SpellSpriteSize::Tiny,
            "no-sprite.png",
            16,
            16,
            1,
            1,
        ),
        (
            SpellElement::Metal,
            SpellSpriteSize::Small,
            "no-sprite.png",
            16,
            16,
            1,
            1,
        ),
        (
            SpellElement::Metal,
            SpellSpriteSize::Normal,
            "no-sprite.png",
            16,
            16,
            1,
            1,
        ),
        (
            SpellElement::Metal,
            SpellSpriteSize::Large,
            "no-sprite.png",
            16,
            16,
            1,
            1,
        ),
        (
            SpellElement::Plant,
            SpellSpriteSize::Tiny,
            "no-sprite.png",
            16,
            16,
            1,
            1,
        ),
        (
            SpellElement::Plant,
            SpellSpriteSize::Small,
            "no-sprite.png",
            16,
            16,
            1,
            1,
        ),
        (
            SpellElement::Plant,
            SpellSpriteSize::Normal,
            "no-sprite.png",
            16,
            16,
            1,
            1,
        ),
        (
            SpellElement::Plant,
            SpellSpriteSize::Large,
            "no-sprite.png",
            16,
            16,
            1,
            1,
        ),
        (
            SpellElement::Electric,
            SpellSpriteSize::Tiny,
            "no-sprite.png",
            16,
            16,
            1,
            1,
        ),
        (
            SpellElement::Electric,
            SpellSpriteSize::Small,
            "no-sprite.png",
            16,
            16,
            1,
            1,
        ),
        (
            SpellElement::Electric,
            SpellSpriteSize::Normal,
            "no-sprite.png",
            16,
            16,
            1,
            1,
        ),
        (
            SpellElement::Electric,
            SpellSpriteSize::Large,
            "no-sprite.png",
            16,
            16,
            1,
            1,
        ),
        (
            SpellElement::Ice,
            SpellSpriteSize::Tiny,
            "no-sprite.png",
            16,
            16,
            1,
            1,
        ),
        (
            SpellElement::Ice,
            SpellSpriteSize::Small,
            "no-sprite.png",
            16,
            16,
            1,
            1,
        ),
        (
            SpellElement::Ice,
            SpellSpriteSize::Normal,
            "no-sprite.png",
            16,
            16,
            1,
            1,
        ),
        (
            SpellElement::Ice,
            SpellSpriteSize::Large,
            "no-sprite.png",
            16,
            16,
            1,
            1,
        ),
        (
            SpellElement::Light,
            SpellSpriteSize::Tiny,
            "no-sprite.png",
            16,
            16,
            1,
            1,
        ),
        (
            SpellElement::Light,
            SpellSpriteSize::Small,
            "no-sprite.png",
            16,
            16,
            1,
            1,
        ),
        (
            SpellElement::Light,
            SpellSpriteSize::Normal,
            "no-sprite.png",
            16,
            16,
            1,
            1,
        ),
        (
            SpellElement::Light,
            SpellSpriteSize::Large,
            "no-sprite.png",
            16,
            16,
            1,
            1,
        ),
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
        sprite_map.insert((*element, *size), SpellSpriteData {
			texture_atlas,
			// TODO get from data list
			y_offset: 24.0
		});
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