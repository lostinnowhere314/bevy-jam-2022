
use bevy::prelude::*;


// Defining spell info ///////
struct SpellData {
	shape: SpellShape,
	effects: Vec<SpellEffect>,
}

enum SpellElement {
	Neutral,
	Fire,
	Water,
	Earth,
	Air,
}

enum SpellShape {
	Orb,
	Line,
}

enum SpellEffect {
	Damage {
		amt: i64,
		element: SpellElement,
	},
	CreateOnImpact(SpellData),
}
