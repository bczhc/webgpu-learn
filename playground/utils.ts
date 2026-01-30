export const rand = (min: number, max: number) => {
    if (min === undefined) {
        min = 0;
        max = 1;
    } else if (max === undefined) {
        max = min;
        min = 0;
    }
    return min + Math.random() * (max - min);
};

export function joinedPrimitivesIndexBuffer(vertexCount: number) {
    if (vertexCount < 3) throw 'requires: vertexCount >= 3';
    let tmp = [0, 1, 2];
    let count = vertexCount - 2;
    let buffer = new Uint32Array(count * 3);
    for (let i = 0; i < count; i++) {
        buffer.set(tmp, 3 * i);
        tmp = tmp.map(x => x + 1);
    }
    return buffer;
}
