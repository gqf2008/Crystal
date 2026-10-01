"""字号口径审计：逐窗把 C# 侧 `new Font(Settings.FontName, XF)` 的**磅值**与
本端 `spawn_label*` 的**像素值**并排列出（1pt = 4/3px）。

为什么需要它
------------
`Client/MirScenes/Dialogs/*.cs` 里 MirLabel 的 `Font` 用 `System.Drawing.Font`，
构造参数是**磅**（`Settings.FontSize = 8F`，见 `Client/Settings.cs:73`）；本端
`Client-Bevy/src/ui/theme.rs` 的 `spawn_label*` 收的是**像素**。两者差 4/3 倍，
且每扇窗的取值**不一致**（Relationship 是 10F≈13px，Friends 是 8F≈11px 而我方写 10px；
MailDialogs 用 `FontSize - 1 = 7F`，KeyboardLayoutDialog 用 `FontSize + 2/+1`）。
靠人眼逐窗翻代码容易漏，所以把「读两端字号」这一步机械化。

它**不做**的事：不判断哪个字号对（那要靠 A/B 帧的字形带/亮像素证据），
也不改代码。输出只是「两端字面量对照表」。

用法：
    py -3.12 tools/acceptance/csharp_golden/font_pt_px_audit.py            # 全部窗
    py -3.12 tools/acceptance/csharp_golden/font_pt_px_audit.py --window Friends
    py -3.12 tools/acceptance/csharp_golden/font_pt_px_audit.py --json out.json
"""

import argparse
import json
import os
import re
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))

# C# 侧全局默认（Client/Settings.cs:73）
SETTINGS_FONT_SIZE = 8.0

# 窗 → (C# 文件（相对仓库根）, C# 类名 or None, 本端 Rust 文件列表)
WINDOWS = {
    "Inventory": ("Client/MirScenes/Dialogs/InventoryDialog.cs", None, ["Client-Bevy/src/game/dialogs/inventory.rs"]),
    "Equipment": ("Client/MirScenes/Dialogs/CharacterDialog.cs", None, ["Client-Bevy/src/game/dialogs/character.rs"]),
    "Skills": ("Client/MirScenes/Dialogs/CharacterDialog.cs", None, ["Client-Bevy/src/game/skills.rs"]),
    "Quests": ("Client/MirScenes/Dialogs/QuestDialogs.cs", None, ["Client-Bevy/src/game/dialogs/quest_log.rs"]),
    "Options": ("Client/MirScenes/Dialogs/MainDialogs.cs", "OptionDialog", ["Client-Bevy/src/game/dialogs/option.rs"]),
    "Group": ("Client/MirScenes/Dialogs/GroupDialog.cs", None, ["Client-Bevy/src/game/dialogs/group.rs"]),
    "Friends": ("Client/MirScenes/Dialogs/FriendDialog.cs", None, ["Client-Bevy/src/game/dialogs/friend.rs"]),
    "Relationship": ("Client/MirScenes/Dialogs/RelationshipDialog.cs", None, ["Client-Bevy/src/game/dialogs/relationship.rs"]),
    "Guilds": ("Client/MirScenes/Dialogs/GuildDialog.cs", None, ["Client-Bevy/src/game/dialogs/guild.rs"]),
    "Ranking": ("Client/MirScenes/Dialogs/RankingDialog.cs", None, ["Client-Bevy/src/game/dialogs/ranking.rs"]),
    "Help": ("Client/MirScenes/Dialogs/HelpDialog.cs", None, ["Client-Bevy/src/game/dialogs/help.rs"]),
    "Keybind": ("Client/MirScenes/Dialogs/KeyboardLayoutDialog.cs", None, ["Client-Bevy/src/game/dialogs/keyboard_layout.rs"]),
    "Creature": ("Client/MirScenes/Dialogs/IntelligentCreatureDialogs.cs", None, ["Client-Bevy/src/game/dialogs/creature.rs"]),
    "MountWindow": ("Client/MirScenes/Dialogs/MountDialog.cs", None, ["Client-Bevy/src/game/dialogs/mount.rs"]),
    "Fishing": ("Client/MirScenes/Dialogs/FishingDialog.cs", None, ["Client-Bevy/src/game/dialogs/fishing.rs"]),
    "GameShop": ("Client/MirScenes/Dialogs/GameshopDialog.cs", None, ["Client-Bevy/src/game/dialogs/game_shop.rs"]),
    "Bigmap": ("Client/MirScenes/Dialogs/BigMapDialog.cs", None, ["Client-Bevy/src/game/dialogs/big_map.rs"]),
    "Minimap": ("Client/MirScenes/Dialogs/MainDialogs.cs", "MiniMapDialog", ["Client-Bevy/src/game/dialogs/minimap.rs"]),
    "Belt": ("Client/MirScenes/Dialogs/MainDialogs.cs", "MainDialog", ["Client-Bevy/src/game/dialogs/potion_belt.rs"]),
    "Skillbar": ("Client/MirScenes/Dialogs/MainDialogs.cs", "SkillBarDialog", ["Client-Bevy/src/game/hud.rs", "Client-Bevy/src/game/skills.rs"]),
}

