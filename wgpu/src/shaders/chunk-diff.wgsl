override WORKGROUP_SIZE: u32;
// Take # u32 per thread.
override WORK_NUM_PER_THREAD: u32;

// '0b0011_1111' * 4
const U32_PALETTE_INDEX_MASK: u32 = 0x3f3f3f3f;

// 0b0100_0000
const MUTATION_MASK: u32 = 0x40;

@group(0) @binding(0)
var<storage, read_write> base_buf: array<u32>;

@group(0) @binding(1)
var<storage, read> new_buf: array<u32>;

@compute @workgroup_size(WORKGROUP_SIZE)
fn compute(
    @builtin(global_invocation_id)
    global_id: vec3u,
) {
    let index = global_id.x;
    let start_index = WORK_NUM_PER_THREAD * index;

    for (var offset = 0u; offset < WORK_NUM_PER_THREAD; offset += 1) {
        let i = start_index + offset;
        if i > arrayLength(&base_buf) {
            break;
        }
        let packed_i1 = base_buf[i] & U32_PALETTE_INDEX_MASK;
        let packed_i2 = new_buf[i] & U32_PALETTE_INDEX_MASK;
        if packed_i1 == packed_i2 {
            base_buf[i] = 0u;
        } else {
            var packed_diff_pix = 0u;

            let v1_p0 = (packed_i1 >> 0) & 0xffu;
            let v2_p0 = (packed_i2 >> 0) & 0xffu;
            packed_diff_pix |= select(v2_p0 | MUTATION_MASK, 0u, v1_p0 == v2_p0) << 0;
            let v1_p1 = (packed_i1 >> 8) & 0xffu;
            let v2_p1 = (packed_i2 >> 8) & 0xffu;
            packed_diff_pix |= select(v2_p1 | MUTATION_MASK, 0u, v1_p1 == v2_p1) << 8;
            let v1_p2 = (packed_i1 >> 16) & 0xffu;
            let v2_p2 = (packed_i2 >> 16) & 0xffu;
            packed_diff_pix |= select(v2_p2 | MUTATION_MASK, 0u, v1_p2 == v2_p2) << 16;
            let v1_p3 = (packed_i1 >> 24) & 0xffu;
            let v2_p3 = (packed_i2 >> 24) & 0xffu;
            packed_diff_pix |= select(v2_p3 | MUTATION_MASK, 0u, v1_p3 == v2_p3) << 24;

            base_buf[i] = packed_diff_pix;
        }
    }
}