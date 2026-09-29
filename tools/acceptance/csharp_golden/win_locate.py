# win_locate.py — 「某个美术帧到底画在实机截图的哪里」：**全屏**模板匹配定位器
#
# 为什么单做一条：`art_match.py` 只在**给定矩形**里比——矩形给错就直接判「没画」，
# 而这一整类缺口（窗口画在别的坐标上）恰恰是"矩形给错"造成的：
#   2026-09-29 实机对表期间，三处缺口全靠**全屏**匹配才看出来——
#   ① `npc_awake` 面板实际在 **(0,224)**（C# `GameScene.cs:307` 逐实例覆盖，本端画在 (0,0)）；
#   ② 觉醒 NPC 的「分解」开的是投放面板 `Prguse2[351]`@**(264,224)**（本端画的是觉醒面板）；
#   ③ NPC 窗的 Quest 按钮开的是 `QuestListDialog` `Prguse[950]`@**(487,0)**（本端画的是任务日记）。
#   当时是现写 `cv2.matchTemplate` 一行行跑，本轮把它固化成工具 + 自检。
#
# 用法：
#   py -3.12 win_locate.py --shot <帧.png> --lib <Data/Prguse.Lib> --index 950
#   py -3.12 win_locate.py --shot <帧.png> --lib <...> --index 950 --expect 487,0        # 顺带判位置
#   py -3.12 win_locate.py --lib <Data/Prguse.Lib> --selftest                          # 正/负对照
#
# 判据（口径与 `art_match.py` 一致：**整帧 RGB 差之和 > tol 计 1 个不符像素**）：
#   ratio = 不符像素 / 该美术**不透明**像素数（alpha < --alpha 的像素不参与）
#   `--expect x,y` 时另判「最佳落点是否落在 x±tol-px, y±tol-px 内」；退出码 0/1。
#
# 与 `art_match.py` 的分工：
#   * 知道窗口该在哪 → `art_match.py --rect ...`（快，能逐候选比）；
#   * **不知道**窗口画在哪、或怀疑它画错位置 → 本工具（全屏搜，代价是与候选无关的 O(W·H)）。
import argparse
import gzip
import json
import os
import struct
import sys
import tempfile

LIB_FILE = {
    "Title": "Title.Lib",
    "Prguse": "Prguse.Lib",
    "Prguse2": "Prguse2.Lib",
    "Prguse3": "Prguse3.Lib",
}


def load_lib(path):
    """读 `.Lib`：返回 (entries, data)；entries[i] = (offset, w, h)（与 libextract.py 同格式）。"""
    with open(path, "rb") as f:
        data = f.read()
    version, count = struct.unpack_from("<ii", data, 0)
    if version < 2:
        raise SystemExit(f"unsupported lib version {version}: {path}")
    base = 8 + (4 if version >= 3 else 0)
    offsets = struct.unpack_from(f"<{count}i", data, base)
    entries = []
    for off in offsets:
        w, h = struct.unpack_from("<hh", data, off)
        entries.append((off, w, h))
    return entries, data


def decode_entry(data, offset, want_size=None):
    """解一帧 → (bgr uint8 HxWx3, alpha uint8 HxW)；`want_size` 给了就按 (w,h) 裁剪/跳过。"""
    import numpy as np

    w, h, x, y, sx, sy, shadow, length = struct.unpack_from("<hhhhhhBi", data, offset)
    if w <= 0 or h <= 0 or length <= 0:
        return None, None
    raw = gzip.decompress(data[offset + 17 : offset + 17 + length])
    need = w * h * 4
    if len(raw) < need:
        return None, None
    arr = np.frombuffer(raw[:need], dtype=np.uint8).reshape(h, w, 4)
    bgr = arr[:, :, :3].copy()
    alpha = arr[:, :, 3].copy()
    if want_size is not None:
        ww, hh = want_size
        if ww > w or hh > h:
            return None, None
        bgr = bgr[:hh, :ww]
        alpha = alpha[:hh, :ww]
    return bgr, alpha