LABEL_FNS = {
    # Rust 函数名 → size 参数的 0 基索引
    "spawn_label": 5,
    "spawn_label_plain": 5,
    "spawn_label_center": 6,
    "spawn_label_center_plain": 6,
    "spawn_outlined_label_block": 6,
    # `Client-Bevy/src/ui/outlined_text.rs` 直调（不经 theme 包装）的同族函数
    "spawn_outlined_label": 5,
    "spawn_outlined_label_center": 6,
}


def read_text(path):
    with open(path, "r", encoding="utf-8", errors="replace") as f:
        return f.read()


def strip_rust_strings_and_comments(text):
    """把字符串/字符字面量与注释替换成等长空白，保留换行，便于括号配对。"""
    out = list(text)
    i, n = 0, len(text)
    while i < n:
        c = text[i]
        if c == "/" and i + 1 < n and text[i + 1] == "/":
            while i < n and text[i] != "\n":
                out[i] = " "
                i += 1
        elif c == "/" and i + 1 < n and text[i + 1] == "*":
            depth = 0
            while i < n:
                if text[i] == "/" and i + 1 < n and text[i + 1] == "*":
                    depth += 1
                    out[i] = out[i + 1] = " "
                    i += 2
                    continue
                if text[i] == "*" and i + 1 < n and text[i + 1] == "/":
                    depth -= 1
                    out[i] = out[i + 1] = " "
                    i += 2
                    if depth == 0:
                        break
                    continue
                if text[i] != "\n":
                    out[i] = " "
                i += 1
        elif c == '"':
            out[i] = " "
            i += 1
            while i < n and text[i] != '"':
                if text[i] == "\\" and i + 1 < n:
                    out[i] = out[i + 1] = " "
                    i += 2
                    continue
                if text[i] != "\n":
                    out[i] = " "
                i += 1
            if i < n:
                out[i] = " "
                i += 1
        else:
            i += 1
    return "".join(out)


def split_top_level(args):
    """按顶层逗号切分实参串（输入应已去掉引号内容）。"""
    parts, depth, cur = [], 0, []
    for ch in args:
        if ch in "([{<":
            depth += 1
        elif ch in ")]}>":
            depth -= 1
        if ch == "," and depth == 0:
            parts.append("".join(cur).strip())
            cur = []
        else:
            cur.append(ch)
    if cur:
        parts.append("".join(cur).strip())
    return parts


def find_call_args(text, start):
    """从 `text[start]` 的 '(' 起找配对 ')'，返回 (实参串, 右括号后位置)。"""
    assert text[start] == "("
    depth, i, n = 0, start, len(text)
    while i < n:
        if text[i] == "(":
            depth += 1
        elif text[i] == ")":
            depth -= 1
            if depth == 0:
                return text[start + 1 : i], i + 1
        i += 1
    return None, n


def line_of(text, pos):
    return text.count("\n", 0, pos) + 1


def csharp_class_span(text, class_name):
    """返回 `class <name>` 的文本区间（到下一个同级/任意 class 行为止）。"""
    m = re.search(r"\bclass\s+" + re.escape(class_name) + r"\b", text)
    if not m:
        return None
    nxt = re.search(r"\n\s*(?:public|internal|private)?\s*(?:sealed\s+|static\s+|partial\s+)*class\s+\w+", text[m.end() :])
    end = m.end() + nxt.start() if nxt else len(text)
    return (m.start(), end)


def resolve_cs_font_expr(expr):
    """`8F` / `Settings.FontSize` / `Settings.FontSize - 1` → 磅值；解析不出返回 None。"""
    e = expr.strip().rstrip(",").strip()
    m = re.fullmatch(r"(\d+(?:\.\d+)?)[fF]?", e)
    if m:
        return float(m.group(1))
    m = re.fullmatch(r"Settings\.FontSize\s*([+-])\s*(\d+(?:\.\d+)?)[fF]?", e)
    if m:
        sign = 1.0 if m.group(1) == "+" else -1.0
        return SETTINGS_FONT_SIZE + sign * float(m.group(2))
    if re.fullmatch(r"Settings\.FontSize", e):
        return SETTINGS_FONT_SIZE
    return None


RE_CS_FONT = re.compile(r"new\s+Font\s*\(")
RE_CS_MIRLABEL = re.compile(r"new\s+MirLabel\s*(\(\s*\))?")


