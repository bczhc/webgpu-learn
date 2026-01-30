import shader from "./main.wgsl?raw";
import {getImageRawData, joinedPrimitivesIndexBuffer} from "../utils";
import GUI from "muigui";
import img from '../../res/container.jpg';

let settings = {
    magFilter: 'nearest',
    texture: '1',
    scale: 0.5,
    samplingTransform: '1',
    speed: 0.1,
};

let gui = new GUI();
gui.add(settings, 'magFilter', ['linear', 'nearest']);
gui.add(settings, 'texture', ['1', '2']);
gui.add(settings, 'scale', 0, 1);
gui.add(settings, 'samplingTransform', ['1', '2']);
gui.add(settings, 'speed', 0, 1);

let fnPanel = {
    smallMovingDemo: () => {
        Object.assign(settings, {
            magFilter: 'nearest',
            texture: '2',
            scale: 0.02,
            samplingTransform: '1',
            speed: 0.01,
        });
        gui.updateDisplay();
    }
};

let fnGui = new GUI();
Object.assign(fnGui.domElement.style, {right: '', top: `${gui.domElement.clientHeight}px`});
fnGui.add(fnPanel, 'smallMovingDemo');

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

    let transformStorageData = new ArrayBuffer(16);
    let transformStorage = device.createBuffer({
        size: transformStorageData.byteLength,
        usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
        mappedAtCreation: false,
    });

    let t = 0;

    async function render() {
        let textureData: TextureData | null = null;
        if (settings.texture == '1') {
            textureData = createTexture1();
        } else if (settings.texture == '2') {
            textureData = await createTexture2();
        }
        if (textureData === null) return;
        let texture = device.createTexture({
            size: [textureData.width, textureData.height],
            format: 'rgba8unorm',
            usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST,
        });

        let sampler = device.createSampler({
            magFilter: settings.magFilter as GPUFilterMode,
            addressModeU: 'clamp-to-edge',
            addressModeV: 'clamp-to-edge',
        });
        let bindGroup = device.createBindGroup({
            layout: pipeline.getBindGroupLayout(0),
            entries: [
                {binding: 0, resource: sampler},
                {binding: 1, resource: texture},
                {binding: 2, resource: transformStorage},
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
            textureData.data as GPUAllowSharedBufferSource,
            {bytesPerRow: textureData.width * 4},
            {width: textureData.width, height: textureData.height},
        );

        let dx = Math.sin(t);
        let scale = settings.scale;
        new Float32Array(transformStorageData, 0, 3).set([dx, 0.0, scale]);
        new Uint32Array(transformStorageData, 12, 1).set([parseInt(settings.samplingTransform)]);
        device.queue.writeBuffer(transformStorage, 0, transformStorageData);

        pass.setPipeline(pipeline);
        pass.setVertexBuffer(0, vertexBuffer);
        pass.setIndexBuffer(indexBuffer, 'uint32');
        pass.setBindGroup(0, bindGroup);
        pass.drawIndexed(6)
        pass.end()

        let commandBuffer = encoder.finish();
        device.queue.submit([commandBuffer]);
        t += 0.02 * settings.speed * 4;
        requestAnimationFrame(render);
    }

    await render();
})();

interface TextureData {
    width: number,
    height: number,
    data: Uint8Array,
}

function createTexture1(): TextureData {
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
        width: kTextureWidth,
        height: kTextureHeight,
        data: textureData,
    };
}

async function createTexture2(): Promise<TextureData> {
    let imageData = await getImageRawData(img);
    return {
        width: imageData.width,
        height: imageData.height,
        data: imageData.data,
    }
}
