# art_match.py — 「屏幕上这一格到底是哪张美术」的数值化判据（2026-09-28，README §3.2n）
#
# 为什么需要它：逐窗 A/B 只能回答「两侧像不像」，答不了「这一格的**身份**对不对」。
# 典型问法是背包页签：切到第 N 页后，三张页签各自该显示 `Title[737/197]`、`[738/168/169]`、
# `[739/198]` 里的哪一张？靠肉眼看整幅 PNG 既慢又容易自相矛盾（LESSON_视觉模型读图…）。
# 这个工具把问题变成：给定**同一格**的候选美术索引清单，逐个算不符率，报最优者。
#
# 用法：
#   py -3.12 art_match.py --shot <帧.png> [--shot <帧2.png> …] \
#       --lib <Data/Title.Lib> --rect 6 7 72 23 --candidates 737,197 [--hotspot]
#   # 多格：--rect 可重复（第 i 个 --rect 对第 i 个 --candidates 组；组内用逗号分隔）
#   py -3.12 art_match.py --shot <帧.png> --lib <Data/Title.Lib> \
#       --rect 6 7 72 23   --candidates 737,197 \
#       --rect 76 7 72 23  --candidates 738,168,169 \
#       --rect 146 7 72 23 --candidates 739,198
# 退出码：0 = 每个 --rect 都有候选命中（不符率 ≤ 阈值）；1 = 有格子没有任何候选命中；2 = 前置不满足
#
# 口径（与 `belt_art_check.py` 同一套，二者共用本文件的 `load_lib/extract/crop_logical/compare`）：
#   * 只比美术里**不透明**的像素（`--alpha` 以上；可再用 `--dark` 把近黑当透明）——
#     透明像素处露的是背景，两边背景不同源时会把结论带偏；
#   * 矩形是**逻辑坐标**（1024x768 布局），截图按 `shot.width / 1024` 自动换算；
#   * C# `MirImageControl` 的 `Size` 可能小于图头尺寸（如 `Title[197]` 图头 72x24、
#     控件 `Size=(72,23)`）⇒ 原版是**裁剪**不是缩放，所以本条判据要求调用方给出的 rect
#     就是控件的 `Size`；裁剪由 `compare()` 里的 NEAREST 重采样兜住（尺寸一致时是 1:1）。
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


def art_mask(art, alpha=32, dark=0):
    """不透明像素掩码（`--alpha` 以上；`--dark` 非 0 时再把近黑当透明）。"""
    mask = Image.new("1", art.size)
    px = art.load()
    mp = mask.load()
    for y in range(art.size[1]):
        for x in range(art.size[0]):
            r, g, b, al = px[x, y]
            mp[x, y] = 1 if (al >= alpha and (dark == 0 or r + g + b >= dark * 3)) else 0
    return mask


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


