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
# 2026-09-26 补**第二种扫描面**：常量表 + `for` 循环（`const TBL: &[(…, LibraryName::X, n, h, pr, y, w, h)]`
#   + `for (…, bw, bh) in TBL { load(…, *lib, *n) … spawn_icon_button(…, *bw, *bh, …) }`）。
#   这种写法的 lib/idx 是**变量**，只认字面量的配对逻辑整组看不见 ⇒ 实测 `menu.rs` 13 颗钮
#   统一写死 38x19（图头 Title[633/636]=32x20、其余 Prguse/Prguse2=32x18）被漏掉。
#   现在逐**表行**比：表行尺寸列 ≠ 该行 normal 帧图头 → 报一行（`--selftest` 里有对应正/负对照）。
#
# 已知**故意**不等的情况（原版就是按内容裁剪/拉伸，不要当缺陷改）：负重条/进度条/经验条这类
# 需要按比例裁宽的精灵（C# 用 `Draw(Index, section, …)` 自绘）。工具照报，人工筛。
#
# 用法：py -3.12 control_size_audit.py --repo <Rust 仓库根> --data <含 *.Lib 的 Data>
#
# **门禁语义**：发现 >0 处即 exit 1（可当常规门禁跑）；`--selftest` 跑正/负对照
# （正：临时把一处尺寸改坏，必须报出来；负：不改就应 0 命中），二者都通过才 exit 0。
import argparse
import os
import re
import shutil
import struct
import sys
import tempfile

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
# 表驱动循环形态：`const TBL: &[(…)] = &[ (…LibraryName::X, n, h, pr, y, w, h), … ];`
#   + `for (a, lib, n, h, pr, y, bw, bh) in TBL {`
#   + 循环体里 `load_lib_image(&mut libs, &mut images, *lib, *n)`（**变量** lib/idx）
#   + `spawn_icon_button(p, nh, hh, ph, x, *y, *bw, *bh, z)`（尺寸来自表行）
# 2026-09-26 发现：这种写法此前**整组扫不到**（LOAD 正则要求字面量 lib/idx），
# 实测 menu.rs 13 颗钮统一写死 38x19（图头 32x20 / 32x18）就是这样漏掉的。
TABLE_DECL = re.compile(r"const\s+(\w+)\s*:[^=]*?=\s*&\[(.*?)\n\];", re.S)
FOR_LOOP = re.compile(r"for\s*\(([^()]*)\)\s*in\s*(\w+)(?:\.iter\(\))?\s*\{")
DYN_LOAD = re.compile(
    r"(?:load_lib_image|ui_image)\(\s*&mut libs,\s*&mut images,\s*\*(\w+),\s*\*(\w+)\s*\)"
)
# 循环体里的三元组绑定：`if let (Some(nh), Some(hh), Some(ph)) = (load…, load…, load…)`
DYN_TUPLE = re.compile(r"if\s+let\s*\((.*?)\)\s*=\s*\(", re.S)


def _dyn_handles(body):
    r"""循环体里 `Some(handle) = (load(…*lib,*idx), …)` 的 句柄 → (lib 变量, idx 变量) 映射。

    注意不能写成 `Some\((\w+)\)\s*=\s*load` —— 元组写法里 `=` 后面还有一个 `(`，
    实测正则会一条都匹配不到（这正是本工具第二次"报了绿却看不见"的现场）。
    """
    out = {}
    for tm in DYN_TUPLE.finditer(body):
        names = re.findall(r"Some\(\s*(\w+)\s*\)", tm.group(1))
        if not names:
            continue
        rest = body[tm.end():tm.end() + 400]
        pairs = [(m.group(1), m.group(2)) for m in DYN_LOAD.finditer(rest)][:len(names)]
        for n, pair in zip(names, pairs):
            out[n] = pair
    return out


def _block_end(text, open_brace):
    """返回 `open_brace`（指向 `{`）所在块的闭合 `}` 的下标。"""
    depth = 0
    j = open_brace
    while j < len(text):
        if text[j] == "{":
            depth += 1
        elif text[j] == "}":
            depth -= 1
            if depth == 0:
                return j
        j += 1
    return len(text) - 1


