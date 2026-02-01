override WORKGROUP_SIZE: u32;

@group(0) @binding(0)
var<storage, read_write> data: array<f32>;

@compute @workgroup_size(WORKGROUP_SIZE)
fn compute(@builtin(global_invocation_id) id: vec3u) {
    let i = id.x;
    if i >= arrayLength(&data) { return; }
    
    for (var j = 0; j < 100000; j += 1) {
        data[i] *= sqrt(sqrt(sin(cos(data[i] * 100.0))));
    }
}
