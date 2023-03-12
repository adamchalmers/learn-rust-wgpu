// Vertex shader

struct VertexOutput {
    // The annotation tells WGPU that this is the vertex's "clip coordinates".
    @builtin(position) clip_position: vec4<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(1 - i32(in_vertex_index)) * 0.5;
    let y = f32(i32(in_vertex_index & 1u) * 2 - 1) * 0.5;
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    return out;
}

// Fragment shader

// location(0) tells WGPU to store the return value in "the first color target"???
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> { 
    // Set color of current fragment to pink-orange.
    return vec4<f32>(1.0, 0.2, 0.1, 1.0);
}