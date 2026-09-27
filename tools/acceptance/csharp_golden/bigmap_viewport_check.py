# bigmap_viewport_check.py — 大地图 / 小地图「画源+布局」的逐像素判据（2026-09-28，README §3.2h–§3.2i）
#
# 为什么需要它：`README §3.2g` 用**原版帧**证明了「大地图视口画的就是
# `Data/mmap.Lib[MapInfo.BigMap]` 缩放进 `min(568,W) x min(380,H)`」（一致率 98.3%），
# 那一侧依赖沙箱原版客户端。本端改成同一条路线后，判据可以**不依赖原版**：
# 直接拿本端截图与同一张 `mmap.Lib[index]` 比 —— 因为两侧画的是同一份美术、同一套布局。
#
# 与 `golden_ab_diff.py` 的分工：那个比"本端 vs 原版"（要两边都跑得起来、且地图数据同源才行）；
# 这个只比"本端 vs 原版会用的那张图"，**不需要原版客户端、也不受两侧地图数据版本差异影响**
# （`MapInfo.BigMap` 索引本端自己的 DB 给，两张图不同只说明地图数据不同源，不代表画错）。
#
# 用法：
#   # 大地图（整图缩放进 568x380，居中）
#   py -3.12 bigmap_viewport_check.py --shot <本端截图.png> --mmap <Data/mmap.Lib> --index 135
#   # 小地图（以玩家为中心裁 120x108，1:1；需要玩家瓦片与地图瓦片数）
#   py -3.12 bigmap_viewport_check.py --view mini --shot <本端截图.png> --mmap <Data/mmap.Lib> \
#       --index 101 --tile 288 616 --map 700 700
# 退出码：0 = 一致率 ≥ 阈值（VERDICT PASS）；1 = 低于阈值（FAIL）；2 = 前置不满足（缺文件/索引越界）
#
# 口径 ①`--view big`（C# `BigMapViewPort.OnBeforeDraw`，`Client/MirScenes/Dialogs/BigMapDialog.cs:642-676`）：
#   Size = Libraries.MiniMap.GetSize(BigMap)  ⇒ 画幅 w = min(568,W)、h = min(380,H)
#   画幅左上（面板内）= (14 + (568-w)/2, 52 + (380-h)/2)
#   整图 `Draw(index, DisplayLocation, Size)` ⇒ 图被**拉伸**到画幅（W>568 时缩小，反之原尺寸）
#   `MImage` 的纯黑像素当透明（画幅之外露的是 `Title[820]` 面板美术）⇒ 比对时跳过近黑像素
#
# 口径 ②`--view mini`（C# `MiniMapDialog`，`Client/MirScenes/Dialogs/MainDialogs.cs:1900-1930`）：
#   scale = mmap尺寸 / 地图瓦片数；窗口 120x108 以玩家为中心、先贴右/下再钳 0
#   `Libraries.MiniMap.Draw(map.MiniMap, viewRect, drawLocation+(3,22), White, fade)` ⇒ **1:1 裁剪**
#   （图比窗口大时只画那一块，不缩放）；面板 `Prguse[2090]` 128x154 @ (1024-126, 0)
import argparse
import os
import sys

from PIL import Image

PANEL = (760.0, 500.0)  # C# `BigMapDialog` 背景 `Title[820]`，面板居中
VIEW = (14.0, 52.0, 568.0, 380.0)
# 小地图（`--view mini`）：面板 `Prguse[2090]` 128x154 @ (898,0)，图区在其内 (3,22) 120x108
MINI_PANEL = (898.0, 0.0, 128.0, 154.0)
MINI_RECT = (3.0, 22.0, 120.0, 108.0)


def load_lib(path):
    import struct

    data = open(path, "rb").read()
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


def view_layout(art):
    """C# 画幅（面板内相对坐标）：`(x, y, w, h)`。"""
    w = min(VIEW[2], art[0])
    h = min(VIEW[3], art[1])
    return (VIEW[0] + (VIEW[2] - w) / 2.0, VIEW[1] + (VIEW[3] - h) / 2.0, w, h)


