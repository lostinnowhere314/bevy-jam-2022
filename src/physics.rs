
use bevy::prelude::*;
use super::ui;

pub struct GeneralPhysicsPlugin;

impl Plugin for GeneralPhysicsPlugin {
    fn build(&self, app: &mut App) {
		app
			.add_system(update_movement);
	}
}

#[derive(Component, Deref, DerefMut, Debug)]
pub struct Speed(pub Vec3);

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
		transform.translation += speed.0 * time.delta_seconds();
	}
}