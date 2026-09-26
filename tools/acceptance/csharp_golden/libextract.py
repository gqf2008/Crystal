# libextract.py — 从 Mir2 `.Lib` 图像库里按索引导出 PNG（金标准 A/B 对拍用）
#
# 为什么需要它：判断「某个像素是谁画的」时，读 C# 源码只能给出*索引*，给不出*画面*。
# 2026-09-26 核对背包窗标题栏时，把 `Title[196/197/738/739/483]` 逐张导出来看，
# 直接确认了「原版美术本身长什么样」，不必再靠对源码推断（也不再需要开两个客户端）。
#
# 格式（对应 `Client/MirGraphics/MLibrary.cs` 与 MapEditor 的 Rust 实现）：
#   header: i32 version, i32 count, [i32 frame_seek if version>=3]
#   count × i32 偏移
#   图像: i16 w, i16 h, i16 x, i16 y, i16 shadowX, i16 shadowY, u8 shadow(bit7=mask), i32 len, gzip(BGRA)
#   有 mask 时再跟 mask 的 12 字节头 + gzip 数据（本工具只导主图）
#
# 用法：py -3.12 libextract.py <Lib文件> <输出目录> 196 197 738 739 483
import gzip
import os
import struct
import sys

from PIL import Image


def load(path):
    with open(path, "rb") as f:
        data = f.read()
    version, count = struct.unpack_from("<ii", data, 0)
    if version < 2:
        raise SystemExit(f"unsupported lib version {version}: {path}")
    off = 8 + (4 if version >= 3 else 0)
    offsets = list(struct.unpack_from(f"<{count}i", data, off))
    return data, version, offsets


def extract(data, offset):
    w, h, x, y, sx, sy, shadow, length = struct.unpack_from("<hhhhhhBi", data, offset)
    raw = gzip.decompress(data[offset + 17 : offset + 17 + length])
    need = w * h * 4
    if len(raw) < need:
        raw = raw + bytes(need - len(raw))
    raw = raw[:need]
    # 库内是 BGRA；纯黑不透明像素按原版语义当透明（MImage 的黑色背景）
    img = Image.frombytes("RGBA", (w, h), raw, "raw", "BGRA")
    px = img.load()
    for yy in range(h):
        for xx in range(w):
            r, g, b, a = px[xx, yy]
            if r == 0 and g == 0 and b == 0 and a == 255:
                px[xx, yy] = (0, 0, 0, 0)
    return img, (x, y, sx, sy, shadow)


def main():
    lib, outdir, indices = sys.argv[1], sys.argv[2], [int(v) for v in sys.argv[3:]]
    os.makedirs(outdir, exist_ok=True)
    data, version, offsets = load(lib)
    print(f"{os.path.basename(lib)}: version={version} count={len(offsets)}")
    for idx in indices:
        if idx < 0 or idx >= len(offsets):
            print(f"  [{idx}] 越界（count={len(offsets)}）")
            continue
        img, meta = extract(data, offsets[idx])
        dst = os.path.join(outdir, f"{os.path.splitext(os.path.basename(lib))[0]}_{idx}.png")
        img.save(dst)
        print(f"  [{idx}] {img.size[0]}x{img.size[1]} x/y=({meta[0]},{meta[1]}) -> {dst}")


if __name__ == "__main__":
    main()
