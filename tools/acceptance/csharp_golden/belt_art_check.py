# belt_art_check.py — 腰带 HUD「画源」逐像素判据（2026-09-28，README §3.2m）
#
# 为什么需要它：§3.2k 先用「只比不透明像素」的口径捞出本端腰带**整块被压暗**
# （`Prguse[1932]` 的不透明像素只有 56.5% 相符，原版同一帧 96.7%）。这条判据不依赖
# 原版客户端、也不需要两侧地图数据同源：腰带面板是**恒定位置 + 固定美术**
# （C# `BeltDialog`：`MainDialog.X + 230, MainDialog.Y + 618`，图 = `Libraries.Prguse[1932]`），
# 所以「本端截图那一块 == 1932 美术」就是判据本身。
#
# 与 `golden_ab_diff.py` 的分工：那个比「本端 vs 原版整帧」（要两边都跑得起来、
# 且状态摆成同一档才判得出窗内）；这个只比**本端 vs 原版会画的那张图**。
#
# 用法：
#   py -3.12 belt_art_check.py --shot <截图.png> [--shot <另一张.png> ...] \
#       --lib <Data/Prguse.Lib> --index 1932 --rect 230 618 240 38
#   # 立式腰带（C# `vert`）：--rect 230 230 38 240 --index 1945
#   # 位置诊断：--offset-search 6（在 ±6 逻辑像素内穷举，打印最优偏移）
# 退出码：0 = 全部帧一致率 ≥ 阈值（VERDICT PASS）；1 = 有帧低于阈值（FAIL）；2 = 前置不满足
#
# 口径（C# `Client/MirScenes/Dialogs/MainDialogs.cs` BeltDialog）：
#   * 背景 = `Libraries.Prguse.Draw(1932, new Point(MainDialog.X + 230, MainDialog.Y + 618), true)`
#     —— `MainDialog` = `Prguse[1]` 1024x152 居中 ⇒ X=0、Y=618 ⇒ 面板屏幕矩形 (230,618,240,38)；
#   * `BeltPanel_BeforeDraw` 里那层 `Prguse.Draw(Index + 1, …, 0.5F)`（= 1933/1945）
#     跑在**控件自己那张图之前**，且面板的**透明像素**处露的是**世界**（不是这层的近黑）
#     ⇒ 该层在可见画面里不出现，比对时**只看 1932 的不透明像素**（透明像素两侧内容不同源）。
import argparse
import os
import sys

from PIL import Image


def load_lib(path):
    import struct

    with open(path, "rb") as f:
        data = f.read()
    version, count = struct.unpack_from("<ii", data, 0)
    if version < 2:
        raise SystemExit(f"unsupported lib version {version}: {path}")
    off = 8 + (4 if version >= 3 else 0)
    return data, list(struct.unpack_from(f"<{count}i", data, off))


def extract(data, offset):
    import gzip
    import struct

    w, h, _x, _y, _sx, _sy, _shadow, length = struct.unpack_from("<hhhhhhBi", data, offset)
    raw = gzip.decompress(data[offset + 17 : offset + 17 + length])
    need = w * h * 4
    if len(raw) < need:
        raw = raw + bytes(need - len(raw))
    return Image.frombytes("RGBA", (w, h), raw[:need], "raw", "BGRA")


def crop_logical(shot, scale, rect, dx=0, dy=0):
    x, y, w, h = rect
    box = (
        int(round((x + dx) * scale)),
        int(round((y + dy) * scale)),
        int(round((x + dx + w) * scale)),
        int(round((y + dy + h) * scale)),
    )
    box = (max(0, box[0]), max(0, box[1]), max(0, box[2]), max(0, box[3]))
    return shot.crop(box)


def compare(art_rgb, art_mask, crop, tol):
    """返回 `(不符像素, 参与比拼像素, 不符像素列表)`。`art_mask` 为 True 的才比对（不透明像素）。"""
    if crop.size != art_rgb.size:
        crop = crop.resize(art_rgb.size, Image.NEAREST)
    bad = 0
    total = 0
    bad_px = []
    a = art_rgb.load()
    m = art_mask.load()
    c = crop.load()
    for y in range(art_rgb.size[1]):
        for x in range(art_rgb.size[0]):
            if not m[x, y]:
                continue
            total += 1
            r, g, b = a[x, y]
            sr, sg, sb = c[x, y]
            if abs(sr - r) + abs(sg - g) + abs(sb - b) > tol:
                bad += 1
                bad_px.append((x, y, (r, g, b), (sr, sg, sb)))
    return bad, total, bad_px


