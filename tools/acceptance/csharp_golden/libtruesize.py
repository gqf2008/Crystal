# libtruesize.py — 算 Mir2 `.Lib` 某帧的**图头尺寸**与 C# `MImage.GetTrueSize()` 的**真尺寸**
#
# 为什么需要它：2026-10-01 更正「居中窗 +1px 口径」时确认——
#   `MirImageControl.Size` getter 在 `AutoSize`（构造器默认 true）下直接返回 `Library.GetTrueSize(Index)`
#   （`Client/MirControls/MirImageControl.cs:142-151`），而所有 `Location = …Size.Width…` / `Center`
#   用的就是这个 getter ⇒ **布局/命中/裁剪一律按真尺寸**；贴图本身仍由 `Library.Draw` 按图头 1:1 铺。
#   `GetTrueSize` = 裁掉 `alpha==0` 的边（`Client/MirGraphics/MLibrary.cs:1050-1127`，逐列/逐行找首个可见像素）。
#   ⚠ 判据用的是**原始 alpha 字节**（`VisiblePixel` 只读 Data 的 alpha，不做「纯黑当透明」，见 `:1027-1048`）。
#
# 用法：
#   py -3.12 libtruesize.py --data Data Title:820 Prguse2:197 Prguse:1341
#   py -3.12 libtruesize.py --data Data --file list.txt          # 每行 `Lib:idx`，`#` 注释
#   py -3.12 libtruesize.py --data Data --json out.json Title:820
# 输出：`Lib[idx]  图头 WxH  真尺寸 WxH  裁剪框(l,t,r,b)  漂移符号`
import argparse
import gzip
import json
import os
import struct
import sys


def load(path):
    with open(path, "rb") as f:
        data = f.read()
    version, count = struct.unpack_from("<ii", data, 0)
    if version < 2:
        raise SystemExit(f"unsupported lib version {version}: {path}")
    off = 8 + (4 if version >= 3 else 0)
    offsets = list(struct.unpack_from(f"<{count}i", data, off))
    return data, version, offsets


def frame(data, offset):
    w, h, x, y, sx, sy, shadow, length = struct.unpack_from("<hhhhhhBi", data, offset)
    raw = gzip.decompress(data[offset + 17 : offset + 17 + length])
    need = w * h * 4
    if len(raw) < need:
        raw = raw + bytes(need - len(raw))
    return w, h, raw[:need]


def true_size(w, h, bgra):
    """精确复刻 MImage.GetTrueSize()（MLibrary.cs:1050-1127）。

    注意几个与直觉不同的点，必须照抄：
      * 全透明帧返回 (Width, Height)，不是 (0,0)（l/t 初值不被覆盖）；
      * 找 r 时内层扫的是**整幅高度**（b 此时还是原值），找 b 时才用更新后的 r。
    """

    def visible(x, y):
        return bgra[(y * w + x) * 4 + 3] != 0

    l, t, r, b = 0, 0, w, h

    for x in range(r):
        if any(visible(x, y) for y in range(b)):
            l = x
            break

    for y in range(b):
        if any(visible(x, y) for x in range(l, r)):
            t = y
            break

    for x in range(r - 1, l - 1, -1):
        if any(visible(x, y) for y in range(b)):
            r = x + 1
            break

    for y in range(b - 1, t - 1, -1):
        if any(visible(x, y) for x in range(l, r)):
            b = y + 1
            break

    return (r - l, b - t), (l, t, r, b)


def resolve_lib(data_dir, name):
    for cand in (name, name + ".Lib", name + ".lib"):
        p = os.path.join(data_dir, cand)
        if os.path.exists(p):
            return p
    raise SystemExit(f"找不到库文件：{name}（在 {data_dir} 下）")


def main(argv=None):
    ap = argparse.ArgumentParser()
    ap.add_argument("--data", required=True, help="含 *.Lib 的 Data 目录")
    ap.add_argument("--file", help="每行 `Lib:idx`，`#` 开头为注释")
    ap.add_argument("--json", help="把结果写成 JSON")
    ap.add_argument("specs", nargs="*", help="`Lib:idx`，例如 Title:820")
    a = ap.parse_args(argv)

    specs = list(a.specs)
    if a.file:
        with open(a.file, "r", encoding="utf-8") as f:
            for line in f:
                line = line.split("#", 1)[0].strip()
                if line:
                    specs.append(line)
    if not specs:
        ap.error("至少给一个 `Lib:idx`")

    rows = []
    cache = {}
    for spec in specs:
        lib_name, _, idx_s = spec.partition(":")
        if not idx_s.strip():
            ap.error(f"格式应为 `Lib:idx`，收到 {spec!r}")
        idx = int(idx_s)
        key = os.path.basename(lib_name)
        if key not in cache:
            path = resolve_lib(a.data, lib_name)
            cache[key] = load(path)
        data, version, offsets = cache[key]
        if not (0 <= idx < len(offsets)):
            print(f"{key}[{idx}]  越界（count={len(offsets)}）")
            continue
        w, h, bgra = frame(data, offsets[idx])
        (tw, th), (l, t, r, b) = true_size(w, h, bgra)
        rows.append(
            {
                "lib": key,
                "index": idx,
                "header": [w, h],
                "true": [tw, th],
                "box": [l, t, r, b],
                "dw": tw - w,
                "dh": th - h,
            }
        )
        flag = "" if (tw == w and th == h) else "  <= 真尺寸≠图头"
        print(
            f"{key}[{idx}]  图头 {w}x{h}  真尺寸 {tw}x{th}  "
            f"裁剪框 ltrb=({l},{t},{r},{b})  Δ=({tw - w:+d},{th - h:+d}){flag}"
        )

    if a.json:
        with open(a.json, "w", encoding="utf-8") as f:
            json.dump(rows, f, ensure_ascii=False, indent=2)
        print(f"-> {a.json}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