def csharp_fonts(text):
    """[(行号, float_pt|None, 原始串)]，只认 `new Font(...)` 且第一条实参是 Settings.FontName。"""
    spans = strip_rust_strings_and_comments(text)  # 同一套引号/注释处理对 C# 也适用
    res = []
    for m in RE_CS_FONT.finditer(spans):
        args, _ = find_call_args(spans, m.end() - 1)
        if args is None:
            continue
        parts = split_top_level(args)
        if len(parts) < 2:
            continue
        if "FontName" not in parts[0]:
            continue
        res.append((line_of(text, m.start()), resolve_cs_font_expr(parts[1]), parts[1]))
    return res


def csharp_label_effective_fonts(text):
    """每个 `new MirLabel {...}` 的**生效字号**：显式 `Font = new Font(...)` 取显式值，
    否则取 `MirLabel()` 构造里的默认 8F（`Client/MirControls/MirLabel.cs:180`）。

    返回 [(行号, pt|None, 'explicit'|'default')]。
    """
    spans = strip_rust_strings_and_comments(text)
    res = []
    for m in RE_CS_MIRLABEL.finditer(spans):
        line = line_of(text, m.start())
        # 找紧跟其后的初始化块（`new MirLabel` 或 `new MirLabel()` 之后可能隔空白/换行）
        j = m.end()
        while j < len(spans) and spans[j].isspace():
            j += 1
        if j < len(spans) and spans[j] == "{":
            # 匹配花括号
            depth, k = 0, j
            while k < len(spans):
                if spans[k] == "{":
                    depth += 1
                elif spans[k] == "}":
                    depth -= 1
                    if depth == 0:
                        break
                k += 1
            body = spans[j:k]
            fm = re.search(r"Font\s*=\s*new\s+Font\s*\(", body)
            if fm:
                args, _ = find_call_args(body, fm.end() - 1)
                parts = split_top_level(args) if args else []
                if len(parts) >= 2:
                    res.append((line, resolve_cs_font_expr(parts[1]), "explicit"))
                    continue
            elif "Font" in body:
                res.append((line, None, "variable"))
                continue
            res.append((line, SETTINGS_FONT_SIZE, "default"))
        else:
            res.append((line, SETTINGS_FONT_SIZE, "default"))
    return res


RE_RUST_LABEL = re.compile(r"\b(" + "|".join(LABEL_FNS) + r")\s*\(")
RE_RUST_TEXTFONT = re.compile(r"font_size\s*:\s*FontSize::Px\s*\(([^)]*)\)")
RE_RUST_CONST = re.compile(r"\bconst\s+([A-Z][A-Z0-9_]*)\s*:\s*f32\s*=\s*([0-9.]+)f?\s*;")


def rust_consts(text):
    spans = strip_rust_strings_and_comments(text)
    return {m.group(1): float(m.group(2)) for m in RE_RUST_CONST.finditer(spans)}


def _num(raw, consts):
    raw = raw.strip()
    try:
        return float(raw.rstrip("fF"))
    except ValueError:
        pass
    m = re.fullmatch(r"([A-Z][A-Z0-9_]*)", raw)
    if m and m.group(1) in consts:
        return consts[m.group(1)]
    return None


def rust_label_sizes(text):
    """[(行号, float_px|None, 原始串, 来源)]"""
    spans = strip_rust_strings_and_comments(text)
    consts = rust_consts(text)
    res = []
    for m in RE_RUST_LABEL.finditer(spans):
        fname = m.group(1)
        args, _ = find_call_args(spans, m.end() - 1)
        if args is None:
            continue
        parts = split_top_level(args)
        idx = LABEL_FNS[fname]
        if len(parts) <= idx:
            continue
        raw = parts[idx]
        val = _num(raw, consts)
        res.append((line_of(text, m.start()), val, raw, fname))
    for m in RE_RUST_TEXTFONT.finditer(spans):
        raw = m.group(1).strip()
        val = _num(raw, consts)
        res.append((line_of(text, m.start()), val, raw, "TextFont"))
    return res


def summarize(pairs):
    """[(行号, 值|None)] → {值: [行号...]}；None 归到 '?' 键下带原始串由调用方处理。"""
    d = {}
    for line, val in pairs:
        d.setdefault(val, []).append(line)
    return d


