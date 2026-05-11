struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;

    // Define a square from two triangles
    // Coordinates are in NDC (-1.0 to 1.0)
    let pos = array<vec2<f32>, 6>(
        vec2<f32>(-0.5,  0.5), vec2<f32>(-0.5, -0.5), vec2<f32>( 0.5, -0.5),
        vec2<f32>( 0.5, -0.5), vec2<f32>( 0.5,  0.5), vec2<f32>(-0.5,  0.5)
    );

    let tex = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 1.0),
        vec2<f32>(1.0, 1.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 0.0)
    );

    out.clip_position = vec4<f32>(pos[in_vertex_index], 0.0, 1.0);
    out.tex_coords = tex[in_vertex_index];
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // This is where the magic happens: combining the texture and sampler
    return textureSample(t_diffuse, s_diffuse, in.tex_coords);
}