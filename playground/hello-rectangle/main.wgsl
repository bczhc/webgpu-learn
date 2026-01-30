const COLOR = vec4f(1,1,0,1);

struct Vertex {
    @location(0) pos: vec2f,
}

@vertex
fn vs(@builtin(vertex_index) _index: u32, vertex: Vertex)
-> @builtin(position) vec4f {
    return vec4f(vertex.pos, 0.0, 1.0);
}

@fragment
fn fs() -> @location(0) vec4f {
    return COLOR;
}