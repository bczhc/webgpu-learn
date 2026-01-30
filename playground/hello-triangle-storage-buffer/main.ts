import shader from "./main.wgsl?raw";

const rand = (min: number, max: number) => {
    if (min === undefined) {
        min = 0;
        max = 1;
    } else if (max === undefined) {
        max = min;
        min = 0;
    }
    return min + Math.random() * (max - min);
};

(async () => {
    let canvas = document.querySelector('canvas')!!;
    let context = canvas.getContext('webgpu');

    let adapter = (await navigator.gpu.requestAdapter())!!;
    let device = await adapter.requestDevice();
    if (!device || !context) {
        alert("WebGPU is not supported");
        return;
    }

    let textureFormat = navigator.gpu.getPreferredCanvasFormat();
    context.configure({
        device,
        format: textureFormat,
    });

    function createShaderModule() {
        return device.createShaderModule({
            label: 'shader1',
            code: shader,
        })
    }

    let shaderModule = createShaderModule();

    let pipeline = device.createRenderPipeline({
        label: 'pipeline 1',
        layout: 'auto',
        vertex: {
            module: shaderModule,
        },
        fragment: {
            module: shaderModule,
            targets: [
                {format: textureFormat}
            ]
        }
    })

    const OBJECT_COUNT: number = 100;

    function createBuffersForEntities() {
        let staticBufferSize = 4 /* align: 4 */ * OBJECT_COUNT;
        let changingBufferSize = 32 /* align: 16 */ * OBJECT_COUNT;
        let staticBuffer = device.createBuffer({
            size: staticBufferSize,
            mappedAtCreation: false,
            usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST
        });
        let changingBuffer = device.createBuffer({
            size: changingBufferSize,
            mappedAtCreation: false,
            usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST
        });
        let staticBufferData = new Float32Array(staticBufferSize / 4);
        let changingBufferData = new Float32Array(changingBufferSize / 4);

        // write data to the two 'unified' buffers
        for (let i = 0; i < OBJECT_COUNT; i++) {
            let offset = [rand(-1, 1), rand(-1, 1)];
            let color = [
                rand(0, 1),
                rand(0, 1),
                rand(0, 1),
                1
            ];
            staticBufferData.set([0.4 /* scale */], i);
            changingBufferData.set(color, i * 8);
            changingBufferData.set(offset, i * 8 + 4);
        }

        let bindGroup = device.createBindGroup({
            layout: pipeline.getBindGroupLayout(0),
            label: 'bind group 0',
            entries: [
                {
                    binding: 0,
                    resource: staticBuffer,
                },
                {
                    binding: 1,
                    resource: changingBuffer,
                }
            ]
        });
        return {
            bindGroup,
            staticBuffer,
            staticBufferData,
            changingBuffer,
            changingBufferData,
        }
    }

    let storageBuffers = createBuffersForEntities();

    function render() {
        let encoder = device.createCommandEncoder();

        let pass = encoder.beginRenderPass({
            colorAttachments: [
                {
                    view: context!!.getCurrentTexture(),
                    loadOp: 'clear',
                    storeOp: 'store',
                    clearValue: [0.3, 0.3, 0.3, 1]
                },
            ],
        });
        pass.setPipeline(pipeline);
        device.queue.writeBuffer(storageBuffers.staticBuffer, 0, storageBuffers.staticBufferData);
        device.queue.writeBuffer(storageBuffers.changingBuffer, 0, storageBuffers.changingBufferData);
        pass.setBindGroup(0, storageBuffers.bindGroup);
        // Draw 100 objects with a single `draw` call!
        pass.draw(3, OBJECT_COUNT);

        pass.end()

        let commandBuffer = encoder.finish();
        device.queue.submit([commandBuffer]);
    }

    render();
})();
