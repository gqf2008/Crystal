"""逐窗像素 A/B：拿 golden_kbd_windows.ps1（原版）与 golden_ab_ours.ps1（我方）两套帧，
按 C# 期望矩形逐窗比「差异像素占比」，输出表 + JSON。

用法：py -3.12 golden_ab_diff.py --shots <sandbox\\shots> --table %TEMP%\\rect_table.json [--out ab_result.json]

为什么要按**期望矩形**裁区域而不是整帧：两台客户端的地图视角/角色不同（原版那只是女道士、
我方是 bevychar），整帧差异几乎全来自地图与角色，判据会失明。窗口矩形是 C# 常量（窗口级几何
已用 window_rect_table.py 验过 0 差异），裁到它上面，差异才归因到「窗内绘制」。
"""
import argparse
import json
import os
import sys

from PIL import Image, ImageChops


def rects_from_table(path):
    rows = json.load(open(path, encoding="utf-8"))
    out = {}
    for r in rows:
        kind = r.get("kind")
        if not kind or r.get("x") is None or not r.get("expect"):
            continue
        out.setdefault(kind, (r["x"], r["y"], r["expect"][0], r["expect"][1]))
    return out


def diff_region(a, b, box):
    x0, y0, w, h = box
    x1, y1 = x0 + w, y0 + h
    x0 = max(0, min(x0, a.width - 1))
    y0 = max(0, min(y0, a.height - 1))
    x1 = max(x0 + 1, min(x1, a.width))
    y1 = max(y0 + 1, min(y1, a.height))
    ca, cb = a.crop((x0, y0, x1, y1)), b.crop((x0, y0, x1, y1))
    d = ImageChops.difference(ca, cb)
    bbox = d.getbbox()
    n = 0
    if bbox:
        px = d.load()
        for y in range(bbox[1], bbox[3]):
            for x in range(bbox[0], bbox[2]):
                if sum(px[x, y]) > 12:
                    n += 1
    total = (x1 - x0) * (y1 - y0)
    return n, total, (x0, y0, x1, y1)


def _changed_vs_base(base_path, frame_path, box, scale):
    """该侧在自己那块区域**相对本侧基线**有没有变化（= 这一侧真的出了窗）。

    返回 True/False/None（None = 缺基线帧，判不了）。判据取"变化像素占比 > 0.5%"——
    阈值取小：一个 200x287 的窗只要画出来，占比远不止 0.5%；而纯采样噪声达不到。
    """
    if not base_path or not os.path.exists(base_path) or not os.path.exists(frame_path):
        return None
    ia = Image.open(base_path).convert("RGB")
    ib = Image.open(frame_path).convert("RGB")
    if scale != 1.0:
        ia = ia.resize(ib.size, Image.NEAREST)
    n, total, _ = diff_region(ia, ib, box)
    return (n / max(total, 1)) > 0.005


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--shots", required=True)
    ap.add_argument("--table", required=True)
    ap.add_argument("--out", default="")
    a = ap.parse_args()

    pairs = json.load(open(os.path.join(a.shots, "ab_windows.json"), encoding="utf-8"))
    rects = rects_from_table(a.table)
    # 两侧各自的**基线帧**（无窗口）：用来判「这一侧到底有没有在这块区域出窗」。
    # 为什么必须有这一步：C# 侧很多键在**当前角色状态下不开窗**（例如没拿钓竿按 N 什么都不出、
    # 没行会按 G 弹的是 MirMessageBox）——那种图的"差异"再大也和窗内绘制无关，
    # 直接报 90%+ 会把人引去改对的代码（第一版就是这么误判的）。
    # 原版基线必须是**干净**那张（golden_kbd_windows.ps1 开头 Escape 后拍的）；
    # `orig_kbd_02_ingame.png` 是 F9/F10 之后拍的、自带两扇窗，不能当基线。
    base_orig = os.path.join(a.shots, "orig_baseline_none.png")
    base_ours = os.path.join(a.shots, "ours_kbd_02_ingame.png")
    results = []
    print(f"{'action':16s} {'kind':22s} {'区域(x,y,w,h)':>22s} {'差异像素':>10s} {'占比':>7s}  判定")
    for p in pairs:
        kind = p.get("kind")
        orig, ours = p.get("orig"), p.get("ours")
        if not orig or not ours or not (os.path.exists(orig) and os.path.exists(ours)):
            print(f"{p.get('action',''):16s} {kind or '-':22s} {'(缺帧)':>22s} {'-':>10s} {'-':>7s}  SKIP")
            continue
        ia = Image.open(orig).convert("RGB")
        ib = Image.open(ours).convert("RGB")
        # 我方是按系统 DPI 渲染的（例如 1.5x，逻辑仍是 1024x768），原版恒 1024x768。
        # **把原版放大到我这边的尺寸（NEAREST），不要把我的缩下去（LANCZOS）**：缩下去会把
        # 面板边缘/字体糊掉，30%+ 的"差异"其实是重采样噪声（第一版就是这么误判的）。
        # 整数倍放大只复制像素，对纯色面板/精灵边框是精确的。
        scale = 1.0
        if ia.size != ib.size:
            scale = ib.width / ia.width
            ia = ia.resize(ib.size, Image.NEAREST)
        box = rects.get(kind)
        if box is None:
            # HUD 类（腰带/技能栏）与英雄技能页没有 C# 窗口矩形：整帧比，只作参考
            n, total, r = diff_region(ia, ib, (0, 0, ia.width, ia.height))
            tag = "参考(整帧)"
        else:
            # 期望矩形是**逻辑坐标**，按 scale 折算到帧像素
            sx, sy, sw, sh = box
            box = (int(round(sx * scale)), int(round(sy * scale)),
                   int(round(sw * scale)), int(round(sh * scale)))
            n, total, r = diff_region(ia, ib, box)
            tag = "OK" if n == 0 else "DIFF"
        # 单边是否真的出了窗（与自己那侧基线在该区域比）
        o_chg = _changed_vs_base(base_orig, orig, r, scale)
        m_chg = _changed_vs_base(base_ours, ours, r, 1.0)
        # 2026-09-28：`我方出窗` 这一列是**像素判据**（这一帧与基线帧不同）。对宠物/行会/坐骑/钓鱼
        # 这四扇窗，"按 C# Show() 守卫应当**不开窗**、只弹提示框"才是正确行为 —— 把
        # `golden_ab_ours.ps1` 记的机器可读事实（window_open / notice）一并打出来，避免读者把
        # "画面变了" 误读成 "窗开了"。
        manifest_note = ""
        if p.get("window_open") is not None:
            manifest_note = f" 我方窗开={p.get('window_open')}"
        if p.get("notice"):
            manifest_note += f" 提示={p['notice']}"
        if o_chg is False or m_chg is False:
            tag = "不可比(单边未出窗)"
        elif o_chg is None or m_chg is None:
            tag = "不可比(缺基线)"
        pct = 100.0 * n / max(total, 1)
        print(f"{p.get('action',''):16s} {kind or '-':22s} {str(r):>22s} {n:>10d} {pct:>6.1f}%  {tag}"
              f"   [原版出窗={o_chg} 我方出窗={m_chg}{manifest_note}]")
        results.append(dict(p, region=r, changed=n, total=total, pct=round(pct, 3), tag=tag,
                            orig_rendered=o_chg, ours_rendered=m_chg))
    if a.out:
        json.dump(results, open(a.out, "w", encoding="utf-8"), ensure_ascii=False, indent=2)
        print("wrote", a.out)
    return 0


if __name__ == "__main__":
    sys.exit(main())