def mini_crop(art, map_wh, tile):
    """C# 小地图裁剪窗（**图像像素坐标**）：`(x, y, w, h)`。

    与 `Client-Bevy/src/game/dialogs/minimap.rs::minimap_view_rect` 同一口径
    （先贴右/下，再钳 0；`viewRect` 取整用的是 C# 的 `(int)`）。
    """
    w, h = MINI_RECT[2], MINI_RECT[3]
    sx = art[0] / max(map_wh[0], 1)
    sy = art[1] / max(map_wh[1], 1)
    x = int(sx * tile[0]) - int(w // 2)
    y = int(sy * tile[1]) - int(h // 2)
    if x + w >= art[0]:
        x = int(art[0] - w)
    if y + h >= art[1]:
        y = int(art[1] - h)
    x = max(0, x)
    y = max(0, y)
    return (x, y, w, h)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--shot", required=True, help="本端截图（整屏 PNG）")
    ap.add_argument("--mmap", required=True, help="含大图的 .Lib（本端 Data/mmap.Lib）")
    ap.add_argument("--index", type=int, required=True, help="MapInfo.BigMap")
    ap.add_argument("--view", choices=("big", "mini"), default="big",
                    help="big=大地图视口（默认）；mini=小地图 HUD 缩略图（需 --tile/--map）")
    ap.add_argument("--tile", type=int, nargs=2, metavar=("X", "Y"),
                    help="玩家瓦片坐标（mini 用；`state` RPC 的 tile_x/tile_y）")
    ap.add_argument("--map", type=int, nargs=2, metavar=("W", "H"),
                    help="地图瓦片数（mini 用；客户端日志 `🗺️ 地图 … 加载成功: 700x700`）")
    ap.add_argument("--dark", type=int, default=30, help="近黑阈值：图里低于它的像素当透明（不参与比对）")
    ap.add_argument("--tol", type=int, default=60, help="单像素 RGB 差之和容差")
    ap.add_argument("--threshold", type=float, default=0.90, help="一致率阈值（默认 0.90）")
    a = ap.parse_args()

    for p in (a.shot, a.mmap):
        if not os.path.exists(p):
            print(f"FAIL(前置)：找不到 {p}", file=sys.stderr)
            return 2
    data, offsets = load_lib(a.mmap)
    if a.index <= 0 or a.index >= len(offsets):
        print(f"FAIL(前置)：mmap 索引 {a.index} 越界（count={len(offsets)}）", file=sys.stderr)
        return 2

    if a.view == "mini" and (not a.tile or not a.map):
        print("FAIL(前置)：--view mini 需要 --tile X Y 与 --map W H", file=sys.stderr)
        return 2

    shot = Image.open(a.shot).convert("RGB")
    # 本端可能按系统 DPI 渲染（`--ui-scale`）；逻辑坐标恒 1024x768
    scale = shot.width / 1024.0
    art = extract(data, offsets[a.index])
    if a.view == "big":
        px = (1024.0 - PANEL[0]) / 2.0
        py = (768.0 - PANEL[1]) / 2.0
        (vx, vy, vw, vh) = view_layout((art.size[0], art.size[1]))
        # 大地图：把整张图缩放到画幅（C# `Draw(index, loc, Size)` 会拉伸）
        want = art.convert("RGB")
    else:
        px, py = MINI_PANEL[0], MINI_PANEL[1]
        (cx, cy, cw, ch) = mini_crop((art.size[0], art.size[1]), tuple(a.map), tuple(a.tile))
        (vx, vy, vw, vh) = (MINI_RECT[0], MINI_RECT[1], cw, ch)
        # 小地图：**1:1 裁剪**（不缩放）
        want = art.convert("RGB").crop((cx, cy, cx + cw, cy + ch))
    box = (
        int(round((px + vx) * scale)),
        int(round((py + vy) * scale)),
        int(round((px + vx + vw) * scale)),
        int(round((py + vy + vh) * scale)),
    )
    crop = shot.crop(box)
    if a.view == "big":
        want = want.resize(crop.size, Image.BILINEAR)

    same = 0
    total = 0
    for y in range(crop.height):
        for x in range(crop.width):
            r, g, b = want.getpixel((x, y))
            if r + g + b < a.dark * 3:
                continue  # 原版把纯黑当透明（露出面板美术）⇒ 不参与比对
            total += 1
            sr, sg, sb = crop.getpixel((x, y))
            if abs(sr - r) + abs(sg - g) + abs(sb - b) <= a.tol:
                same += 1
    ratio = same / max(total, 1)
    print(f"shot={os.path.basename(a.shot)} scale={scale:g}")
    if a.view == "big":
        print(f"mmap[{a.index}] {art.size[0]}x{art.size[1]} → 画幅 {vw:g}x{vh:g} @ 面板内 ({vx:g},{vy:g})")
    else:
        print(f"mmap[{a.index}] {art.size[0]}x{art.size[1]}，地图 {a.map[0]}x{a.map[1]}，玩家 {a.tile} "
              f"→ 裁剪 ({cx},{cy},{cw},{ch}) @ 面板内 ({vx:g},{vy:g})")
    print(f"比对像素 {total}（跳过近黑/透明），一致 {same}，一致率 {ratio:.3f}（阈值 {a.threshold}）")
    if ratio >= a.threshold:
        what = "大地图视口" if a.view == "big" else "小地图缩略图"
        print(f"VERDICT=PASS：{what}画源与布局 = C# 的 mmap.Lib 口径")
        return 0
    print("VERDICT=FAIL：与 mmap.Lib 的 C# 口径不一致（画源/裁剪/缩放有出入）")
    return 1


if __name__ == "__main__":
    sys.exit(main())
