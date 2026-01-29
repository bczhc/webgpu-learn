const SQRT3 = sqrt(3.0);
const CANVAS_WIDTH = 500.0;

fn pos_vs_to_fs(pos: vec2f) -> vec2f {
    return
        (mat2x2f(1, 0, 0, -1) * pos)
        * CANVAS_WIDTH / 2.0
        + vec2f(CANVAS_WIDTH, CANVAS_WIDTH) / 2.0;
}

fn pos_fs_to_vs(pos: vec2f) -> vec2f {
    let vec = (pos - vec2f(CANVAS_WIDTH, CANVAS_WIDTH) / 2.0)
    * 2.0 / CANVAS_WIDTH;
    return mat2x2f(1, 0, 0, -1) * vec;
}

@vertex
fn vs(@builtin(vertex_index) index: u32)
-> @builtin(position) vec4f {
    let pos = array(
        vec2f(0.0, 0.5),
        vec2f(SQRT3 / 4.0, -0.25),
        vec2f(-SQRT3 / 4.0, -0.25),
    );

    return vec4f(pos[index], 0.0, 1.0);
}

@fragment
fn fs(@builtin(position) pos: vec4f) -> @location(0) vec4f {
    let red = vec4f(1.0, 0.0, 0.0, 1.0);
    let green = vec4f(0.0, 1.0, 0.0, 1.0);
    let blue = vec4f(0.0, 0.0, 1.0, 1.0);

    let radius = 0.1;
    let vs_pos = pos_fs_to_vs(pos.xy);
    let d = sqrt(pow(vs_pos.x, 2) + pow(vs_pos.y, 2));
    let in_circle = d < radius;

    let pos_u = vec2u(pos.xy / 8.0);
    let cb_switch = (pos_u.x + pos_u.y) % 2 == 0;

    if in_circle {
        return red;
    } else {
        return select(green, blue, cb_switch);
    }
}