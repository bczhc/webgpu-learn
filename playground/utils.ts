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

export interface ImageDataResult {
    data: Uint8Array;
    width: number;
    height: number;
}

export const getImageRawData = (url: string): Promise<ImageDataResult> => {
    return new Promise((resolve, reject) => {
        const img = new Image();
        // 处理跨域问题（如果图片来自其他域名）
        img.crossOrigin = 'anonymous';

        img.onload = () => {
            const canvas = document.createElement('canvas');
            const ctx = canvas.getContext('2d');

            if (!ctx) {
                reject(new Error('无法创建 Canvas 上下文'));
                return;
            }

            canvas.width = img.width;
            canvas.height = img.height;

            // 将图片绘制到画布
            ctx.drawImage(img, 0, 0);

            // 获取像素数据 (RGBA)
            const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);

            resolve({
                data: new Uint8Array(imageData.data.buffer), // 转化为 Uint8Array
                width: img.width,
                height: img.height
            });
        };

        img.onerror = reject;
        img.src = url;
    });
};

export interface MipmapLevel {
    data: Uint8Array;
    width: number;
    height: number;
}

/**
 * 生成多级 Mipmap
 * @param source 原始 RGBA 数据 (Uint8Array)
 * @param width 原始宽度
 * @param height 原始高度
 * @returns 包含所有层级的数组（Level 0 是原图）
 */
export function generateMipmaps(source: Uint8Array, width: number, height: number): MipmapLevel[] {
    const levels: MipmapLevel[] = [{ data: source, width, height }];

    let currentWidth = width;
    let currentHeight = height;
    let currentData = source;

    // 当宽度或高度大于 1 时，继续生成下一层级
    while (currentWidth > 1 || currentHeight > 1) {
        const nextWidth = Math.max(1, Math.floor(currentWidth / 2));
        const nextHeight = Math.max(1, Math.floor(currentHeight / 2));
        const nextData = new Uint8Array(nextWidth * nextHeight * 4);

        for (let y = 0; y < nextHeight; y++) {
            for (let x = 0; x < nextWidth; x++) {
                // 对应原图中 2x2 区域的起始索引
                const srcX = x * 2;
                const srcY = y * 2;

                const pixelOffsets = [
                    (srcY * currentWidth + srcX) * 4,             // 左上
                    (srcY * currentWidth + (srcX + 1)) * 4,       // 右上
                    ((srcY + 1) * currentWidth + srcX) * 4,       // 左下
                    ((srcY + 1) * currentWidth + (srcX + 1)) * 4  // 右下
                ];

                // 针对 RGBA 四个通道分别计算平均值
                for (let c = 0; c < 4; c++) {
                    let sum = 0;
                    let count = 0;

                    pixelOffsets.forEach(offset => {
                        // 边界检查：防止奇数尺寸时越界
                        if (offset < currentData.length) {
                            sum += currentData[offset + c];
                            count++;
                        }
                    });

                    nextData[(y * nextWidth + x) * 4 + c] = Math.round(sum / count);
                }
            }
        }

        currentWidth = nextWidth;
        currentHeight = nextHeight;
        currentData = nextData;

        levels.push({ data: currentData, width: currentWidth, height: currentHeight });
    }

    return levels;
}
