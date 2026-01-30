import shader from "./main.wgsl?raw";

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
    const VERTEX_DATA = [
        /* (x, y, z), (r, g, b) */
        0, 0.5, 0.0, 1, 0, 0,
        -0.5, -0.5, 0, 0, 1, 0,
        0.5, -0.5, 0, 0, 0, 1,
    ];
    const VERTEX_DATA_RAW = new Float32Array(8 * 3);
    VERTEX_DATA_RAW.set(VERTEX_DATA.slice(0, 3), 0);
    VERTEX_DATA_RAW.set(VERTEX_DATA.slice(3, 6), 4);
    VERTEX_DATA_RAW.set(VERTEX_DATA.slice(6, 9), 8);
    VERTEX_DATA_RAW.set(VERTEX_DATA.slice(9, 12), 12);
    VERTEX_DATA_RAW.set(VERTEX_DATA.slice(12, 15), 16);
    VERTEX_DATA_RAW.set(VERTEX_DATA.slice(15, 18), 20);

    console.log(VERTEX_DATA_RAW);

    let pipeline = device.createRenderPipeline({
        layout: 'auto',
        vertex: {
            module: shaderModule,
            buffers: [
                {
                    arrayStride: 8 * 4,
                    attributes: [
                        {
                            shaderLocation: 0,
                            offset: 0,
                            format: "float32x3",
                        },
                        {
                            shaderLocation: 1,
                            offset: 4 * 4,
                            format: "float32x3",
                        }
                    ]
                }
            ]
        },
        fragment: {
            module: shaderModule,
            targets: [
                {format: textureFormat}
            ]
        }
    })

    function render() {
        let vertexBuffer = device.createBuffer({
            size: VERTEX_DATA_RAW.byteLength,
            usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
            mappedAtCreation: false,
        });
        device.queue.writeBuffer(vertexBuffer, 0, VERTEX_DATA_RAW);

        let encoder = device.createCommandEncoder();

        let pass = encoder.beginRenderPass({
            colorAttachments: [
                {
                    view: context!!.getCurrentTexture(),
                    loadOp: 'clear',
                    storeOp: 'store',
                    clearValue: [0.3, 0.3, 0.3, 1],
                },
            ],
        });
        pass.setPipeline(pipeline);
        pass.setVertexBuffer(0, vertexBuffer);
        pass.draw(3);
        pass.end()

        let commandBuffer = encoder.finish();
        device.queue.submit([commandBuffer]);
    }

    render();
})();
