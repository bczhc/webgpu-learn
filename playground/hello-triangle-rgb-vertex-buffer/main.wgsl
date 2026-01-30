struct VsOut {
    @builtin(position) pos: vec4f,
    @location(0) color: vec4f,
}

struct Vertex {
    @location(0) pos: vec3f,
    @location(1) color: vec3f,
}

@vertex
fn vs(@builtin(vertex_index) _index: u32, v: Vertex)
-> VsOut {
    return VsOut(vec4f(v.pos, 1.0), vec4f(v.color, 1.0));
}

@fragment
fn fs(@location(0) color: vec4f) -> @location(0) vec4f {
    return color;
}
