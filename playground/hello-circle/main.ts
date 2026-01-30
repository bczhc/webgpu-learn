import shader from "./main.wgsl?raw";
import {joinedPrimitivesIndexBuffer} from "../utils";

function createCircleVertices(innerRadius: number, radius: number, divisions: number) {
    let verticesData = [];
    let step = 2 * Math.PI / divisions;
    for (let r = 0; r <= 2 * Math.PI; r += step) {
        let point = [Math.cos(r), Math.sin(r)];
        let innerVertex = point.map(x => x * innerRadius);
        let outerVertex = point.map(x => x * radius);
        verticesData.push(...innerVertex, ...outerVertex);
    }
    return new Float32Array(verticesData);
}

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
        layout: 'auto',
        vertex: {
            module: shaderModule,
            buffers: [
                {
                    // slot 0
                    stepMode: 'vertex',
                    arrayStride: 8,
                    attributes: [
                        {
                            shaderLocation: 0,
                            offset: 0,
                            format: 'float32x2',
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

    let vertexData = createCircleVertices(0.3, 0.5, 100);
    let vertexBuffer = device.createBuffer({
        size: vertexData.byteLength,
        usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
        mappedAtCreation: false,
    });

    let indexBufferData = joinedPrimitivesIndexBuffer(vertexData.length);
    let indexBuffer = device.createBuffer({
        size: indexBufferData.byteLength,
        usage: GPUBufferUsage.INDEX | GPUBufferUsage.COPY_DST,
        mappedAtCreation: false,
    });

    function render() {
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
        device.queue.writeBuffer(vertexBuffer, 0, vertexData);
        device.queue.writeBuffer(indexBuffer, 0, indexBufferData);
        pass.setVertexBuffer(0, vertexBuffer);
        pass.setIndexBuffer(indexBuffer, 'uint32');
        pass.drawIndexed(indexBufferData.length)
        pass.end()

        let commandBuffer = encoder.finish();
        device.queue.submit([commandBuffer]);
    }

    render();
})();
