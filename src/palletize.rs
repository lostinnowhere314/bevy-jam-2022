
use bevy::{
	prelude::*,
	reflect::TypeUuid,
	render::{
		camera::{Camera, RenderTarget},
        render_resource::{
			AsBindGroup, ShaderRef, Extent3d, 
			TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        },
        view::RenderLayers,
	},
	sprite::{Material2d, Material2dPlugin, MaterialMesh2dBundle},
};
use std::fs::File;
use std::path::Path;
use std::io::{BufRead, BufReader};
// For checking CARGO_MANIFEST_DIR
use std::env; 

const PALETTE_RESOLUTION: u32 = 8;

/// Filepath: will be relative to the Assets folder
#[allow(dead_code)] 
pub enum PaletteSource {
	Raw(Vec<Color>),
	Filepath(String),
}

pub struct Palletize {
	pub palette_source: PaletteSource,
}

/// Loads the palette data.
/// If a Filepath source is provided, will panic if the file cannot be opened,
/// contains invalid input, or contains no input.
/// Palette files are assumed to be formatted as how paint.net expects them;
/// each line either is a comment beginning with the ; character, or a
/// valid hex code. Hex codes can be either RRGGBB or AARRGGBB.
fn load_palette(source: &PaletteSource) -> PaletteData {
	PaletteData(match source {
		PaletteSource::Raw(values) => values.to_vec(),
		PaletteSource::Filepath(path) => {
			// Convert into file objects
			// Get the root directory first
			let root = if let Ok(manifest) = env::var("CARGO_MANIFEST_DIR") {
				manifest
			} else {
				String::from(".")
			};
			// unfortunately with how we're calling this there isn't a super great way to get the assets folder
			// TODO move stuff around so we can get the AssetServerSettings resource
			let path = root + &String::from("/assets/") + path;
			let filepath = Path::new(&path);
			let display = filepath.display();
			
			let file = match File::open(filepath) {
				Err(why) => panic!("failed to open palette file {}: {}", display, why),
				Ok(file) => file
			};
			
			// Incrementally build the palette
			let mut colors = Vec::<Color>::new();
			
			for line in BufReader::new(file).lines().flatten() {
				let contents = line.trim();
				// Only proceed if nonempty
				if !contents.is_empty() {
					// Comments are preceded by ";"
					if contents[0..1].eq(";") {
						continue
					}
					colors.push(match match contents.len() {
						6 => Color::hex(contents),
						8 => Color::hex(String::from(&contents[2..8])),
						_ => panic!("invalid hex value {} encountered in palette file {}", contents, display)
					} {
						Ok(color) => color,
						Err(e) => panic!("hex error {} encountered in palette file {}", e, display)
					});
				}
			}
			
			if colors.is_empty() {
				panic!("palette file {} contained no color values", display)
			}
			
			colors
		}
	})
}


#[derive(Deref)]
struct PaletteData(Vec<Color>);

impl Plugin for Palletize {
	fn build(&self, app: &mut App) {
		app
			.insert_resource(Msaa {samples: 1})
			.insert_resource(load_palette(&self.palette_source))
			.add_plugin(Material2dPlugin::<PostProcessingMaterial>::default())
			// Make sure the image handle is inserted by the time we want to run other systems
			.add_startup_system_to_stage(StartupStage::PreStartup, setup);
	}
}

// for finding/filtering out this camera if needed
#[derive(Default, Component)]
pub struct PostProcessCameraMarker;

// this exists so that we can attach it to cameras elsewhere
pub struct PostprocessRenderTarget {
	target: Handle<Image>,
}
impl PostprocessRenderTarget {
	pub fn get_render_target(&self) -> RenderTarget {
		RenderTarget::Image(self.target.clone())
	}
	
	pub fn get_default_camera(&self) -> Camera {
		Camera {
			target: self.get_render_target(),
			..default()
		}
	}
}

// Code to generate the palette texture
fn get_palette_image(palette: &Vec<Color>) -> Image {
	let size = Extent3d {
		width: PALETTE_RESOLUTION,
		height: PALETTE_RESOLUTION,
		depth_or_array_layers: PALETTE_RESOLUTION,
	};
	
	// Initialize the image and fill with zeros
	let mut image = Image {
		texture_descriptor: TextureDescriptor {
			label: None,
            size,
            dimension: TextureDimension::D3,
            format: TextureFormat::Bgra8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
				| TextureUsages::COPY_DST,
		},
		..default()
	};
	image.resize(size);
	
	for i in 0..PALETTE_RESOLUTION {
		for j in 0..PALETTE_RESOLUTION {
			for k in 0..PALETTE_RESOLUTION {
				let source_color = get_source_color(i, j, k);
				let closest_color = get_closest_color(source_color, palette);
				write_pixel(&mut image.data, i, j, k, closest_color);
			}
		}
	}
	
	image
}

