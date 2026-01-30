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

// size: 16
struct TransformInfo {
    offset: vec2f,
    scale: f32,
    samplingMode: u32,
}

@group(0) @binding(2)
var<storage, read> t_info: TransformInfo;

@vertex
fn vs(@builtin(vertex_index) _index: u32, vertex: Vertex)
 -> VsOut {
    let pos = vec4f(vertex.pos * t_info.scale + t_info.offset, 0.0, 1.0);
    let samp_coord = select(vertex.pos, pos.xy, t_info.samplingMode == 2);
    return VsOut(pos, samp_coord);
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4f {
    let samp_coord = vec2f(
        in.tex_coord.x / 2 + 0.5,
        -in.tex_coord.y / 2+ 0.5
    );
    return textureSample(tex, samp, samp_coord);
}