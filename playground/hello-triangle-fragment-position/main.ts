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
        }
    })

    function render() {
        let encoder = device.createCommandEncoder();

        let pass = encoder.beginRenderPass({
            colorAttachments: [
                {
                    view: context!!.getCurrentTexture(),
                    loadOp: 'clear',
                    storeOp: 'store',
                    clearValue: [0.3,0.3,0.3,1],
                },
            ],
        });
        pass.setPipeline(pipeline);
        pass.draw(3);
        pass.end()

        let commandBuffer = encoder.finish();
        device.queue.submit([commandBuffer]);
    }

    render();
})();