def locate(shot_path, art, alpha, tol=60, alpha_floor=32):
    """全屏找 art 的最佳落点 → (ratio, x, y, w, h, bad, total)。"""
    import cv2
    import numpy as np

    shot = cv2.imread(shot_path, cv2.IMREAD_COLOR)
    if shot is None:
        raise SystemExit(f"读不到截图：{shot_path}")
    h, w = art.shape[:2]
    if shot.shape[0] < h or shot.shape[1] < w:
        raise SystemExit(f"美术 {w}x{h} 比截图 {shot.shape[1]}x{shot.shape[0]} 还大")
    res = cv2.matchTemplate(shot, art, cv2.TM_SQDIFF)
    _, _, minloc, _ = cv2.minMaxLoc(res)
    x, y = int(minloc[0]), int(minloc[1])
    reg = shot[y : y + h, x : x + w]
    d = np.abs(reg.astype(np.int16) - art.astype(np.int16)).sum(axis=2)
    mask = alpha > alpha_floor
    total = int(mask.sum())
    bad = int(((d > tol) & mask).sum())
    ratio = (bad / total) if total else 1.0
    return ratio, x, y, w, h, bad, total


def cmd_locate(a):
    entries, data = load_lib(a.lib)
    if a.index < 0 or a.index >= len(entries):
        raise SystemExit(f"[{a.index}] 越界（该库 {len(entries)} 帧）")
    off, w, h = entries[a.index]
    art, alpha = decode_entry(data, off)
    if art is None:
        raise SystemExit(f"[{a.index}] 是空帧（w={w} h={h}）——空帧没有可比像素，别用它定位")
    ratio, x, y, w, h, bad, total = locate(a.shot, art, alpha, tol=a.tol, alpha_floor=a.alpha)
    out = {
        "shot": a.shot,
        "lib": a.lib,
        "index": a.index,
        "size": [w, h],
        "best": [x, y],
        "ratio": round(ratio, 4),
        "bad": bad,
        "total": total,
    }
    print(
        "%s  [%d] %dx%d → 最佳落点 (%d,%d)  不符率 %.4f（%d/%d 不透明像素）"
        % (os.path.basename(a.shot), a.index, w, h, x, y, ratio, bad, total)
    )
    ok = ratio <= a.threshold
    if not ok:
        print(f"  判定：FAIL —— 不符率 {ratio:.4f} > 阈值 {a.threshold}（这一帧大概率没画在屏幕上）")
    if a.expect:
        ex, ey = [int(v) for v in a.expect.split(",")]
        hit = abs(x - ex) <= a.tol_px and abs(y - ey) <= a.tol_px
        out["expect"] = [ex, ey]
        out["expect_hit"] = hit
        print(
            "  期望 (%d,%d) ±%dpx → %s（实测 %d,%d）"
            % (ex, ey, a.tol_px, "命中" if hit else "**偏离**", x, y)
        )
        ok = ok and hit
    if a.json:
        with open(a.json, "w", encoding="utf-8") as f:
            json.dump(out, f, ensure_ascii=False, indent=2)
        print("  写出", a.json)
    return 0 if ok else 1


