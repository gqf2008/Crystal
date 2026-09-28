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

# 与 `art_match.py` **共用同一套**解析/取格/比对实现（单一来源：两处各写一份必然漂移）。
# 本脚本是它的"单候选 + 位移诊断 + 边缘带"专用变体（腰带那类**位置恒定、图唯一**的 HUD）。
from art_match import art_mask, compare, crop_logical, extract, load_lib, print_hotspots


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
    mask = art_mask(art, a.alpha, a.dark)
    mp = mask.load()
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
