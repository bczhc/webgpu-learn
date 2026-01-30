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

@vertex
fn vs(@builtin(vertex_index) index: u32, @builtin(instance_index) instance: u32)
-> VsOut {
    let pos = array(
        vec2f(0.0, 0.5),
        vec2f(-0.5, -0.5),
        vec2f(0.5, -0.5),
    );

    let pos2 = (pos[index] * static_storage[instance].scale)
        + changing_storage[instance].offset;

    let out_pos = vec4f(pos2, 0.0, 1.0);
    return VsOut(out_pos, changing_storage[instance].color);
}

@fragment
fn fs(in: FsIn) -> @location(0) vec4f {
    return in.color;
}