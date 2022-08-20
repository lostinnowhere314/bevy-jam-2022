#import bevy_pbr::mesh_view_bindings

@group(1) @binding(0)
var ren_texture: texture_2d<f32>;

@group(1) @binding(1)
var ren_sampler: sampler;

@group(1) @binding(2)
var pal_texture: texture_3d<f32>;

@group(1) @binding(3)
var pal_sampler: sampler;


fn round_space(val: f32, n: f32) -> f32 { 
	return floor(val * n)/(n - 1.0); 
}

fn round_space2(val: f32, n: f32) -> f32 { 
	return (floor(n * val) + 0.5)/(n); 
}

@fragment
fn fragment(
    @builtin(position) position: vec4<f32>,
    #import bevy_sprite::mesh2d_vertex_output
) -> @location(0) vec4<f32> {
	// screen position, coordinates go from 0 to 1
	let uv = position.xy / vec2<f32>(view.width, view.height);
	let rounded_uv = vec2<f32>(
		round_space2(uv.x, 320.0),
		round_space2(uv.y, 200.0),
	);
	
	// this number needs to be the palette texture resolution
	let n_interval = 8.0; 
	
	// original rendered color; should be in [0,1]^3
	let ren_color = textureSample(ren_texture, ren_sampler, rounded_uv).rgb;
	
	//let offset_strength = 0.00;
	//let sample_color = vec3<f32>(
	//	round_space(ren_color.r + noise2(uv.x*2.0, uv.y + 0.3)*offset_strength, n_interval),
	//	round_space(ren_color.g + noise2(uv.x*0.3, uv.y + 0.9)*offset_strength, n_interval),
	//	round_space(ren_color.b + noise2(uv.x, uv.y)*offset_strength,  n_interval)
	//);
	
	let sample_color = vec3<f32>(
		round_space(ren_color.r, n_interval),
		round_space(ren_color.g, n_interval),
		round_space(ren_color.b,  n_interval)
	);
								
	// map to the output color
	var output_color = textureSample(pal_texture, pal_sampler, sample_color);
	
	return output_color;
}