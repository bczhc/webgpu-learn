// size: 4
struct StaticStorage {
    scale: f32,
}

// size: 32
struct ChangingStorage {
    color: vec4f,
    offset: vec2f,
    _pad: vec2f,
}

struct VsOut {
    @builtin(position) out_pos: vec4f,
    @location(0) color: vec4f,
}

struct FsIn {
    @location(0) color: vec4f,
}

@group(0) @binding(0)
var<storage, read> static_storage: array<StaticStorage>;

@group(0) @binding(1)
var<storage, read> changing_storage: array<ChangingStorage>;

// Use a compact layout in favor of the JS code convenience.
// view: array<array<array<f32, 3>, 3>>
@group(0) @binding(2)
var<storage, read> vertex_storage: array<array<f32, 9>>;

@vertex
fn vs(@builtin(vertex_index) index: u32, @builtin(instance_index) instance: u32)
-> VsOut {
    let vertex_instance = vertex_storage[instance];
    let vertex = vec3f(
        vertex_instance[index * 3],
        vertex_instance[index * 3 + 1],
        vertex_instance[index * 3 + 2],
    );
    let scaled_vertex = vertex * static_storage[instance].scale;
    let offset_vertex = scaled_vertex + vec3f(changing_storage[instance].offset.xy, 0);

    let out_pos = vec4f(offset_vertex.xyz, 1.0);
    return VsOut(out_pos, changing_storage[instance].color);
}

@fragment
fn fs(in: FsIn) -> @location(0) vec4f {
    return in.color;
}