import shader from "./main.wgsl?raw";
import GUI from "muigui";

let settings = {
    multisampling: 4,
};
let gui = new GUI();
gui.add(settings, 'multisampling');
;
(async () => {
    let canvas = document.querySelector('canvas')!!;
    let context = canvas.getContext('webgpu');

    let adapter = (await navigator.gpu.requestAdapter({
    }))!!;
    let device = await adapter.requestDevice({
        // requiredFeatures: ['texture-adapter-specific-format-features']
    });
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
        },
        fragment: {
            module: shaderModule,
            targets: [
                {format: textureFormat}
            ]
        },
        multisample: {
            count: settings.multisampling,
        }
    })

    function render() {
        let canvasTexture = context!!.getCurrentTexture();
        let multisampleTexture = device.createTexture({
            format: canvasTexture.format,
            usage: GPUTextureUsage.RENDER_ATTACHMENT,
            size: [canvasTexture.width, canvasTexture.height],
            sampleCount: settings.multisampling,
        });

        let encoder = device.createCommandEncoder();

        let pass = encoder.beginRenderPass({
            colorAttachments: [
                {
                    view: multisampleTexture.createView(),
                    resolveTarget: canvasTexture.createView(),
                    loadOp: 'clear',
                    storeOp: 'store',
                    clearValue: [0.3, 0.3, 0.3, 1],
                },
            ],
        });
        pass.setPipeline(pipeline);
        pass.draw(3);
        pass.end()

        let commandBuffer = encoder.finish();
        device.queue.submit([commandBuffer]);
    }

    gui.onChange(() => {
        render();
    })

    render();
})();
