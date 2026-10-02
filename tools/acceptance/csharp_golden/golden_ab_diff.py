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

# 归因档位（`--localize`）：**是否差异**用与主表同一条判据（`sum(RGB) > 12`），
# **幅度**另按「最大通道差」分档——用来区分「字形/AA 级差异」（小档）与「一侧根本没有这块内容」
# （大档：实测面板底 vs 地图 Δmax≈48、纯黑 vs 面板底 Δmax≈240）。第一档是 `≤12`，
# 因为两个判据不完全等价（如 (5,5,5)：sum=15 算差异，但 max=5 落在此档）——列出来让各档之和 = 差异像素数。
DELTA_BUCKETS = ((0, 13, "<=12"), (13, 25, "13-24"), (25, 65, "25-64"),
                 (65, 128, "65-127"), (128, 256, "128-255"))

# 窗口 kind 的**别名**：同一块 C# 面板在不同键位下被我们的 A/B 清单记成不同 kind。
#  - `character_skill_page`（F11 技能页）= `CharacterDialog` 的**同一扇窗**（技能页是它的子页），
#    矩形与 `character` 逐值相同 ⇒ 直接复用几何表里的 `character` 行。
KIND_ALIAS = {
    "character_skill_page": "character",
}


def rects_from_table(path):
    rows = json.load(open(path, encoding="utf-8"))
    out = {}
    for r in rows:
        kind = r.get("kind")
        if not kind or r.get("x") is None or not r.get("expect"):
            continue
        out.setdefault(kind, (r["x"], r["y"], r["expect"][0], r["expect"][1]))
    for alias, target in KIND_ALIAS.items():
        if alias not in out and target in out:
            out[alias] = out[target]
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


def diff_region_shifted(a, b, box, max_shift=1):
    """同一块区域，允许把**我方帧**整块平移 ±max_shift 像素后再比，返回最小差异。

    为什么需要：「居中窗」在 A/B 里会恒定偏 **+1px 横向**——C# `Center` 用
    `(ScreenWidth - Width) / 2`，而原版侧取帧的客户区比 1024 宽 2px（⇒ 算出 381），
    本端按 1024 算得 380。实测（2026-09-28，Friends）：我方 `dialog_rect rx=380 ry=248`
    与 C# 公式逐值相同，但原版帧面板左边框在 x=381 —— 那是**取帧口径**，不是本端排版 bug。
    显式坐标窗（Inventory/Equipment/Options/Group/Quests）位移扫描都是 dx=0。

    返回 `(n, dx, dy)`：`n` = 平移后最小差异像素数，`(dx, dy)` = 取到最小的平移量。
    """
    x0, y0, w, h = box
    x1, y1 = x0 + w, y0 + h
    best = None
    for dy in range(-max_shift, max_shift + 1):
        for dx in range(-max_shift, max_shift + 1):
            # ⚠️ 只能平移**一侧**：`diff_region(a, b, shifted_box)` 会把两边一起平移 ⇒
            # 差异符号不变、等于没平移（第一版就是这么写的，扫描结果恒等于 raw）。
            ca = a.crop((x0 + dx, y0 + dy, x1 + dx, y1 + dy))
            cb = b.crop((x0, y0, x1, y1))
            d = ImageChops.difference(ca, cb)
            px = d.load()
            n = 0
            for y in range(d.height):
                for x in range(d.width):
                    if sum(px[x, y]) > 12:
                        n += 1
            if best is None or n < best[0]:
                best = (n, dx, dy)
    return best if best is not None else (0, 0, 0)


