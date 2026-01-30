import shader from "./main.wgsl?raw";
import {joinedPrimitivesIndexBuffer} from "../utils";
import GUI from "muigui";

let guiParam = {
    magFilter: 'nearest',
};

let gui = new GUI();
gui.add(guiParam, 'magFilter', ['linear', 'nearest']);

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

    let vertexData = new Float32Array([
        -0.5, -0.5,
        -0.5, 0.5,
        0.5, -0.5,
        0.5, 0.5,
    ]);
    vertexData = vertexData.map(x => x * 2);
    let vertexBuffer = device.createBuffer({
        size: vertexData.byteLength,
        usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
        mappedAtCreation: false,
    });

    let indexBufferData = joinedPrimitivesIndexBuffer(4);
    let indexBuffer = device.createBuffer({
        size: indexBufferData.byteLength,
        usage: GPUBufferUsage.INDEX | GPUBufferUsage.COPY_DST,
        mappedAtCreation: false,
    });

    let textureData = createTextureData();
    let texture = device.createTexture({
        size: [textureData.kTextureWidth, textureData.kTextureHeight],
        format: 'rgba8unorm',
        usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST,
    });

    function render() {
        let sampler = device.createSampler({
            magFilter: guiParam.magFilter as GPUFilterMode,
            addressModeU: 'clamp-to-edge',
            addressModeV: 'clamp-to-edge',
        });
        let bindGroup = device.createBindGroup({
            layout: pipeline.getBindGroupLayout(0),
            entries: [
                {binding: 0, resource: sampler},
                {binding: 1, resource: texture},
            ]
        });

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
        device.queue.writeBuffer(vertexBuffer, 0, vertexData);
        device.queue.writeBuffer(indexBuffer, 0, indexBufferData);
        device.queue.writeTexture(
            {texture},
            textureData.textureData,
            {bytesPerRow: textureData.kTextureWidth * 4},
            {width: textureData.kTextureWidth, height: textureData.kTextureHeight},
        );

        pass.setPipeline(pipeline);
        pass.setVertexBuffer(0, vertexBuffer);
        pass.setIndexBuffer(indexBuffer, 'uint32');
        pass.setBindGroup(0, bindGroup);
        pass.drawIndexed(6)
        pass.end()

        let commandBuffer = encoder.finish();
        device.queue.submit([commandBuffer]);
    }

    gui.onChange(render);
    render();
})();

function createTextureData() {
    const kTextureWidth = 5;
    const kTextureHeight = 7;
    const _ = [255, 0, 0, 255]; // red
    const y = [255, 255, 0, 255]; // yellow
    const b = [0, 0, 255, 255]; // blue
    //prettier-ignore
    const textureData = new Uint8Array([
        b, _, _, _, _,
        _, y, y, y, _,
        _, y, _, _, _,
        _, y, y, _, _,
        _, y, _, _, _,
        _, y, _, _, _,
        _, _, _, _, _,
    ].flat());
    return {
        kTextureWidth,
        kTextureHeight,
        textureData,
    };
}
