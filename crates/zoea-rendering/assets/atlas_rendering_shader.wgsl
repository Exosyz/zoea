struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

struct SpriteInstance {
    pos: vec2<f32>,
    scale: vec2<f32>,
    uv_offset: vec2<f32>,
    uv_scale: vec2<f32>,
    rotation: f32,
    pad1: f32, // for padding we must have a %8 == 0 struct
    pad2: f32,
    pad3: f32,
};

// Group 0: The Texture Atlas (Global)
@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

// Group 1: The Specific Sprite Data (Uniform for now)
@group(1) @binding(0) var<storage, read> instances: array<SpriteInstance>;
// Group 2: Camera Data
@group(2) @binding(0) var<uniform> camera: mat4x4<f32>;

var<private> pos: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    vec2<f32>(-0.5,  0.5), vec2<f32>(-0.5, -0.5), vec2<f32>( 0.5, -0.5),
    vec2<f32>( 0.5, -0.5), vec2<f32>( 0.5,  0.5), vec2<f32>(-0.5,  0.5)
);

var<private> uvs: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    vec2<f32>(0.0, 0.0), vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 1.0),
    vec2<f32>(1.0, 1.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 0.0)
);

@vertex
fn vs_main(
    @builtin(vertex_index) vi: u32,
    @builtin(instance_index) ii: u32 // Added instance index!
) -> VertexOutput {
    var out: VertexOutput;
    // Access the specific data for THIS instance
    let sprite = instances[ii];

    // ... (Your rotation and position math remains the same, just use 'sprite') ...
    let c = cos(sprite.rotation);
    let s = sin(sprite.rotation);

    // Local vertex position (from our hardcoded array)
    let local_pos = pos[vi] * sprite.scale;

    // Rotate the vertex around its center
    let rotated_x = local_pos.x * c - local_pos.y * s;
    let rotated_y = local_pos.x * s + local_pos.y * c;

    // Final screen position: Rotated + Translated
    let final_pos = vec2<f32>(rotated_x, rotated_y) + sprite.pos;

    out.position = camera * vec4<f32>(final_pos, 0.0, 1.0);

    // UV Mapping
    out.tex_coords = (uvs[vi] * sprite.uv_scale) + sprite.uv_offset;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.tex_coords);
}