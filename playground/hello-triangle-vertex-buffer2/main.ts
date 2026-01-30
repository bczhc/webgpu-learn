import shaderCode from './main.wgsl?raw';
import {rand} from "../utils";

(async () => {
    let canvas = document.querySelector('canvas')!!;
    let context = canvas.getContext('webgpu')!!;

    let adapter = (await navigator.gpu.requestAdapter())!!;
    let device = (await adapter.requestDevice());
    let preferredFormat = navigator.gpu.getPreferredCanvasFormat();
    context.configure({
        device,
        format: preferredFormat,
    });

    let shaderModule = device.createShaderModule({
        code: shaderCode,
        label: 'shader 1',
    });

    let pipeline = device.createRenderPipeline({
        layout: 'auto',
        vertex: {
            module: shaderModule,
            buffers: [
                {
                    // slot 0
                    stepMode: 'vertex',
                    arrayStride: 12,
                    attributes: [
                        {
                            shaderLocation: 0,
                            offset: 0,
                            format: 'float32x2',
                        },
                        {
                            shaderLocation: 1,
                            offset: 2 * 4,
                            format: 'unorm8x4',
                        }
                    ]
                },
                {
                    // slot 1
                    stepMode: 'instance',
                    arrayStride: 3 * 4,
                    attributes: [
                        {
                            shaderLocation: 2,
                            offset: 0,
                            format: 'float32x2'
                        },
                        {
                            shaderLocation: 3,
                            offset: 2 * 4,
                            format: 'float32',
                        }
                    ]
                }
            ]
        },
        fragment: {
            module: shaderModule,
            targets: [{format: preferredFormat}]
        },
    });

    const OBJECT_COUNT = 10;

    // row layout: (pos: vec2f, color: unorm8x4)
    let vertexData = new ArrayBuffer(12 * 3);
    // 我是小丑。
    new Float32Array(vertexData, 0, 2).set([0, 0.5]);
    new Float32Array(vertexData, 12, 2).set([-0.5, -0.5]);
    new Float32Array(vertexData, 24, 2).set([0.5, -0.5]);
    new Uint8Array(vertexData, 0 * 12 + 8, 4).set([255,0,0,255]);
    new Uint8Array(vertexData, 1 * 12 + 8, 4).set([0,255,0,255]);
    new Uint8Array(vertexData, 2 * 12 + 8, 4).set([0, 0, 255, 255]);

    console.log(new Uint8Array(vertexData));
    let vertexBuffer = device.createBuffer({
        size: vertexData.byteLength,
        usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
        mappedAtCreation: false,
    });

    // row layout: (offset: vec2f, scale: f32)
    let instanceVertexData = new Float32Array(3 * OBJECT_COUNT);
    for (let i = 0; i < OBJECT_COUNT; i++) {
        let data = [rand(-1, 1), rand(-1, 1), rand(0.2, 1)];
        instanceVertexData.set(data, 3 * i);
    }

    let instanceVertexBuffer = device.createBuffer({
        size: instanceVertexData.byteLength,
        usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
        mappedAtCreation: false,
    });

    function render() {
        let encoder = device.createCommandEncoder();

        let pass = encoder.beginRenderPass({
            colorAttachments: [{
                view: context.getCurrentTexture(),
                loadOp: 'clear',
                storeOp: 'store',
                clearValue: [0.3, 0.3, 0.3, 1] /* gray */,
            }],
        });

        pass.setPipeline(pipeline);
        device.queue.writeBuffer(vertexBuffer, 0, vertexData);
        device.queue.writeBuffer(instanceVertexBuffer, 0, instanceVertexData);
        pass.setVertexBuffer(0, vertexBuffer);
        pass.setVertexBuffer(1, instanceVertexBuffer);
        pass.draw(3, OBJECT_COUNT);
        pass.end();
        let commandBuffer = encoder.finish();

        device.queue.submit([commandBuffer]);
    }

    render();
})();
