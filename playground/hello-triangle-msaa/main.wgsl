const COLOR = vec4f(1,0,0,1);

@vertex
fn vs(@builtin(vertex_index) index: u32)
-> @builtin(position) vec4f {
    let pos = array(
        vec2f(0.0, 0.5),
        vec2f(-0.5, -0.5),
        vec2f(0.5, -0.5),
    );

    return vec4f(pos[index], 0.0, 1.0);
}

@fragment
fn fs() -> @location(0) vec4f {
    return COLOR;
}