def selftest() -> int:
    """判据自检：① 纯 1px 平移必须被认成「口径」；② 真差异（整块 6px 平移）不许被认成口径。"""
    import tempfile

    from PIL import ImageDraw

    tmp = tempfile.mkdtemp(prefix="golden_ab_diff_selftest_")
    bad = 0

    def mk(path, dx, dy):
        im = Image.new("RGB", (120, 120), (0, 0, 0))
        d = ImageDraw.Draw(im)
        d.rectangle((10 + dx, 10 + dy, 60 + dx, 60 + dy), fill=(200, 180, 120))
        d.rectangle((20 + dx, 20 + dy, 30 + dx, 30 + dy), fill=(255, 0, 0))
        im.save(path)

    a = os.path.join(tmp, "a.png")
    b1 = os.path.join(tmp, "b1.png")
    b6 = os.path.join(tmp, "b6.png")
    mk(a, 0, 0)
    mk(b1, 1, 0)
    mk(b6, 6, 0)
    ia, i1, i6 = (Image.open(p).convert("RGB") for p in (a, b1, b6))
    box = (10, 10, 50, 50)
    n_raw, _t, _r = diff_region(ia, i1, box)
    n_shift, dx, _dy = diff_region_shifted(ia, i1, box, 1)
    # 合成图把 `b` 相对 `a` 右移了 1px ⇒ 对齐要平移**一侧**；dx 的符号取决于取哪一侧平移，
    # 判据只要求「量到 1px 平移且平移后差异归零」。
    ok1 = n_raw > 0 and n_shift == 0 and abs(dx) == 1
    print(f"  [{'PASS' if ok1 else 'FAIL'}] 纯 1px 平移：raw={n_raw} shifted={n_shift}(dx={dx})")
    bad += 0 if ok1 else 1
    n6_raw, _t6, _r6 = diff_region(ia, i6, box)
    n6_shift, _dx6, _dy6 = diff_region_shifted(ia, i6, box, 1)
    ok6 = n6_shift > 0 and n6_shift >= n6_raw * 0.5
    print(f"  [{'PASS' if ok6 else 'FAIL'}] 真差异（6px 平移）：raw={n6_raw} shifted={n6_shift}（不许被 ±1 口径吃掉）")
    bad += 0 if ok6 else 1

    # ③ `--localize`：一侧多出的实心块必须被 top 块抓到、且最优平移为 (0,0)
    blank = Image.new("RGB", (64, 48), (0, 0, 0))
    withblk = blank.copy()
    ImageDraw.Draw(withblk).rectangle((16, 16, 31, 31), fill=(255, 255, 255))
    pa2, pb2 = os.path.join(tmp, "loc_a.png"), os.path.join(tmp, "loc_b.png")
    blank.save(pa2)
    withblk.save(pb2)
    n2, _t2, lines2 = localize_report(
        Image.open(pa2).convert("RGB"), Image.open(pb2).convert("RGB"), (0, 0, 64, 48), 1.0, 1
    )
    top2 = next((l for l in lines2 if "block" in l), "")
    ok3 = n2 == 256 and "+( 16, 16)" in top2
    print(f"  [{'PASS' if ok3 else 'FAIL'}] --localize：实心 16x16 块被 top 抓到（{top2.strip()}）")
    bad += 0 if ok3 else 1
    import shutil

    shutil.rmtree(tmp, ignore_errors=True)
    print("SelfTest PASS" if not bad else "SelfTest FAIL")
    return 1 if bad else 0


def _diff_mask(a, b, box, dx=0, dy=0):
    """把 `a`（我方/本端）整体平移 (dx,dy) 后与 `b`（原版）在 box 内做差异掩膜。

    返回 `(diffsum, magmax, w, h)`：
      * `diffsum`：三通道差之**和**（`ImageChops.add` 链，饱和在 255）——与主表 `sum(RGB) > 12` **同口径**；
      * `magmax`：三通道差的**最大值**——只用来给差异像素标幅度。
    """
    x0, y0, w, h = box
    ca = a.crop((x0 + dx, y0 + dy, x0 + w + dx, y0 + h + dy))
    cb = b.crop((x0, y0, x0 + w, y0 + h))
    d = ImageChops.difference(ca.convert("RGB"), cb.convert("RGB"))
    r, g, bl = d.split()
    diffsum = ImageChops.add(ImageChops.add(r, g), bl)
    magmax = ImageChops.lighter(ImageChops.lighter(r, g), bl)
    return diffsum, magmax, w, h


