use super::{player, spells};
use bevy::{math::Vec4Swizzles, prelude::*, utils::HashMap};
use leafwing_input_manager::prelude::*;

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AllMouseoverTargets::new())
            .insert_resource(SpellUiActive(false))
            .insert_resource(CurrentMouseoverTarget(None))
            .add_startup_system(setup_spell_ui)
            .add_system(update_spell_ui_visibility)
            .add_system(toggle_spell_ui)
			.add_system(update_rune_ui_displays)
			.add_system(update_selection_rune_containers.before(update_rune_ui_displays))
            .add_system_to_stage(CoreStage::PreUpdate, update_cursor_ui_target);
    }
}

/// Keeps track of whether the spell inventory UI is active.
pub struct SpellUiActive(pub bool);

/// Component for updating ui rune sprites. Recommended to put on a child entity.(probably)
#[derive(Debug, Component, Deref)]
pub struct UiRuneContainer(pub Option<spells::Rune>);
/// Component for updating the above for selected runes
#[derive(Debug, Component)]
pub struct SelectedRuneContainer(pub usize);
/// Resource to store rune sprites
#[derive(Debug, Deref)]
pub struct RuneUiSprites(pub HashMap<spells::Rune, Handle<Image>>);

/// Updates sprites based off of containers
fn update_rune_ui_displays(
	mut query: Query<(&UiRuneContainer, &mut UiImage, &mut Visibility)>,
	sprites: Res<RuneUiSprites>,
) {
	for (rune_container, mut image, mut visibility) in query.iter_mut() {
		let maybe_image_handle = match rune_container.0 {
			Some(rune) => sprites.get(&rune),
			None => None
		};
		
		if let Some(image_handle) = maybe_image_handle {
			image.0 = image_handle.clone();
			visibility.is_visible = true;
		} else {
			visibility.is_visible = false;
		}
		
	}
}

/// Updates containers based off of the selected runes
fn update_selection_rune_containers(
	mut query: Query<(&SelectedRuneContainer, &mut UiRuneContainer)>,
	equipped: Res<spells::EquippedRunes>
) {
	for (index_container, mut rune_container) in query.iter_mut() {
		rune_container.0 = *equipped.0.get(index_container.0).expect("invalid SelectedRuneContainer index encountered (must be in 0..4)");
	}
}

/// Mark as part of the spell UI. if inventory_page is true, will only show when inventory is open.
#[derive(Component, Debug)]
struct SpellSelectUi {
    inventory_page: bool,
}

fn update_spell_ui_visibility(
    mut query: Query<(&mut Visibility, &SpellSelectUi)>,
    spell_ui_active: Res<SpellUiActive>,
) {
    for (mut vis, spell_ui) in &mut query {
        // Visible if the booleans are the same value
        vis.is_visible = spell_ui.inventory_page == spell_ui_active.0;
    }
}

fn toggle_spell_ui(
    action_state: Query<&ActionState<player::Action>>,
    mut spell_ui_active: ResMut<SpellUiActive>,
) {
    let action_state = action_state.single();
    // toggle if tab is pressed
    if action_state.just_pressed(player::Action::OpenInventory) {
        spell_ui_active.0 = !spell_ui_active.0;
    }
}

const SPELL_SLOT_SIZE: i32 = 20;

fn get_scaled_size(w: i32, h: i32) -> Node {
    Node {
        size: Vec2::new(w as f32, h as f32) * 2.0,
    }
}

fn setup_spell_ui(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut all_mouseover_targets: ResMut<AllMouseoverTargets>,
) {
    let mut new_mouseover_targets = Vec::<MouseoverTargetSpace>::new();

    let rune_slot_handle = asset_server.load("ui/spell-slot.png");

	// Selected rune slots
	let spell_slot_file_paths = [
		"ui/spell-slot-1.png",
		"ui/spell-slot-2.png",
		"ui/spell-slot-3.png",
		"ui/spell-slot-q.png",
		"ui/spell-slot-e.png",
	];
	
	for (i, &path) in spell_slot_file_paths.iter().enumerate() {
		let image_handle: Handle<Image> = asset_server.load(path);
		
		// When UI is closed
		commands
			.spawn_bundle(NodeBundle {
				node: get_scaled_size(20, 28),
				style: Style {
					position_type: PositionType::Absolute,
					position: UiRect {
						top: Val::Px(4.0),
						left: Val::Px(640.0 - (4.0 + (4-i + 1) as f32 * 44.0)),
						..default()
					},
					..default()
				},
				..default()
			})
			.insert(SpellSelectUi{ inventory_page: false})
			.with_children(|parent| {
				// Inner node for actually displaying things
				parent
					.spawn_bundle(ImageBundle {
						node: get_scaled_size(20, 28),
						style: Style {
							position_type: PositionType::Absolute,
							position: UiRect {
								top: Val::Px(0.0),
								left: Val::Px(0.0),
								..default()
							},
							..default()
						},
						image: UiImage(asset_server.load(path)),
						..default()
					});
				parent
					.spawn_bundle(ImageBundle {
						node: get_scaled_size(16, 16),
						style: Style {
							position_type: PositionType::Absolute,
							position: UiRect {
								top: Val::Px(4.0),
								left: Val::Px(4.0),
								..default()
							},
							..default()
						},
						image: UiImage(asset_server.load("no-sprite.png")),
						visibility: Visibility { is_visible: false },
						..default()
					})
					.insert(UiRuneContainer(None))
					.insert(SelectedRuneContainer(i));
			});
			
		/*commands
			.spawn_bundle(ImageBundle {
				node: get_scaled_size(20, 26),
				style: Style {
					position_type: PositionType::Absolute,
					position: UiRect {
						top: Val::Px(4.0),
						right: Val::Px(4.0 + (4-i) as f32 * 44.0),
						..default()
					},
					..default()
				},
				image: UiImage(asset_server.load(path)),
				..default()
			})
			.insert(SpellSelectUi{ inventory_page: false})
			.with_children(|parent| {
				// Inner node for displaying runes
				parent
					.spawn_bundle(ImageBundle {
						node: get_scaled_size(16, 16),
						style: Style {
							position_type: PositionType::Relative,
							position: UiRect {
								top: Val::Px(4.0),
								left: Val::Px(4.0),
								..default()
							},
							..default()
						},
						image: UiImage(asset_server.load("no-sprite.png")),
						visibility: Visibility { is_visible: false },
						..default()
					})
					.insert(UiRuneContainer(None))
					.insert(SelectedRuneContainer(i));
			});*/
	}

    // TODO actually make the UI we want
    // Spawn the ui element
    let entity = commands
        .spawn_bundle(ImageBundle {
            node: get_scaled_size(SPELL_SLOT_SIZE, SPELL_SLOT_SIZE),
            image: rune_slot_handle.into(),
            style: Style {
                position_type: PositionType::Absolute,
                position: UiRect {
                    top: Val::Px(10.0),
                    left: Val::Px(10.0),
                    ..default()
                },
                ..default()
            },
            ..default()
        })
        .insert(SpellSelectUi {
            inventory_page: true,
        })
        .id();

    // Insert its target
    new_mouseover_targets.push(MouseoverTargetSpace {
        target: MouseoverTarget::SpellSelectedSlot(0),
        top: 10.0,
        left: 10.0,
        width: SPELL_SLOT_SIZE as f32 * 2.0,
        height: SPELL_SLOT_SIZE as f32 * 2.0,
        source_entity: entity,
    });
    //... (in a loop probably)

    // include all the new targets
    all_mouseover_targets.0.append(&mut new_mouseover_targets);
}