def scan_table_loops(text, data_dir, rows, path):
    """扫「常量表 + for 循环」形态（见 TABLE_DECL 注释）：
    表行里的**尺寸列**（循环体 spawn 的第 7/8 个实参 = `*bw, *bh`）必须等于该行**normal 帧**的
    美术原生尺寸。逐行比 ⇒ 一张表里的三角尺寸不一致也能被抓到。"""
    decls = {m.group(1): m.group(2) for m in TABLE_DECL.finditer(text)}
    if not decls:
        return
    for lm in FOR_LOOP.finditer(text):
        table_name = lm.group(2)
        tbl = decls.get(table_name)
        if tbl is None:
            continue
        loop_vars = [v.strip() for v in lm.group(1).split(",")]
        if not loop_vars or loop_vars[0].startswith("_"):
            pass  # 仍要继续：`_` 只影响通配符语义，字段顺序照旧
        open_brace = text.index("{", lm.end() - 1)
        body = text[open_brace:_block_end(text, open_brace)]
        handles = _dyn_handles(body)
        if not handles:
            continue
        # 表行：`(a, LibraryName::X, n, h, pr, y, 32.0, 18.0),`
        table_rows = []
        for rm in re.finditer(r"\(([^()]*)\)", tbl):
            fields = [f.strip() for f in rm.group(1).split(",")]
            lib_pos = next((k for k, f in enumerate(fields) if f.startswith("LibraryName::")), None)
            if lib_pos is None:
                continue
            lib = fields[lib_pos].split("::", 1)[1]
            idx_pos = next((k for k in range(lib_pos + 1, len(fields)) if fields[k].isdigit()), None)
            if idx_pos is None:
                continue
            table_rows.append((fields, lib, int(fields[idx_pos])))
        if not table_rows:
            continue

        def resolve(arg, fields):
            """`*bw` → 该行对应列的字面量；`32.0` → 字面量本身；其余（表达式）→ None。"""
            a = arg.strip()
            if a.startswith("*"):
                name = a.lstrip("*").strip()
                if name in loop_vars:
                    k = loop_vars.index(name)
                    if k < len(fields):
                        m = NUM.match(fields[k])
                        return float(m.group(1)) if m else None
                return None
            m = NUM.match(a)
            return float(m.group(1)) if m else None

        for sm in SPAWN.finditer(body):
            args = [a.strip() for a in sm.group(2).split(",")]
            pos = (6, 7) if sm.group(1) == "icon_button" else (4, 5)
            if len(args) <= pos[1] or len(args) < 2:
                continue
            # 第 2 个实参是 normal 帧句柄：找到它对应的 (lib 列, idx 列)
            hname = args[1].lstrip("*").strip()
            field_vars = handles.get(hname)
            if field_vars is None:
                continue
            lib_var, idx_var = field_vars
            if lib_var not in loop_vars or idx_var not in loop_vars:
                continue
            lib_k, idx_k = loop_vars.index(lib_var), loop_vars.index(idx_var)
            line = text[:open_brace + sm.start()].count("\n") + 1
            for fields, _lib, _idx in table_rows:
                if len(fields) <= max(lib_k, idx_k) or not fields[idx_k].isdigit():
                    continue
                lib = fields[lib_k].split("::")[-1]
                idx = int(fields[idx_k])
                art = art_size(data_dir, lib, idx)
                if not art:
                    continue
                wv, hv = resolve(args[pos[0]], fields), resolve(args[pos[1]], fields)
                if wv is None or hv is None:
                    continue
                if (wv, hv) != (float(art[0]), float(art[1])):
                    try:
                        rel = os.path.relpath(path)
                    except ValueError:
                        rel = path
                    rows.append({
                        "file": rel,
                        "line": line,
                        "load_line": line,
                        "lib": lib,
                        "index": idx,
                        "art": art,
                        "explicit": (wv, hv),
                        "via": f"表 {table_name}",
                    })