fn abs(val: f32) -> f32 {
	if val >= 0.0 {
		val
	} else {
		-val
	}
}

fn get_closest_color(color: Color, color_list: &Vec<Color>) -> &Color {
	let mut closest_color = &color_list[0];
	let mut closest_dist = f32::INFINITY;
	
	for other_color in color_list {
		let power = 0.3;
		let dist = abs(color.r().powf(power) - other_color.r().powf(power))
					+ abs(color.g().powf(power) - other_color.g().powf(power))
					+ abs(color.b().powf(power) - other_color.b().powf(power));
		
		if dist < closest_dist {
			closest_dist = dist;
			closest_color = other_color;
		}
	}
	
	closest_color
}

fn get_source_color(i: u32, j: u32, k: u32) -> Color {
	let max = (PALETTE_RESOLUTION as f32) - 1.0;
	Color::rgb((i as f32) / max, 
			(j as f32) / max, 
			(k as f32) / max)
}

fn write_pixel(data: &mut [u8], i: u32, j: u32, k: u32, color: &Color) {
	let start_index = 4 * (i + PALETTE_RESOLUTION * (j + PALETTE_RESOLUTION * k)) as usize;
	
	// Set the different parts
	data[start_index] = cval_to_u8(color.b());
	data[start_index + 1] = cval_to_u8(color.g());
	data[start_index + 2] = cval_to_u8(color.r());
	data[start_index + 3] = cval_to_u8(1.0);
}

fn cval_to_u8(cval: f32) -> u8 {
	if cval >= 1.0 {
		255
	} else if cval <= 0.0 {
		0
	} else {
		(255.0 * cval) as u8
	}
}

#[allow(clippy::unwrap_used)] 
fn setup(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
    mut post_processing_materials: ResMut<Assets<PostProcessingMaterial>>,
    mut images: ResMut<Assets<Image>>,
	palette: Res<PaletteData>,
) {
	
	// Get image size
	let size = Extent3d {
		width: 320,
		height: 200,
		..default()
	};
	
	// Texture to render to
	let mut render_image = Image {
		texture_descriptor: TextureDescriptor {
			label: None,
            size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
		},
		..default()
	};
	render_image.resize(size);
	
	let image_handle = images.add(render_image);
	commands.insert_resource(PostprocessRenderTarget{ target: image_handle.clone() });
	
	// Get the pallette image
	let pallette_image = get_palette_image(&palette.0);
	let pallette_handle = images.add(pallette_image);
	
	// Render layers
	// TODO make a paremeter
	let post_processing_pass_layer = RenderLayers::layer((RenderLayers::TOTAL_LAYERS - 1) as u8);
	
	// What we will render to
	let quad_handle = meshes.add(Mesh::from(shape::Quad::new(Vec2::new(
		size.width as f32,
		size.height as f32,
	))));
	
	let material_handle = post_processing_materials.add(PostProcessingMaterial {
		source_image: image_handle,
		pallette_texture: pallette_handle,
	});
	
	// Create entities for quad and camera
	commands
		.spawn_bundle(MaterialMesh2dBundle {
			mesh: quad_handle.into(),
			material: material_handle,
			transform: Transform {
				translation: Vec3::new(0.0, 0.0, 1.5),
				scale: Vec3::splat(2.0),
				..default()
			},
			..default()
		})
		.insert(post_processing_pass_layer);
		
	commands.spawn_bundle(
		Camera2dBundle {
			camera: Camera {
				priority: 1,
				..default()
			},
			..Camera2dBundle::default()
		})
		.insert(post_processing_pass_layer);
}

// Material for the post-processing
#[derive(AsBindGroup, TypeUuid, Clone)]
#[uuid = "7aed755a-f83b-11ec-b939-0242ac120002"]
struct PostProcessingMaterial {
	#[texture(0)]
	#[sampler(1)]
	source_image: Handle<Image>,
	#[texture(2, dimension="3d")]
	#[sampler(3)]
	pallette_texture: Handle<Image>,
}

impl Material2d for PostProcessingMaterial {
	fn fragment_shader() -> ShaderRef {
		"shaders/palletization.wgsl".into()
	}
}