// Resource for storing available targets
#[derive(Debug)]
struct AllMouseoverTargets(Vec<MouseoverTargetSpace>);

impl AllMouseoverTargets {
    fn new() -> AllMouseoverTargets {
        AllMouseoverTargets(Vec::<MouseoverTargetSpace>::new())
    }
}

// Defines a mouse-over-able space
#[derive(Debug)]
struct MouseoverTargetSpace {
    target: MouseoverTarget,
    top: f32,
    left: f32,
    width: f32,
    height: f32,
    source_entity: Entity,
}

impl MouseoverTargetSpace {
    fn contains(&self, pos: Vec2) -> bool {
        // mouse coordinates are inverted vertically
        pos.x >= self.left
            && pos.x < self.left + self.width
            && pos.y >= self.top
            && pos.y < self.top + self.height
    }
}

// Resource for current mouseover target. None if there is not any.
#[derive(Debug)]
pub struct CurrentMouseoverTarget(pub Option<(MouseoverTarget, Entity)>);

// Mouse interaction, to determine if we are clicking on UI
#[derive(Debug, Clone, Copy)]
pub enum MouseoverTarget {
    SpellSelectedSlot(u32),
    SpellInventorySlot(u32),
}

/// Gets the position of the cursor if in the primary window
pub fn get_cursor_position(windows: Res<Windows>) -> Option<Vec2> {
    let raw_pos = windows.get_primary()?.cursor_position()?;
    Some(Vec2::new(raw_pos.x, 400.0 - raw_pos.y))
}

/// Gets the intersection of the cursor ray with the plane containing the
/// point `plane_point` with normal `plane_normal`.
/// Returns None if the cursor is not in the window.
/// (this is here because get_cursor_position is)
#[allow(non_snake_case)]
pub fn get_cursor_world_position(
    windows: Res<Windows>,
    camera: &Camera,
    camera_transform: &GlobalTransform,
    plane_point: Vec3,
    plane_normal: Vec3,
) -> Option<Vec3> {
    let cursor_screen_pos = get_cursor_position(windows)?;

    let scaled_screen_pos = Vec2::new(
        (cursor_screen_pos.x - 320.0) / 320.0,
        (200.0 - cursor_screen_pos.y) / 200.0,
    );

    // funny linear algebra time
    let A = camera.projection_matrix() * camera_transform.compute_matrix().inverse();

    let x: Vec4 = scaled_screen_pos.extend(0.0).extend(0.0);
    let n: Vec3 = plane_normal.normalize();
    let n_hat: Vec4 = n.extend(0.0);
    let p: Vec3 = plane_point;

    let rhs = A.transpose() * x + n_hat * n.dot(p);
    let lhs = A.transpose() * A + outer_product(n_hat, n_hat);

    Some((lhs.inverse() * rhs).xyz())
}

fn outer_product(left: Vec4, other: Vec4) -> Mat4 {
    Mat4::from_cols(
        left * other.x,
        left * other.y,
        left * other.z,
        left * other.w,
    )
}

// System to keep track of what the mouse is over
fn update_cursor_ui_target(
    targets: Res<AllMouseoverTargets>,
    windows: Res<Windows>,
    mut current_target: ResMut<CurrentMouseoverTarget>,
    query: Query<&Visibility>,
) {
    current_target.0 = match get_cursor_position(windows) {
        Some(pos) => {
            let mut result = None;
            for target in &targets.0 {
                // Check if the mouse is inside this target
                if target.contains(pos) {
                    if let Ok(visibility) = query.get(target.source_entity) {
                        if visibility.is_visible {
                            result = Some((target.target, target.source_entity));
                            break;
                        }
                    }
                }
            }
            result
        }
        None => None,
    };
}
