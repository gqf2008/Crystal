# npc_page_probe.py — 把「原版 C# 这一帧是不是 NPC 窗 / 是哪一种页」变成数值判据（2026-09-28，README §3.2z）
#
# 为什么需要它：原版侧点 NPC 只能在世界里盲点（§3.2q 的格点扫描），点完之后**肉眼判页型**既慢又
# 会骗自己 —— §3.2x 就被「底部 QUEST 按钮的黄字」骗过一次（当成商人链接）。这个工具只读像素：
#   ① 窗美术判据：把窗区与 `Data/Prguse.Lib[995]`（NPC 窗美术 440x224 @ (0,0)）比**不透明像素不符率**，
#      ≤ 阈值 ⇒ 这一帧 NPC 窗开着；关着时 ~0.95（§3.2q 实测 40 倍区分度）。
#   ② 页型判据：窗区里扫黄字（`R>200 && G>200 && B<80`，与 §3.2r 同一口径 —— C# 的链接标签
#      `NewButton` 就是 `ForeColour = Color.Yellow`），按 y 相邻分带；再把「贴左边（x0 ≤ link-xmax）
#      的短带」当链接。商人页 ≥2 条、Assistant 只有 `Close` 一条；底部 QUEST 按钮落在
#      `y ≥ link-ymax`（C# `QuestButton.Location = (172, Size.Height-30)` = (172,194)），一律排除。
#
# 用法：
#   py -3.12 npc_page_probe.py --prguse <Data\Prguse.Lib> --shot a.png [--shot b.png …] [--json out.json]
#   # 可选：--goods 同时判商品窗（Prguse[1000] @ (0,224) 244x334）
#
# 退出码：0 = 至少一帧 NPC 窗开着；1 = 全部没开窗；2 = 前置不满足
#
# 与 `art_match.py` / `belt_art_check.py` 共用同一套 lib 解析与比对口径（直接 import 复用，
# 不另抄一份 —— 口径分叉是这类工具最容易长出来的债）。
import argparse
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from PIL import Image  # noqa: E402

from art_match import art_mask, compare, crop_logical, extract, load_lib  # noqa: E402


def is_yellow(rgb):
    r, g, b = rgb[:3]
    return r > 200 and g > 200 and b < 80


def yellow_bands(px, x0, x1, y0, y1, gap=2, min_px=2):
    """窗区内扫黄字、按 y 分带；返回 `[{y0,y1,x0,x1,n}]`（x 范围是该带内黄字的 min/max）。"""
    rows = {}
    for y in range(y0, y1):
        xs = [x for x in range(x0, x1) if is_yellow(px[x, y])]
        if len(xs) >= min_px:
            rows[y] = (min(xs), max(xs), len(xs))
    bands = []
    for y in sorted(rows):
        lo, hi, n = rows[y]
        if bands and y - bands[-1]["y1"] <= gap:
            b = bands[-1]
            b["y1"] = y
            b["x0"] = min(b["x0"], lo)
            b["x1"] = max(b["x1"], hi)
            b["n"] += n
        else:
            bands.append({"y0": y, "y1": y, "x0": lo, "x1": hi, "n": n})
    for b in bands:
        b["w"] = b["x1"] - b["x0"] + 1
        b["h"] = b["y1"] - b["y0"] + 1
    return bands


