import shader from "./main.wgsl?raw";

(async () => {
    let adapter = (await navigator.gpu.requestAdapter())!!;
    let device = await adapter.requestDevice();

    let pipeline = device.createComputePipeline({
        layout: 'auto',
        compute: {
            module: device.createShaderModule({
                code: shader,
            })
        },
    });

    let input = [1, 2, 3, 4, 5];

    let workBufferData = new Float32Array(input);
    let workBuffer = device.createBuffer({
        size: workBufferData.byteLength,
        usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST | GPUBufferUsage.COPY_SRC,
        mappedAtCreation: false,
    });
    device.queue.writeBuffer(workBuffer, 0, workBufferData);

    let resultBuffer = device.createBuffer({
        size: workBufferData.byteLength,
        usage: GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST,
        mappedAtCreation: false,
    });

    let bindGroup = device.createBindGroup({
        layout: pipeline.getBindGroupLayout(0),
        entries: [
            {binding: 0, resource: workBuffer},
        ]
    });

    let encoder = device.createCommandEncoder();
    let pass = encoder.beginComputePass({});
    pass.setPipeline(pipeline);
    pass.setBindGroup(0, bindGroup);
    pass.dispatchWorkgroups(workBufferData.length);
    pass.end();

    encoder.copyBufferToBuffer(workBuffer, resultBuffer, workBufferData.byteLength);
    let commandBuffer = encoder.finish();

    device.queue.submit([commandBuffer]);

    await resultBuffer.mapAsync(GPUMapMode.READ);
    let result = new Float32Array(resultBuffer.getMappedRange()).slice();
    resultBuffer.unmap();

    console.log('input', workBufferData);
    console.log('output', result);
})();
