// Vertex shader

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color_of: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color_of: vec3<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.color_of = model.color_of;
    out.clip_position = vec4<f32>(model.position, 1.0);
    return out;
}

// Fragment shader

// location(0) tells WGPU to store the return value in "the first color target"???
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> { 
    return vec4<f32>(in.color_of, 1.0);
}