def classify_page(bands, link_xmax, link_ymin, link_ymax, wide):
    body = [b for b in bands if b["y0"] >= link_ymin and b["y1"] <= link_ymax]
    links = [b for b in body if b["x0"] <= link_xmax and b["w"] <= wide]
    wide_bands = [b for b in body if b["w"] > wide]
    if not body:
        page = "no-yellow-text"
    elif len(links) >= 2:
        page = "merchant"          # 商人页：`View` + `Ask` 一类多条左对齐链接
    elif len(links) == 1:
        page = "single-link"       # Assistant 一类只有一条链接
    elif wide_bands:
        page = "board"             # 布告板：一条长黄字，x0 偏右
    else:
        page = "other"
    return page, links, wide_bands


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--shot", action="append", required=True, help="截图（整屏 PNG），可重复")
    ap.add_argument("--data", help="原版 Data 目录（含 Prguse.Lib）；给了就不必再给 --prguse/--title")
    ap.add_argument("--prguse", help="Data\\Prguse.Lib 的完整路径")
    ap.add_argument("--rect", type=int, nargs=4, default=[0, 0, 440, 224], help="NPC 窗矩形（逻辑坐标）")
    ap.add_argument("--index", type=int, default=995, help="NPC 窗美术索引（默认 995）")
    ap.add_argument("--goods-rect", type=int, nargs=4, default=[0, 224, 244, 334])
    ap.add_argument("--goods-index", type=int, default=1000)
    ap.add_argument("--threshold", type=float, default=0.10, help="窗美术不符率阈值（≤ ⇒ 窗开着）")
    ap.add_argument("--alpha", type=int, default=32)
    ap.add_argument("--tol", type=int, default=60)
    ap.add_argument("--dark", type=int, default=0)
    ap.add_argument("--link-xmax", type=int, default=60, help="链接带允许的最大 x0（贴左边）")
    ap.add_argument("--link-ymin", type=int, default=30)
    ap.add_argument("--link-ymax", type=int, default=170, help="排除底部 QUEST 按钮的黄字")
    ap.add_argument("--wide", type=int, default=70, help="宽于它就不算链接（布告板那类长黄字）")
    ap.add_argument("--json", help="把结果写成 JSON 文件")
    a = ap.parse_args()

    prguse = a.prguse or (os.path.join(a.data, "Prguse.Lib") if a.data else None)
    if not prguse or not os.path.exists(prguse):
        print("FAIL(前置)：找不到 Prguse.Lib（给 --data 或 --prguse）", file=sys.stderr)
        return 2
    for p in a.shot:
        if not os.path.exists(p):
            print(f"FAIL(前置)：找不到 {p}", file=sys.stderr)
            return 2

    blob, offsets = load_lib(prguse)

    def _ratio(rect, index):
        if index <= 0 or index >= len(offsets):
            return None, 0, 0
        art = extract(blob, offsets[index])
        mask = art_mask(art, a.alpha, a.dark)
        scale = shot.width / 1024.0
        crop = crop_logical(shot, scale, rect)
        bad, total, _ = compare(art.convert("RGB"), mask, crop, a.tol)
        return bad / max(total, 1), bad, total

    results = []
    any_open = False
    for shot_path in a.shot:
        shot = Image.open(shot_path).convert("RGB")
        ratio, bad, total = _ratio(a.rect, a.index)
        rec = {
            "shot": os.path.abspath(shot_path),
            "npc_index": a.index,
            "npc_ratio": None if ratio is None else round(ratio, 4),
            "npc_bad": bad,
            "npc_total": total,
            "npc_open": ratio is not None and ratio <= a.threshold,
        }
        if rec["npc_open"]:
            any_open = True
            x, y, w, h = a.rect
            px = shot.load()
            bands = yellow_bands(px, x, x + w, y, y + h)
            page, links, wide_bands = classify_page(
                bands, a.link_xmax, a.link_ymin, a.link_ymax, a.wide)
            rec["bands"] = bands
            rec["links"] = links
            rec["wide_bands"] = wide_bands
            rec["page"] = page
        else:
            rec["page"] = "closed"
        if a.goods_index:
            gr, gbad, gtot = _ratio(a.goods_rect, a.goods_index)
            rec["goods_ratio"] = None if gr is None else round(gr, 4)
            rec["goods_open"] = gr is not None and gr <= a.threshold
        results.append(rec)

    for r in results:
        tag = f"{os.path.basename(r['shot']):<34} NPC窗={'开' if r['npc_open'] else '关'}"
        tag += f" 不符率={r['npc_ratio']}"
        tag += f" 页型={r['page']}"
        if r["npc_open"]:
            tag += " 链接=" + "-".join(
                f"x{b['x0']}..{b['x1']},y{b['y0']}..{b['y1']}" for b in r["links"])
        if "goods_open" in r:
            tag += f" 商品窗={'开' if r['goods_open'] else '关'}({r['goods_ratio']})"
        print(tag)
    if a.json:
        with open(a.json, "w", encoding="utf-8") as f:
            json.dump(results, f, ensure_ascii=False, indent=2)
    return 0 if any_open else 1


if __name__ == "__main__":
    sys.exit(main())