def localize_report(a, b, box, scale=1.0, max_shift=1, block=16, band=8, min_band=15.0,
                    top=8, label=""):
    """`--localize` 的单窗归因：最优平移 + 幅度直方图 + 16px 网格 top 块 + 逐行带。

    为什么要它：整窗一个「差异占比」只能判"这两扇窗长得不一样"，判不了**差在哪、是什么性质**。
    实测三种典型形态各自有签名（见 README §3.2ef）：
      * 实心方块（一侧多一个控件）  ⇒ 某个 16px 块 100%、且连续多行同宽；
      * 字形/AA 差异（同文本不同字体）⇒ 逐行带铺满整窗、top 块是**窄竖条**（汉字笔画）、
        幅度直方图两头都重（笔画边缘 Δ 大、边缘外 Δ 小）；
      * 整体 ±1px 取帧口径          ⇒ 平移后差异掉到 <0.5%。
    文本输出（不落图），供把结论抄进 README。
    """
    sx, sy, sw, sh = box
    box = (int(round(sx * scale)), int(round(sy * scale)),
           int(round(sw * scale)), int(round(sh * scale)))
    x0, y0, w, h = box
    best = None
    for dy in range(-max_shift, max_shift + 1):
        for dx in range(-max_shift, max_shift + 1):
            diffsum, _, _, _ = _diff_mask(a, b, box, dx, dy)
            n = sum(1 for p in diffsum.tobytes() if p > 12)
            if best is None or n < best[0]:
                best = (n, dx, dy)
    n_shift, sdx, sdy = best
    diffsum, magmax, w, h = _diff_mask(a, b, box, sdx, sdy)
    px = diffsum.load()
    mp = magmax.load()
    total = w * h
    flat = sum(1 for p in diffsum.tobytes() if p > 12)
    lines = [f"== {label or ''} rect=({x0},{y0},{w},{h}) 差异={flat}/{total}={100.0 * flat / max(total, 1):.1f}%"
             f"  最优平移 dx={sdx} dy={sdy} ⇒ 平移后={n_shift}({100.0 * n_shift / max(total, 1):.1f}%)"]
    # 幅度直方图
    bucket = {name: 0 for _, _, name in DELTA_BUCKETS}
    for y in range(h):
        for x in range(w):
            if px[x, y] <= 12:
                continue
            p = mp[x, y]
            for lo, hi, name in DELTA_BUCKETS:
                if lo <= p < hi:
                    bucket[name] += 1
                    break
    lines.append("   幅度档(max通道差，仅统计差异像素)： " + "  ".join(
        f"{name}={bucket[name]}({100.0 * bucket[name] / max(flat, 1):.0f}%)"
        for _, _, name in DELTA_BUCKETS))
    # 16px 网格块
    blocks = []
    for by in range(0, h, block):
        for bx in range(0, w, block):
            c = t = 0
            for yy in range(by, min(by + block, h)):
                for xx in range(bx, min(bx + block, w)):
                    t += 1
                    if px[xx, yy] > 12:
                        c += 1
            blocks.append((c / max(t, 1), bx, by, min(block, w - bx), min(block, h - by)))
    blocks.sort(key=lambda r: -r[0])
    for ratio, bx, by, bw, bh in blocks[:top]:
        if ratio <= 0:
            break
        lines.append(f"   block +({bx:>3},{by:>3}) {bw}x{bh} {100.0 * ratio:5.1f}%  abs=({x0 + bx},{y0 + by})")
        # 几乎整块都差（≥90%）时再钻一层：它在那一带里是**实心矩形**（一侧多/少一个控件）
        # 还是**字迹笔画**？判据 = 「该 16px 带内整列都差」的**连续列段宽度**——
        # 实心块给一条 ≥ 块宽的连续段；汉字/数字笔画给多段 2~20px 的窄条（GameShop 实测）。
        if ratio >= 0.9:
            col_full = []
            for xx in range(bx, bx + bw):
                col_full.append(all(px[xx, y2] > 12 for y2 in range(by, by + bh)))
            runs, s = [], None
            for i, full in enumerate(col_full + [False]):
                if full and s is None:
                    s = i
                elif not full and s is not None:
                    runs.append((bx + s, bx + i - 1))
                    s = None
            widths = [e - b + 1 for b, e in runs]
            lines.append(
                f"      整列都差的列段（{len(runs)} 段，宽 {min(widths) if widths else 0}"
                f"~{max(widths) if widths else 0}px）： " + ", ".join(f"{b}-{e}" for b, e in runs[:8]))
    # 逐行带（8px 聚成一带）
    for yy in range(0, h, band):
        c = t = 0
        xs = []
        for y2 in range(yy, min(yy + band, h)):
            for xx in range(w):
                t += 1
                if px[xx, y2] > 12:
                    c += 1
                    xs.append(xx)
        if c and 100.0 * c / max(t, 1) > min_band:
            lines.append(f"   band y={yy:>3} abs={y0 + yy:>3} {100.0 * c / max(t, 1):5.1f}%  x={min(xs)}..{max(xs)}")
    return n_shift, total, lines


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
    # `--selftest` 不需要帧，所以这里不设 required，由下面的分支各自校验
    ap.add_argument("--shots", default="")
    ap.add_argument("--table", default="")
    ap.add_argument("--out", default="")
    # 「居中窗 ±1px」口径（见 diff_region_shifted 的说明）：默认开；`--no-shift` 关掉
    ap.add_argument("--max-shift", type=int, default=1)
    ap.add_argument("--no-shift", action="store_true")
    # 逐窗归因（可选）：`--localize relationship,game_shop` 或 `--localize all`。
    # 只在主表之后**多打一段文本**，不改主表口径/退出码。
    ap.add_argument("--localize", default="")
    ap.add_argument("--localize-top", type=int, default=8)
    ap.add_argument("--localize-min-band", type=float, default=15.0)
    ap.add_argument("--selftest", action="store_true")
    a = ap.parse_args()
    if a.selftest:
        return selftest()
    if not a.shots or not a.table:
        print("FAIL(前置)：--shots 与 --table 必填（或 --selftest 只跑判据自检）", file=sys.stderr)
        return 2

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
    kept = {}
    print(f"{'action':16s} {'kind':22s} {'区域(x,y,w,h)':>22s} {'差异像素':>10s} {'占比':>7s}  判定")
    for p in pairs:
        kind = p.get("kind")
        orig, ours = p.get("orig"), p.get("ours")
        shift_note = ""
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
            shift_note = ""
            # 平移口径：把差异里「整体 ±1px 平移就能消掉」的那部分单独列出来。
            # 判据（对齐 §3.2f 的实测）：平移后差异降到 0.5% 以下 ⇒ 这一行算**取帧口径**，
            # 不是窗内绘制缺陷；只降到"好一些"则仍按真实差异对待，但把读数打出来供判断。
            if not a.no_shift and n > 0:
                n_shift, sdx, sdy = diff_region_shifted(ia, ib, box, a.max_shift)
                if n_shift == 0 or n_shift / max(total, 1) < 0.005:
                    tag = f"口径(±{a.max_shift}px 平移, dx={sdx} dy={sdy})"
                elif n_shift < n * 0.6:
                    shift_note = f" 平移后={n_shift}({100.0 * n_shift / max(total, 1):.1f}%, dx={sdx} dy={sdy})"
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
              f"   [原版出窗={o_chg} 我方出窗={m_chg}{manifest_note}{shift_note}]")
        results.append(dict(p, region=r, changed=n, total=total, pct=round(pct, 3), tag=tag,
                            orig_rendered=o_chg, ours_rendered=m_chg,
                            shift_note=shift_note.strip()))
        kept[kind] = (ia, ib, box if box is not None else (0, 0, ia.width, ia.height), 1.0)
    if a.localize:
        want = [k.strip() for k in a.localize.split(",") if k.strip()]
        if want == ["all"]:
            want = [p.get("kind") for p in pairs if p.get("kind")]
        print()
        for kind in want:
            if kind not in kept:
                print(f"== {kind}: 无可用帧（未在该轮 A/B 清单里 / 缺帧 / 无矩形）")
                continue
            ia_, ib_, box_, scale_ = kept[kind]
            _n, _t, lines = localize_report(
                ia_, ib_, box_, scale_, a.max_shift, top=a.localize_top,
                min_band=a.localize_min_band, label=kind,
            )
            print("\n".join(lines))
    if a.out:
        json.dump(results, open(a.out, "w", encoding="utf-8"), ensure_ascii=False, indent=2)
        print("wrote", a.out)
    return 0


if __name__ == "__main__":
    sys.exit(main())