def print_hotspots(bad_px, art_size):
    """把不符像素压成「靠边缘带 + 粗网格」两个读数（比看整幅 PNG 省事且可复现）。"""
    xs = [p[0] for p in bad_px]
    ys = [p[1] for p in bad_px]
    print(f"    不符像素包围盒 x[{min(xs)}..{max(xs)}] y[{min(ys)}..{max(ys)}]"
          f"（美术 {art_size[0]}x{art_size[1]}）")
    # 粗网格：把美术切成 8x4 格，打印每格不符数（`##` = ≥20，`.` = 0）
    gw = max(1, art_size[0] // 8)
    gh = max(1, art_size[1] // 4)
    cells = {}
    for x, y, _, _ in bad_px:
        cells[(x // gw, y // gh)] = cells.get((x // gw, y // gh), 0) + 1
    rows = (art_size[1] + gh - 1) // gh
    cols = (art_size[0] + gw - 1) // gw
    print(f"    不符分布（{cols}x{rows} 粗网格，格内不符数）：")
    for cy in range(rows):
        line = "      "
        for cx in range(cols):
            n = cells.get((cx, cy), 0)
            line += "  ." if n == 0 else f"{n:3d}"
        print(line)
    # 边缘带统计：第一行/最后一行/第一列/最后一列的不符数（用来判"是不是差 1px"）
    top = sum(1 for _, y, _, _ in bad_px if y == 0)
    bottom = sum(1 for _, y, _, _ in bad_px if y == art_size[1] - 1)
    left = sum(1 for x, _, _, _ in bad_px if x == 0)
    right = sum(1 for x, _, _, _ in bad_px if x == art_size[0] - 1)
    print(f"    边缘带不符：上边界 {top} / 下边界 {bottom} / 左边界 {left} / 右边界 {right}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--shot", action="append", required=True, help="截图（整屏 PNG），可重复")
    ap.add_argument("--lib", required=True, help="含面板美术的 .Lib（本端 Data/Prguse.Lib）")
    ap.add_argument("--index", type=int, default=1932, help="面板图索引（默认 1932 腰带）")
    ap.add_argument("--rect", type=int, nargs=4, required=True, metavar=("X", "Y", "W", "H"),
                    help="面板屏幕矩形（逻辑坐标，默认 1024x768 布局）")
    ap.add_argument("--alpha", type=int, default=32, help="美术低于该 alpha 的像素当透明（不比对）")
    ap.add_argument("--dark", type=int, default=0,
                    help="额外把美术近黑像素当透明（0 = 不启用；C# 的 MImage 把纯黑当透明）")
    ap.add_argument("--tol", type=int, default=60, help="单像素 RGB 差之和容差")
    ap.add_argument("--threshold", type=float, default=0.90, help="一致率阈值（默认 0.90）")
    ap.add_argument("--offset-search", type=int, default=0,
                    help="在 ±N 逻辑像素内穷举最优偏移（位置诊断；0 = 只比 (0,0)）")
    ap.add_argument("--hotspot", action="store_true",
                    help="打印不符像素的包围盒与粗网格分布（定位残差来自哪一块）")
    a = ap.parse_args()

    for p in [a.lib] + a.shot:
        if not os.path.exists(p):
            print(f"FAIL(前置)：找不到 {p}", file=sys.stderr)
            return 2
    data, offsets = load_lib(a.lib)
    if a.index <= 0 or a.index >= len(offsets):
        print(f"FAIL(前置)：{os.path.basename(a.lib)} 索引 {a.index} 越界（count={len(offsets)}）",
              file=sys.stderr)
        return 2

    art = extract(data, offsets[a.index])
    base = art.convert("RGB")
    mask = Image.new("1", art.size)
    px = art.load()
    mp = mask.load()
    for y in range(art.size[1]):
        for x in range(art.size[0]):
            r, g, b, al = px[x, y]
            opaque = al >= a.alpha and (a.dark == 0 or r + g + b >= a.dark * 3)
            mp[x, y] = 1 if opaque else 0
    opaque_total = sum(1 for y in range(art.size[1]) for x in range(art.size[0]) if mp[x, y])
    print(f"美术 {os.path.basename(a.lib)}[{a.index}] {art.size[0]}x{art.size[1]}，"
          f"不透明像素 {opaque_total}（比对基准）")
    print(f"面板矩形 {tuple(a.rect)}（逻辑坐标）")

    worst = 0.0
    for shot_path in a.shot:
        shot = Image.open(shot_path).convert("RGB")
        scale = shot.width / 1024.0
        box = tuple(a.rect)
        crop = crop_logical(shot, scale, box)
        if crop.size != base.size:
            print(f"  注意：{os.path.basename(shot_path)} 裁剪 {crop.size} ≠ 美术 {base.size}"
                  f"（按 NEAREST 重采样后比对）")
        best = None
        if a.offset_search > 0:
            for dy in range(-a.offset_search, a.offset_search + 1):
                for dx in range(-a.offset_search, a.offset_search + 1):
                    c = crop_logical(shot, scale, box, dx, dy)
                    bad, total, _ = compare(base, mask, c, a.tol)
                    ratio = bad / max(total, 1)
                    if best is None or ratio < best[0]:
                        best = (ratio, dx, dy, bad, total)
        bad, total, bad_px = compare(base, mask, crop, a.tol)
        ratio = bad / max(total, 1)
        line = (f"{os.path.basename(shot_path)}: 比对 {total} 不透明像素，"
                f"不符 {bad}，不符率 {ratio:.3f}")
        if best is not None and (best[1], best[2]) != (0, 0):
            line += f"；±{a.offset_search}px 最优偏移 ({best[1]},{best[2]}) 不符率 {best[0]:.3f}"
        print(line)
        if a.hotspot and bad_px:
            print_hotspots(bad_px, art.size)
        worst = max(worst, ratio)

    if worst <= 1.0 - a.threshold:
        print(f"VERDICT=PASS：腰带区与 {os.path.basename(a.lib)}[{a.index}] 一致"
              f"（最差不符率 {worst:.3f} ≤ {1.0 - a.threshold:.3f}）")
        return 0
    print(f"VERDICT=FAIL：腰带区与 {os.path.basename(a.lib)}[{a.index}] 不一致"
          f"（最差不符率 {worst:.3f} > {1.0 - a.threshold:.3f}）")
    return 1


if __name__ == "__main__":
    sys.exit(main())
