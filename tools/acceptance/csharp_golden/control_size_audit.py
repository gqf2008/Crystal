# control_size_audit.py — 「窗内控件」尺寸审计：我方 `spawn_*` 里写死的 (w,h) 是否等于该帧的**美术原生尺寸**
#
# 为什么单做一条：2026-09-26 逐窗复核角色窗时，技能页翻页钮写死 40x22，而原版那两颗钮
# 不设 `Size` ⇒ 取 `Prguse[396]/[398]` 的原生 **16x14**（箭头被拉大 2.5 倍）。这是**一整类**缺陷
# ——"抄了别的按钮的尺寸"，逐个窗口肉眼找太慢，用机械扫描一次扫全仓。
#
# 判据（启发式，输出必须人工过一遍）：
#   1. 找 `load_lib_image(..., LibraryName::<Lib>, <idx>)` / `ui_image(..., LibraryName::<Lib>, <idx>)`
#      的**调用点**（拿 lib/idx）；
#   2. 找其后 25 行内的 `spawn_icon_button(p,n,h,pr,x,y,w,h,z)` / `spawn_image(p,h,x,y,w,h,z)`；
#      两者的 w/h 是**数字字面量**时，与第 1 步那帧的美术尺寸比；
#   3. 不相等 → 报一行（file:line, lib[idx], 美术尺寸, 写死尺寸）。
#
# 已知**故意**不等的情况（原版就是按内容裁剪/拉伸，不要当缺陷改）：负重条/进度条/经验条这类
# 需要按比例裁宽的精灵（C# 用 `Draw(Index, section, …)` 自绘）。工具照报，人工筛。
#
# 用法：py -3.12 control_size_audit.py --repo <Rust 仓库根> --data <含 *.Lib 的 Data>
import argparse
import os
import re
import struct
import sys

LIB_FILE = {
    "Title": "Title.Lib", "Prguse": "Prguse.Lib", "Prguse2": "Prguse2.Lib",
    "Prguse3": "Prguse3.Lib", "Items": "Items.Lib", "MagIcon": "MagIcon.Lib",
    "MagIcon2": "MagIcon2.Lib", "Help": "Help.Lib", "MMap": "mmap.Lib",
    "MapLinkIcon": "MapLinkIcon.Lib", "BuffIcon": "BuffIcon.Lib", "ChrSel": "ChrSel.Lib",
    "Background": "Background.Lib", "Deco": "Deco.Lib", "Dragon": "Dragon.Lib",
    "Effect": "Effect.Lib", "Effect2": "Effect2.Lib", "GuildSkill": "GuildSkill.Lib",
    "Stateitem": "Stateitem.Lib", "dnitems": "dnitems.Lib", "Weather": "Weather.lib",
}

_cache = {}


def art_size(data_dir, lib, index):
    key = (lib, index)
    if key in _cache:
        return _cache[key]
    path = os.path.join(data_dir, LIB_FILE.get(lib, lib + ".Lib"))
    out = None
    if os.path.exists(path):
        with open(path, "rb") as f:
            data = f.read()
        version, count = struct.unpack_from("<ii", data, 0)
        if version >= 2 and 0 <= index < count:
            base = 8 + (4 if version >= 3 else 0)
            (offset,) = struct.unpack_from("<i", data, base + index * 4)
            w, h = struct.unpack_from("<hh", data, offset)
            out = (w, h)
    _cache[key] = out
    return out


LOAD = re.compile(
    r"(?:let\s+(?:Some\()?\s*(\w+)\s*\)?\s*=\s*)?"
    r"(?:load_lib_image|ui_image)\(\s*&mut libs,\s*&mut images,(?:[^,]*?,)?\s*LibraryName::(\w+),\s*(\d+)\s*\)"
)
SPAWN = re.compile(r"spawn_(icon_button|image)\(([^;]*?)\)", re.S)
NUM = re.compile(r"^\s*(\d+(?:\.\d+)?)\s*(?:f32|f64)?\s*$")


def scan_file(path, data_dir, rows):
    text = open(path, encoding="utf-8", errors="replace").read()
    lines = text.split("\n")
    loads = [(text[:m.start()].count("\n") + 1, m.group(1) or "", m.group(2), int(m.group(3)))
             for m in LOAD.finditer(text)]
    if not loads:
        return
    for m in SPAWN.finditer(text):
        line = text[:m.start()].count("\n") + 1
        # 取该 spawn 之前最近的一次 load
        # 配对必须**靠句柄变量名**：`if let Some(h) = load_lib_image(…Prguse, 586)` 后面
        # `spawn_image(p, h, …)` 才是同一张图。只按"最近一次 load"配会把面板背景配到小按钮上，
        # 产出成片假阳性（实测 48 条里大半是这种）。
        args_probe = [a.strip() for a in m.group(2).split(",")]
        handle_pos = (1, 2) if m.group(1) == "icon_button" else (1,)
        handles = {args_probe[i] for i in handle_pos if i < len(args_probe)}
        prev = [l for l in loads if l[0] <= line and l[1] and l[1] in handles]
        if not prev:
            continue
        lline, _var, lib, idx = prev[-1]
        # 再收紧两档，压掉"变量名撞车"的假阳性：
        #   ① 距离 ≤ 8 行（`if let Some(h) = load…` 紧跟着 `spawn_*(p, h, …)` 的写法）；
        #   ② 这两行之间**不许再有别的 load**（否则说明这次的 h 来自更近的那次）。
        if line - lline > 8:
            continue
        if any(lline < l[0] <= line for l in loads):
            continue
        args = args_probe
        # spawn_icon_button(p,n,h,pr,x,y,w,h,z) → w,h = 6,7；spawn_image(p,h,x,y,w,h,z) → 4,5
        pos = (6, 7) if m.group(1) == "icon_button" else (4, 5)
        if len(args) <= pos[1]:
            continue
        wm, hm = NUM.match(args[pos[0]]), NUM.match(args[pos[1]])
        if not (wm and hm):
            continue
        w, h = float(wm.group(1)), float(hm.group(1))
        art = art_size(data_dir, lib, idx)
        if not art:
            continue
        if (w, h) != (float(art[0]), float(art[1])):
            rows.append({
                "file": os.path.relpath(path),
                "line": line,
                "load_line": lline,
                "lib": lib,
                "index": idx,
                "art": art,
                "explicit": (w, h),
            })


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--repo", required=True, help="Rust 仓库根（含 Client-Bevy/src）")
    ap.add_argument("--data", required=True, help="含 *.Lib 的数据目录")
    ap.add_argument("--subdir", default=os.path.join("Client-Bevy", "src"))
    a = ap.parse_args()

    rows = []
    root = os.path.join(a.repo, a.subdir)
    for dirpath, _dirs, files in os.walk(root):
        for fn in sorted(files):
            if fn.endswith(".rs"):
                scan_file(os.path.join(dirpath, fn), a.data, rows)
    rows.sort(key=lambda r: (r["file"], r["line"]))
    for r in rows:
        print("%-52s:%-5d %s[%d] 美术=%sx%s 写死=%gx%g" % (
            r["file"], r["line"], r["lib"], r["index"], r["art"][0], r["art"][1],
            r["explicit"][0], r["explicit"][1]))
    print(f"合计 {len(rows)} 处「写死尺寸 ≠ 美术原生尺寸」（含按比例裁宽的进度条等已知故意项，需人工筛）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
