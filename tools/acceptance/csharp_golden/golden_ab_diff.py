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
    import shutil

    shutil.rmtree(tmp, ignore_errors=True)
    print("SelfTest PASS" if not bad else "SelfTest FAIL")
    return 1 if bad else 0


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
    if a.out:
        json.dump(results, open(a.out, "w", encoding="utf-8"), ensure_ascii=False, indent=2)
        print("wrote", a.out)
    return 0


if __name__ == "__main__":
    sys.exit(main())