def scan_file(path, data_dir, rows):
    text = open(path, encoding="utf-8", errors="replace").read()
    lines = text.split("\n")
    # 表驱动循环形态先扫（它用的是变量 lib/idx，下面的字面量配对逻辑看不见）
    scan_table_loops(text, data_dir, rows, path)
    loads = [(text[:m.start()].count("\n") + 1, m.group(1) or "", m.group(2), int(m.group(3)),
              m.start(), None)
             for m in LOAD.finditer(text)]
    # **元组形式**的 load：`if let (Some(n), Some(h), Some(pr)) = (load…, load…, load…) {`
    # —— 这种写法**没有 `let x =` 绑定**，上面正则抓到的 var 是空串，按"句柄变量名配对"就被整组跳过
    # ⇒ 假阴性（实测 `big_map.rs` 的滚屏箭头 12x12 写成 16x14 就是这样漏掉的）。
    # 这里把元组模式里的 `Some(<var>)` 按**顺序**补给该元组内的 load。
    # 模式部分是 `(Some(n), Some(h), Some(pr))` —— **里面还有括号**，所以用惰性匹配到 `)\s*=\s*(`，
    # 而不是 `[^)]*`（后者在第一个 `)` 就停了，group(1) 只会是 "Some(n"）。
    TUPLE = re.compile(r"if\s+let\s*\((.*?)\)\s*=\s*\(", re.S)
    for tm in TUPLE.finditer(text):
        vars_in_order = re.findall(r"Some\(\s*(\w+)\s*\)", tm.group(1))
        if not vars_in_order:
            continue
        # 找出 `=(` 之后那个配平括号的区间
        i = text.index("(", tm.start(0) + len("if let ("))
        i = text.index("=", tm.start(0)) + 1
        depth = 0
        j = i
        while j < len(text):
            if text[j] == "(":
                depth += 1
            elif text[j] == ")":
                depth -= 1
                if depth == 0:
                    break
            j += 1
        k = 0
        for n, (ln, var, lib, idx, off, _grp) in enumerate(loads):
            if i <= off <= j and not var and k < len(vars_in_order):
                # 记**组号**（= 该 `if let` 的起点）：同组的三条 load 是兄弟，彼此不算"中间插了别的 load"
                loads[n] = (ln, vars_in_order[k], lib, idx, off, tm.start())
                k += 1
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
        # **优先取 normal 帧**（`args[1]`）：控件的尺寸按 C# `MirImageControl.Size` =
        # `Library.GetTrueSize(Index)` 取的是**当前 `Index`（= normal 帧）**的尺寸；若拿 hover/pressed
        # 帧去比就会产出假阳性（实测：`npc_goods` 买钮 normal=Title[312]（76x25，与写死值一致），
        # 但按"最后一个匹配"取到 hover=313（80x25）⇒ 报了一条假 FAIL）。
        normal_handle = args_probe[1] if len(args_probe) > 1 else ""
        same = [l for l in prev if l[1] == normal_handle]
        lline, _var, lib, idx, _off, lgrp = (same[-1] if same else prev[0])
        # 再收紧两档，压掉"变量名撞车"的假阳性：
        #   ① 距离 ≤ 8 行（`if let Some(h) = load…` 紧跟着 `spawn_*(p, h, …)` 的写法）；
        #   ② 这两行之间**不许再有别的 load**（否则说明这次的 h 来自更近的那次）。
        if line - lline > 8:
            continue
        # 中间有**别的 load** 才算"这次句柄来自更近的那次"；**同一 `if let` 元组内的兄弟 load 不算**
        # （`if let (Some(n),Some(h),Some(pr)) = (load…,load…,load…)` 里 `pr` 必然夹在中间，
        #  按"任何 load"判会把整组否掉 —— 实测就是这样漏掉 big_map 滚屏箭头那条的）
        if any(lline < l[0] <= line and l[5] != lgrp for l in loads):
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
            # 自证时扫描的是 %TEMP% 下的副本（可能在别的盘符）——`relpath` 跨盘会抛，退回原路径
            try:
                rel = os.path.relpath(path)
            except ValueError:
                rel = path
            rows.append({
                "file": rel,
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
    ap.add_argument("--selftest", action="store_true",
                    help="跑正/负对照：临时改坏一处尺寸必须被报出，未改的副本必须 0 命中")
    ap.add_argument("--known", default="",
                    help="已知待核清单（每行 `file<TAB>lib<TAB>idx<TAB>w<TAB>h`，`#` 开头为注释）。"
                         "清单内的命中只提示、不判红；**新增**命中才 FAIL。"
                         "默认取脚本同目录的 control_size_audit_known.txt（存在时）。")
    a = ap.parse_args()

    if a.selftest:
        return selftest(a)

    known_path = a.known or os.path.join(os.path.dirname(os.path.abspath(__file__)),
                                         "control_size_audit_known.txt")
    known = set()
    if known_path and os.path.exists(known_path):
        for line in open(known_path, encoding="utf-8"):
            line = line.split("#", 1)[0].strip()
            if not line:
                continue
            parts = line.split()
            if len(parts) >= 5:
                known.add((parts[0].replace("/", os.sep), parts[1], int(parts[2]),
                           float(parts[3]), float(parts[4])))

    rows = []
    root = os.path.join(a.repo, a.subdir)
    for dirpath, _dirs, files in os.walk(root):
        for fn in sorted(files):
            if fn.endswith(".rs"):
                scan_file(os.path.join(dirpath, fn), a.data, rows)
    rows.sort(key=lambda r: (r["file"], r["line"]))
    new_rows = []
    for r in rows:
        key = (os.path.normpath(r["file"]), r["lib"], r["index"], r["explicit"][0], r["explicit"][1])
        r["known"] = key in known
        if not r["known"]:
            new_rows.append(r)
        print("%-52s:%-5d %s[%d] 美术=%sx%s 写死=%gx%g%s" % (
            r["file"], r["line"], r["lib"], r["index"], r["art"][0], r["art"][1],
            r["explicit"][0], r["explicit"][1], ("（来自" + r["via"] + "）") if r.get("via") else ""))
    print(f"合计 {len(rows)} 处「写死尺寸 ≠ 美术原生尺寸」，其中已知待核 {len(rows) - len(new_rows)}、"
          f"**新增 {len(new_rows)}**")
    if new_rows:
        print("VERDICT=FAIL：写死尺寸与美术不一致——要么改成按图头取尺寸（spawn_image_native），"
              "要么人工确认是「按比例裁宽」后加白名单并说明理由")
        return 1
    if rows:
        print(f"VERDICT=PASS（新增 0；{len(rows)} 条已在 {os.path.basename(known_path)} 登记为待核）")
    else:
        print("VERDICT=PASS：0 处写死尺寸与美术不一致")
    return 0


def load_known(path):
    known = set()
    if path and os.path.exists(path):
        for line in open(path, encoding="utf-8"):
            line = line.split("#", 1)[0].strip()
            if not line:
                continue
            parts = line.split()
            if len(parts) >= 5:
                known.add((os.path.normpath(parts[0].replace("/", os.sep)), parts[1], int(parts[2]),
                           float(parts[3]), float(parts[4])))
    return known


def row_key(r):
    return (os.path.normpath(r["file"]), r["lib"], r["index"], r["explicit"][0], r["explicit"][1])


def _scan_root(root, subdir, data_dir):
    rows = []
    for dirpath, _dirs, files in os.walk(os.path.join(root, subdir)):
        for fn in sorted(files):
            if fn.endswith(".rs"):
                scan_file(os.path.join(dirpath, fn), data_dir, rows)
    return rows


def selftest(a):
    """正/负对照：证明这条门禁**真的会红**，而不是恒绿。

    做法：把 `Client-Bevy/src` 复制到临时目录，① 原样扫 → 必须 0 命中（负对照）；
    ② 把 `group.rs` 里的 `spawn_image_native(… Title, 5 …)` 改回**修复前的写法**
    （`if let Some(h) = load_lib_image(… Title, 5) { spawn_image(p, h, …, 57.0, 15.0, …) }`，
    美术是 55x15）→ 必须报出来（正对照）。

    注意这两条也**界定了扫描面**：本工具只认「`load` 的句柄变量名 ↔ 后续 spawn 的同一个变量」
    这种形式（原版移植里最普遍）；把 `load_lib_image(...)` **内联**当参数传给 spawn 的写法它扫不到
    （正对照第一次就是按内联写法定制的，结果扫不出来 ⇒ 改成上面这个真实历史形态才成立）。
    """
    src = os.path.join(a.repo, a.subdir)
    ok = True
    with tempfile.TemporaryDirectory() as tmp:
        dst = os.path.join(tmp, a.subdir)
        shutil.copytree(src, dst)
        known_path = a.known or os.path.join(os.path.dirname(os.path.abspath(__file__)),
                                             "control_size_audit_known.txt")
        known = load_known(known_path)
        # 副本里的 `file` 是绝对路径 ⇒ 归一到"相对 subdir"的口径后再与 known 表比
        def rel_key(r):
            p = os.path.normpath(r["file"])
            marker = os.sep + os.path.normpath(a.subdir) + os.sep
            i = p.find(marker)
            rel = p[i + 1:] if i >= 0 else p
            return (os.path.normpath(rel), r["lib"], r["index"], r["explicit"][0], r["explicit"][1])

        neg = [r for r in _scan_root(tmp, a.subdir, a.data) if rel_key(r) not in known]
        print(f"[负对照] 原样扫描的**新增**命中 {len(neg)}（期望 0；已知待核表 {len(known)} 条不算）")
        ok &= (len(neg) == 0)

        target = os.path.join(dst, "game", "dialogs", "group.rs")
        text = open(target, encoding="utf-8").read()
        old = "let _ = spawn_image_native(p, &mut libs, &mut images, LibraryName::Title, 5, 18.0, 8.0, 9);"
        new = ("if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Title, 5) {\n"
               "            spawn_image(p, h, 18.0, 8.0, 57.0, 15.0, 9);\n        }")
        if old not in text:
            print("[正对照] 找不到要改坏的锚点 —— 门禁自证失败（锚点漂了，先更新 selftest）")
            return 1
        open(target, "w", encoding="utf-8").write(text.replace(old, new))
        pos = _scan_root(tmp, a.subdir, a.data)
        hit = [r for r in pos if r["file"].endswith("group.rs") and rel_key(r) not in known]
        print(f"[正对照] 改坏一处后命中 {len(hit)} 条（期望 ≥1）："
              + "; ".join(f"group.rs:{r['line']} {r['lib']}[{r['index']}] 美术={r['art']} 写死={r['explicit']}"
                          for r in hit))
        ok &= (len(hit) >= 1)

        # 正对照②（2026-09-26 补）：**表驱动循环**那条扫描面的自证。
        # 把 `menu.rs` 的 `*bw, *bh` 换回写死的 38x19（修复前的真实形态）→ 13 行表项必须全报。
        menu = os.path.join(dst, "game", "dialogs", "menu.rs")
        mtext = open(menu, encoding="utf-8").read()
        m_new = mtext.replace("*bw, *bh, 10", "38.0, 19.0, 10")
        if m_new == mtext:
            print("[正对照②] 找不到要改坏的锚点 `*bw, *bh, 10` —— 门禁自证失败（锚点漂了）")
            return 1
        open(menu, "w", encoding="utf-8").write(m_new)
        pos2 = [r for r in _scan_root(tmp, a.subdir, a.data)
                if r["file"].endswith("menu.rs") and rel_key(r) not in known]
        print(f"[正对照②] 表驱动循环写死 38x19 后命中 {len(pos2)} 条（期望 ≥1）："
              + "; ".join(f"menu.rs:{r['line']} {r['lib']}[{r['index']}] 美术={r['art']} 写死={r['explicit']}"
                          for r in pos2[:3]))
        ok &= (len(pos2) >= 1)
    print("VERDICT=" + ("PASS" if ok else "FAIL") + "（正/负对照）")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