def audit_window(name, cs_path, cls, rust_paths):
    out = {"window": name, "csharp": {}, "ours": {}}
    full = os.path.join(ROOT, cs_path)
    text = read_text(full)
    span = csharp_class_span(text, cls) if cls else None
    scoped = text[span[0] : span[1]] if span else text
    offset = span[0] if span else 0
    fonts = csharp_fonts(scoped)
    label_fonts = csharp_label_effective_fonts(scoped)
    # 行号换算回整文件（scoped 是从整文件切的，行号需重算）
    base_line = text.count("\n", 0, offset) if offset else 0
    cs = {}
    for line, val, raw in fonts:
        cs.setdefault(val, []).append((base_line + line, raw))
    csl = {}
    for line, val, how in label_fonts:
        csl.setdefault(val, []).append((base_line + line, how))
    out["csharp"] = {
        "file": cs_path,
        "class": cls,
        # 生效字号：每个 MirLabel 实例（未设 Font 用 MirLabel 默认 8F）
        "pt": {str(k): {"px": (round(k * 4 / 3, 2) if k is not None else None),
                        "count": len(v),
                        "lines": [{"line": ln, "how": h} for ln, h in v]}
               for k, v in sorted(csl.items(), key=lambda kv: (kv[0] is None, kv[0]))},
        # 所有 `new Font(...)` 字面量（含非 MirLabel 的，如 ToolTip/TextBox）
        "all_pt": {str(k): {"px": (round(k * 4 / 3, 2) if k is not None else None),
                            "count": len(v),
                            "lines": [{"line": ln, "raw": r} for ln, r in v]}
                   for k, v in sorted(cs.items(), key=lambda kv: (kv[0] is None, kv[0]))},
    }
    ours = {}
    for rp in rust_paths:
        p = os.path.join(ROOT, rp)
        if not os.path.exists(p):
            continue
        for line, val, raw, fname in rust_label_sizes(read_text(p)):
            ours.setdefault(val, []).append((rp, line, raw, fname))
    out["ours"] = {
        "px": {str(k): {"count": len(v),
                        "sites": [{"file": f, "line": ln, "raw": r, "fn": fn} for f, ln, r, fn in v]}
               for k, v in sorted(ours.items(), key=lambda kv: (kv[0] is None, kv[0]))},
    }
    return out


def fmt_window(res):
    lines = [f"## {res['window']}  ({res['csharp']['file']}" + (f" :: {res['csharp']['class']}" if res['csharp']['class'] else "") + ")"]
    cs = res["csharp"]["pt"]
    cs_all = res["csharp"]["all_pt"]
    ours = res["ours"]["px"]
    cs_items = [(k, v) for k, v in cs.items() if k != "None"]
    cs_bad = cs.get("None")
    cs_all_items = [(k, v) for k, v in cs_all.items() if k != "None"]
    our_items = [(k, v) for k, v in ours.items() if k != "None"]
    our_bad = ours.get("None")
    lines.append("  C# MirLabel 生效 pt → px : " + (", ".join(f"{k}pt(={v['px']}px)×{v['count']}" for k, v in cs_items) or "(无 MirLabel)"))
    lines.append("  C# new Font 字面量       : " + (", ".join(f"{k}pt(={v['px']}px)×{v['count']}" for k, v in cs_all_items) or "(无)"))
    lines.append("  本端 px    : " + ", ".join(f"{k}px×{v['count']}" for k, v in our_items))
    if cs_bad:
        lines.append(f"  ⚠ C# 生效字号未解析: {[ (l['line'], l.get('how')) for l in cs_bad['lines'] ][:6]}")
    if our_bad:
        lines.append(f"  ⚠ 本端未解析: {[ (s['file'].split('/')[-1], s['line'], s['raw']) for s in our_bad['sites'] ][:6]}")
    # 逐 C# 档找最近的本端 px
    if cs_items and our_items:
        ours_vals = [float(k) for k, _ in our_items]
        notes = []
        for k, v in cs_items:
            want = float(v["px"])
            near = min(ours_vals, key=lambda x: abs(x - want))
            if abs(near - want) >= 1.0:
                notes.append(f"{k}pt({want:.1f}px) ↔ 最近本端 {near:g}px 差 {near - want:+.1f}px")
        if notes:
            lines.append("  ⚑ 落差 ≥1px: " + "; ".join(notes))
    return "\n".join(lines)


def main():
    global ROOT
    ap = argparse.ArgumentParser()
    ap.add_argument("--window", action="append", default=[])
    ap.add_argument("--json", default="")
    ap.add_argument("--repo", default=ROOT)
    args = ap.parse_args()
    ROOT = os.path.abspath(args.repo)
    names = args.window or list(WINDOWS)
    results = []
    for name in names:
        if name not in WINDOWS:
            print(f"skip unknown window {name}", file=sys.stderr)
            continue
        cs, cls, rust = WINDOWS[name]
        results.append(audit_window(name, cs, cls, rust))
    for r in results:
        print(fmt_window(r))
        print()
    if args.json:
        with open(args.json, "w", encoding="utf-8") as f:
            json.dump(results, f, ensure_ascii=False, indent=1)
        print(f"written {args.json}", file=sys.stderr)


if __name__ == "__main__":
    main()
