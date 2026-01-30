const CANVAS_WIDTH = 500;
const CANVAS_HEIGHT = 500;

struct Vertex {
    @location(0) pos: vec2f,
}

struct VsOut {
    @builtin(position) pos: vec4f,
    @location(0) tex_coord: vec2f,
}

@group(0) @binding(0)
var samp: sampler;

@group(0) @binding(1)
var tex: texture_2d<f32>;

@vertex
fn vs(@builtin(vertex_index) _index: u32, vertex: Vertex)
 -> VsOut {
    let pos = vec4f(vertex.pos, 0.0, 1.0);
    return VsOut(pos, pos.xy);
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4f {
    let samp_coord = vec2f(
        in.tex_coord.x / 2 + 0.5,
        -in.tex_coord.y / 2+ 0.5
    );
    return textureSample(tex, samp, samp_coord);
}