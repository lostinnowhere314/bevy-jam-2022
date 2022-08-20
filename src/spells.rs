
use bevy::prelude::*;

// Define rune info ///////
#[derive(Component, Debug, Deref)]
pub struct RuneCastQueue(Vec<Rune>);

impl RuneCastQueue {
	pub fn new() -> RuneQueue {
		RuneQueue(Vec::<Rune>::new())
	}
	
	pub fn add_rune(&mut self, rune: Rune) {
		self.0.push(rune)
	}
	
	pub fn generate_spell(&self) -> SpellData {
		unimplemented!()
	}
}



#[derive(Debug)]
pub enum Rune {
	ElementRune(SpellElement),
	ShapeRune(SpellShape),
}


// Defining spell info ///////
#[derive(Debug)]
struct SpellData {
	shape: SpellShape,
	effects: Vec<SpellEffect>,
}

#[derive(Debug)]
enum SpellElement {
	Neutral,
	Fire,
	Water,
	Earth,
	Air,
}

#[derive(Debug)]
enum SpellShape {
	Orb,
	Line,
}

#[derive(Debug)]
enum SpellEffect {
	Damage {
		amt: i64,
		element: SpellElement,
	},
	CreateOnImpact(SpellData),
}
