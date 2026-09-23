#!/usr/bin/env python3
"""从原版 C# 生成「施法特效表」（Client-Bevy/src/game/spell_effects.rs 的 SPELL_FX 段）。

为什么要有这个脚本：原版施法特效是 `PlayerObject.cs` 里 MirAction.Spell 分支的一张大 switch
（每个 Spell → 一条或多条 `new Effect(Libraries.X, start, frames, interval, this)`）。
手抄 100+ 条一定会错，所以表由 C# 源码机械生成，脚本留在仓库里以便将来对表。

用法（仓库根）：
    python Client-Bevy/tools/spell_effects_from_csharp.py            # 打印表
    python Client-Bevy/tools/spell_effects_from_csharp.py --write    # 写回 spell_effects.rs
"""
import argparse
import re
from pathlib import Path

CS = Path("Client/MirObjects/PlayerObject.cs")
BEGIN_MARK = "case Spell.FireBall:"
END_MARK = "case MirAction.Dead:"
OUT_MARK_BEGIN = "// ==== SPELL_FX_BEGIN"
OUT_MARK_END = "// ==== SPELL_FX_END ===="
OUT_MISSILE_BEGIN = "// ==== SPELL_MISSILE_BEGIN"
OUT_MISSILE_END = "// ==== SPELL_MISSILE_END ===="


def parse(text):
    lines = text.split("\n")
    start = next(i for i, l in enumerate(lines) if BEGIN_MARK in l)
    end = next(i for i, l in enumerate(lines) if i > start and END_MARK in l)
    cur = None
    out = []
    for ln in lines[start:end]:
        m = re.search(r"case Spell\.(\w+):", ln)
        if m:
            cur = m.group(1)
            continue
        m2 = re.search(
            r"new Effect\(Libraries\.(\w+),\s*([^,]+),\s*([0-9]+),\s*([^,)]+)", ln
        )
        if m2 and cur:
            out.append(
                (
                    cur,
                    m2.group(1),
                    m2.group(2).strip(),
                    int(m2.group(3)),
                    m2.group(4).strip(),
                )
            )
    return out


def parse_missiles(text):
    """抓 `MirAction.Spell` 分支里的 CreateProjectile（施法弹道）。

    原版：`CreateProjectile(baseIndex, library, blend, count, interval, skip)`
    —— count 帧、每帧 interval ms、skip 与光/步进有关（本端按帧序播，skip 记下来备查）。
    """
    lines = text.split("\n")
    cur_action = None
    cur_spell = None
    out = []
    for ln in lines:
        m = re.search(r"case MirAction\.(\w+):", ln)
        if m:
            cur_action = m.group(1)
            cur_spell = None
            continue
        m = re.search(r"case Spell\.(\w+):", ln)
        if m:
            cur_spell = m.group(1)
            continue
        m = re.search(
            r"CreateProjectile\(\s*([^,]+),\s*Libraries\.(\w+),\s*(?:true|false),\s*(\d+),\s*(\d+),\s*(\d+)",
            ln,
        )
        if m and cur_action == "Spell" and cur_spell:
            base = m.group(1).strip()
            if not base.isdigit():
                continue
            out.append((cur_spell, m.group(2), int(base), int(m.group(3)), int(m.group(4)), int(m.group(5))))
    return out


def render_missiles(rows):
    lines = [OUT_MISSILE_BEGIN + "（由 Client-Bevy/tools/spell_effects_from_csharp.py 生成，勿手改）===="]
    lines.append("/// 原版施法弹道表（`Client/MirObjects/PlayerObject.cs` MirAction.Spell 分支的 CreateProjectile）")
    lines.append("#[rustfmt::skip]  // 生成块：保持每条一行，便于 diff 与 --write 幂等")
    lines.append("pub const SPELL_MISSILE: &[(&str, MissileFx)] = &[")
    for spell, lib, base, count, interval, skip in rows:
        lines.append(
            "    (\"%s\", MissileFx { library: %s, base: %d, frames: %d, frame_ms: %d, skip: %d }),"
            % (spell, lib, base, count, interval, skip)
        )
    lines.append("];")
    lines.append(OUT_MISSILE_END)
    return "\n".join(lines)


def rust_entries(rows):
    entries = []
    for spell, lib, st, frames, interval in rows:
        if lib not in ("Magic", "Magic2", "Magic3"):
            continue
        d = re.search(r"Direction\s*\*\s*(\d+)", st)
        base = re.sub(r"\s*\+\s*\(?\s*\(?\s*int\s*\)?\s*Direction\s*\*\s*\d+\s*\)?", "", st).strip()
        base = base.replace("(", "").replace(")", "").strip()
        if not base.isdigit():
            continue
        mi = re.fullmatch(r"(\d+)", interval)
        ms = int(mi.group(1)) if mi else 0
        entries.append((spell, lib, int(base), frames, int(d.group(1)) if d else 0, ms))
    return entries


def render(entries):
    lines = [OUT_MARK_BEGIN + "（由 Client-Bevy/tools/spell_effects_from_csharp.py 生成，勿手改）===="]
    lines.append("/// 原版施法特效表（`Client/MirObjects/PlayerObject.cs` MirAction.Spell 分支机械生成）")
    lines.append("#[rustfmt::skip]  // 生成块：保持每条一行，便于 diff 与 --write 幂等")
    lines.append("pub const SPELL_FX: &[(&str, SpellFx)] = &[")
    for spell, lib, base, frames, step, ms in entries:
        lines.append(
            "    (\"%s\", SpellFx { library: %s, start: %d, frames: %d, dir_step: %d, interval_ms: %d }),"
            % (spell, lib, base, frames, step, ms)
        )
    lines.append("];")
    lines.append(OUT_MARK_END)
    return "\n".join(lines)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--write", action="store_true")
    a = ap.parse_args()
    text = CS.read_text(encoding="utf-8", errors="replace")
    rows = parse(text)
    entries = rust_entries(rows)
    block = render(entries)
    missiles = parse_missiles(text)
    missile_block = render_missiles(missiles)
    print("# C# MirAction.Spell 分支共 %d 条 Effect，其中魔法库条目 %d 条；施法弹道 %d 条" % (len(rows), len(entries), len(missiles)))
    if not a.write:
        print(block)
        print(missile_block)
        return
    out = Path("Client-Bevy/src/game/spell_effects.rs")
    txt = out.read_text(encoding="utf-8")
    i = txt.index(OUT_MARK_BEGIN)
    j = txt.index(OUT_MARK_END) + len(OUT_MARK_END)
    txt = txt[:i] + block + txt[j:]
    i = txt.index(OUT_MISSILE_BEGIN)
    j = txt.index(OUT_MISSILE_END) + len(OUT_MISSILE_END)
    out.write_text(txt[:i] + missile_block + txt[j:], encoding="utf-8")
    print("# 已写回 %s（特效 %d 条 / 弹道 %d 条）" % (out, len(entries), len(missiles)))


if __name__ == "__main__":
    main()