def compare(art_rgb, art_mask_img, crop, tol):
    """返回 `(不符像素, 参与比拼像素, 不符像素列表)`；只比 `art_mask_img` 为真的像素。"""
    if crop.size != art_rgb.size:
        crop = crop.resize(art_rgb.size, Image.NEAREST)
    bad = 0
    total = 0
    bad_px = []
    a = art_rgb.load()
    m = art_mask_img.load()
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
    xs = [p[0] for p in bad_px]
    ys = [p[1] for p in bad_px]
    print(f"    不符像素包围盒 x[{min(xs)}..{max(xs)}] y[{min(ys)}..{max(ys)}]"
          f"（美术 {art_size[0]}x{art_size[1]}）")
    gw = max(1, art_size[0] // 8)
    gh = max(1, art_size[1] // 4)
    cells = {}
    for x, y, _, _ in bad_px:
        cells[(x // gw, y // gh)] = cells.get((x // gw, y // gh), 0) + 1
    rows = (art_size[1] + gh - 1) // gh
    cols = (art_size[0] + gw - 1) // gw
    print(f"    不符分布（{cols}x{rows} 粗网格）：")
    for cy in range(rows):
        line = "      "
        for cx in range(cols):
            n = cells.get((cx, cy), 0)
            line += "  ." if n == 0 else f"{n:3d}"
        print(line)
    # 边缘带统计：第一行/最后一行/第一列/最后一列的不符数（判"是不是差 1px"）
    top = sum(1 for _, y, _, _ in bad_px if y == 0)
    bottom = sum(1 for _, y, _, _ in bad_px if y == art_size[1] - 1)
    left = sum(1 for x, _, _, _ in bad_px if x == 0)
    right = sum(1 for x, _, _, _ in bad_px if x == art_size[0] - 1)
    print(f"    边缘带不符：上边界 {top} / 下边界 {bottom} / 左边界 {left} / 右边界 {right}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--shot", action="append", required=True, help="截图（整屏 PNG），可重复")
    ap.add_argument("--lib", required=True, help="候选美术所在 .Lib")
    ap.add_argument("--rect", type=int, nargs=4, action="append", required=True,
                    metavar=("X", "Y", "W", "H"), help="格子矩形（逻辑坐标），可重复")
    ap.add_argument("--candidates", action="append", required=True,
                    help="第 i 组候选美术索引（逗号分隔），与第 i 个 --rect 对应")
    ap.add_argument("--alpha", type=int, default=32, help="美术低于该 alpha 的像素当透明（不比对）")
    ap.add_argument("--dark", type=int, default=0, help="额外把美术近黑像素当透明（0 = 不启用）")
    ap.add_argument("--tol", type=int, default=60, help="单像素 RGB 差之和容差")
    ap.add_argument("--threshold", type=float, default=0.90, help="命中阈值（默认 0.90）")
    ap.add_argument("--hotspot", action="store_true", help="打印最优候选的不符像素分布")
    a = ap.parse_args()

    if len(a.rect) != len(a.candidates):
        print("FAIL(前置)：--rect 与 --candidates 数量必须一致", file=sys.stderr)
        return 2
    for p in [a.lib] + a.shot:
        if not os.path.exists(p):
            print(f"FAIL(前置)：找不到 {p}", file=sys.stderr)
            return 2
    data, offsets = load_lib(a.lib)
    groups = []
    for rect, cand in zip(a.rect, a.candidates):
        idxs = [int(t) for t in cand.replace(" ", "").split(",") if t]
        if not idxs:
            print(f"FAIL(前置)：--candidates {cand!r} 为空", file=sys.stderr)
            return 2
        arts = []
        for i in idxs:
            if i <= 0 or i >= len(offsets):
                print(f"FAIL(前置)：{os.path.basename(a.lib)} 索引 {i} 越界（count={len(offsets)}）",
                      file=sys.stderr)
                return 2
            art = extract(data, offsets[i])
            arts.append((i, art.convert("RGB"), art_mask(art, a.alpha, a.dark)))
        groups.append((tuple(rect), arts))

    fail = False
    for shot_path in a.shot:
        shot = Image.open(shot_path).convert("RGB")
        scale = shot.width / 1024.0
        print(f"== {os.path.basename(shot_path)}（scale={scale:g}）")
        for rect, arts in groups:
            crop = crop_logical(shot, scale, rect)
            results = []
            for idx, rgb, mask in arts:
                bad, total, bad_px = compare(rgb, mask, crop, a.tol)
                results.append((bad / max(total, 1), idx, bad, total, bad_px, rgb.size))
            results.sort()
            best = results[0]
            line = f"  rect{rect}: "
            line += " | ".join(
                f"[{idx}] {ratio:.3f}" for ratio, idx, _, _, _, _ in results)
            line += f"  ⇒ 最像 [{best[1]}]"
            ok = best[0] <= 1.0 - a.threshold
            print(line + ("" if ok else "  <-- 无候选命中"))
            if a.hotspot and best[4]:
                print(f"    最优候选 [{best[1]}] 不符像素分布：")
                print_hotspots(best[4], best[5])
            if not ok:
                fail = True
    if fail:
        print(f"VERDICT=FAIL：有格子在阈值 {a.threshold} 下没有任何候选命中")
        return 1
    print(f"VERDICT=PASS：所有格子都命中候选（阈值 {a.threshold}）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