def cmd_selftest(a):
    """正/负对照：
    正——把已知美术贴到合成帧的**已知坐标**上，工具必须报出那个坐标（差 0px）；
    负——同一张美术去搜**没贴它的空帧**，必须报出明显更高的不符率（证明它真在"找"）。"""
    import cv2
    import numpy as np

    entries, data = load_lib(a.lib)
    # 找一张**非空、够大又放得下、且够"亮"**的帧当目标：
    # 负对照要求"同一张图去搜空帧必须明显不符"，而一张本身就是深色的美术贴在深色底上
    # 比对照样会"看起来还行"（实测 Prguse[50] 只有 0.21）——故显式挑亮点占比 >=25% 的帧。
    pick = None
    for idx in range(len(entries)):
        art, alpha = decode_entry(data, entries[idx][0])
        if art is None:
            continue
        h, w = art.shape[:2]
        if not (120 <= w <= 420 and 120 <= h <= 320):
            continue
        opaque = alpha > 32
        total = int(opaque.sum())
        if total < 5000:
            continue
        bright = int(((art.max(axis=2) > 120) & opaque).sum())
        if bright / total >= 0.25:
            pick = (idx, art, alpha)
            break
    if pick is None:
        print("SELFTEST-SKIP：该库里找不到 120..420 x 120..320 的非空帧（换 --lib 再试）")
        return 2
    idx, art, alpha = pick
    px, py = 137, 221  # 故意取非零、非对齐的坐标
    h, w = art.shape[:2]
    d = tempfile.mkdtemp(prefix="winloc_")
    target = os.path.join(d, "target.png")
    blank = os.path.join(d, "blank.png")
    canvas = np.zeros((py + h + 40, px + w + 40, 3), dtype=np.uint8)
    canvas[:, :] = (24, 24, 24)
    canvas[py : py + h, px : px + w] = art
    cv2.imwrite(target, canvas)
    blank_canvas = np.zeros((py + h + 40, px + w + 40, 3), dtype=np.uint8)
    blank_canvas[:, :] = (24, 24, 24)
    cv2.imwrite(blank, blank_canvas)

    r1, x1, y1, _, _, _, _ = locate(target, art, alpha, tol=a.tol, alpha_floor=a.alpha)
    r2, x2, y2, _, _, _, _ = locate(blank, art, alpha, tol=a.tol, alpha_floor=a.alpha)
    pos_ok = (x1, y1) == (px, py) and r1 < 0.02
    # 负对照用**相对**判据：空帧上的最佳不符率必须明显差于贴了它那帧。
    # （固定阈值不可靠：贴在深色底上的深色美术"看起来也还行"，实测 0.47 —— 别按绝对值判。）
    neg_ok = r2 > 0.25 and r2 > r1 + 0.2
    print(
        "正对照：帧 [%d] 贴到 (%d,%d) → 报 (%d,%d) 不符率 %.4f %s"
        % (idx, px, py, x1, y1, r1, "OK" if pos_ok else "**FAIL**")
    )
    print(
        "负对照：同一帧去搜空帧 → 报 (%d,%d) 不符率 %.4f（须 > %.4f+0.2）%s"
        % (x2, y2, r2, r1, "OK" if neg_ok else "**FAIL**")
    )
    return 0 if (pos_ok and neg_ok) else 1


def main():
    ap = argparse.ArgumentParser(description="全屏模板匹配：某个 .Lib 帧画在截图里的哪里")
    ap.add_argument("--shot", help="整屏 PNG")
    ap.add_argument("--lib", required=True, help="美术库（如 Data/Prguse.Lib）")
    ap.add_argument("--index", type=int, help="帧号")
    ap.add_argument("--expect", default="", help='期望落点 "x,y"（给了就顺带判位置）')
    ap.add_argument("--tol-px", type=int, default=2, help="--expect 的位置容差（默认 2px）")
    ap.add_argument("--tol", type=int, default=60, help="单像素 RGB 差之和容差（默认 60，同 art_match）")
    ap.add_argument("--alpha", type=int, default=32, help="美术低于该 alpha 的像素不参与比对（默认 32）")
    ap.add_argument("--threshold", type=float, default=0.15, help="命中阈值（默认 0.15）")
    ap.add_argument("--json", default="", help="把结论写成 JSON")
    ap.add_argument("--selftest", action="store_true", help="跑正/负对照（不需要 --shot）")
    a = ap.parse_args()
    if a.selftest:
        return cmd_selftest(a)
    if not a.shot or a.index is None:
        ap.error("非 --selftest 模式必须给 --shot 与 --index")
    return cmd_locate(a)


if __name__ == "__main__":
    sys.exit(main())
