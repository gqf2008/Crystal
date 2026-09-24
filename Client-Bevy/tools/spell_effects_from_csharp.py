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
GS = Path("Client/MirScenes/GameScene.cs")
# 资产索引的真值：`Data/Monster/{:03}.Lib` 是按 **C# `Monster` 枚举值**编号的
# （本端 `SharedRust/src/enums.rs` 的枚举整体比 C# 大 3：C# `Guard = 0` / 本端 `Guard = 3`，
# 504 个同名项逐一核对全部 +3）。拿本端枚举值当资产索引会整段错位。
ENUMS_CS = Path("Shared/Enums.cs")
BEGIN_MARK = "case Spell.FireBall:"
END_MARK = "case MirAction.Dead:"
OUT_MARK_BEGIN = "// ==== SPELL_FX_BEGIN"
OUT_MARK_END = "// ==== SPELL_FX_END ===="
OUT_MISSILE_BEGIN = "// ==== SPELL_MISSILE_BEGIN"
OUT_MISSILE_END = "// ==== SPELL_MISSILE_END ===="
OUT_RANGE_BEGIN = "// ==== RANGE_MISSILE_BEGIN"
OUT_RANGE_END = "// ==== RANGE_MISSILE_END ===="
OUT_OBJECT_BEGIN = "// ==== OBJECT_FX_BEGIN"
OUT_OBJECT_END = "// ==== OBJECT_FX_END ===="
# `S.ObjectEffect` 的处理在 GameScene.cs 的 ObjectEffect 方法里，到 RangeAttack 为止
OBJ_BEGIN_MARK = "private void ObjectEffect(S.ObjectEffect p)"
OBJ_END_MARK = "private void RangeAttack(S.RangeAttack p)"


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


def parse_range_missiles(text):
    """抓 `MirAction.AttackRange{1,2,3}` 分支里的 `CreateProjectile`（弓/箭矢远程弹道）。

    与 MirAction.Spell 分支不同，这里有两层 switch：外层 FrameIndex、内层 `switch (Spell)`。
    必须做花括号深度跟踪，否则 `case Spell.Focus:` 之后的**普通弓射默认箭**
    （`case 5: CreateProjectile(1030, ...)`，在 Spell switch 之外）会被误记到 Focus 名下。

    `1930 + exFrameStart` 按 C# 现场赋值解析：`Spell.PoisonShot → 200`、`Spell.CrippleShot → 400`
    （`PlayerObject.cs:2839-2841`）。
    """
    lines = text.split("\n")
    depth = 0
    action = None
    spell_switch_depth = None   # 进入 `switch (Spell)` 后的深度
    spell_switch_pending = False  # C# 是 Allman 风格：`switch (Spell)` 与 `{` 不在同一行
    spell = None
    out = []
    for ln in lines:
        stripped = ln.strip()
        opens = ln.count("{")
        closes = ln.count("}")

        m = re.search(r"case MirAction\.(\w+):", ln)
        if m:
            action = m.group(1)
            spell = None
            spell_switch_depth = None
        elif re.search(r"switch\s*\(\s*Spell\s*\)", ln):
            spell_switch_pending = True
            spell = None
        elif stripped.startswith("case ") and spell_switch_depth is not None and depth == spell_switch_depth:
            ms = re.match(r"case Spell\.(\w+):", stripped)
            spell = ms.group(1) if ms else None

        if spell_switch_depth is not None and depth < spell_switch_depth:
            spell_switch_depth = None
            spell = None

        if spell_switch_pending and "{" in ln:
            spell_switch_depth = depth + opens
            spell_switch_pending = False

        if action and action.startswith("AttackRange"):
            mp = re.search(
                r"CreateProjectile\(\s*([^,]+),\s*Libraries\.(\w+),\s*(?:true|false),\s*(\d+),\s*(\d+),\s*(\d+)",
                ln,
            )
            if mp:
                expr = mp.group(1).strip()
                base = None
                if expr.isdigit():
                    base = int(expr)
                else:
                    m2 = re.fullmatch(r"(\d+)\s*\+\s*exFrameStart", expr)
                    if m2:
                        ex = {"PoisonShot": 200, "CrippleShot": 400}.get(spell or "", 0)
                        base = int(m2.group(1)) + ex
                if base is not None:
                    key = spell if spell else "DefaultArrow"
                    if not any(k == key for k, *_ in out):
                        out.append(
                            (key, mp.group(2), base, int(mp.group(3)), int(mp.group(4)), int(mp.group(5)))
                        )

        depth += opens - closes
    return out


def render_range_missiles(rows):
    lines = [OUT_RANGE_BEGIN + "（由 Client-Bevy/tools/spell_effects_from_csharp.py 生成，勿手改）===="]
    lines.append("/// 原版远程攻击弹道表（`Client/MirObjects/PlayerObject.cs` MirAction.AttackRange1/2/3 分支）")
    lines.append("/// `DefaultArrow` = 普通弓射（AttackRange1 的 `case 5:`，无技能）")
    lines.append("#[rustfmt::skip]  // 生成块：保持每条一行，便于 diff 与 --write 幂等")
    lines.append("pub const RANGE_MISSILE: &[(&str, MissileFx)] = &[")
    for spell, lib, base, count, interval, skip in rows:
        lines.append(
            "    (\"%s\", MissileFx { library: %s, base: %d, frames: %d, frame_ms: %d, skip: %d }),"
            % (spell, lib, base, count, interval, skip)
        )
    lines.append("];")
    lines.append(OUT_RANGE_END)
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


def _call_args(stmt, call):
    """取出 `new <call>(...)` 的实参列表（按顶层逗号切分）。找不到返回 None。"""
    key = "new %s(" % call
    i = stmt.find(key)
    if i < 0:
        return None
    j = i + len(key)
    depth = 1
    k = j
    while k < len(stmt) and depth > 0:
        c = stmt[k]
        if c in "([{":
            depth += 1
        elif c in ")]}":
            depth -= 1
        k += 1
    inner = stmt[j : k - 1]
    args, cur, d = [], "", 0
    for c in inner:
        if c in "([{":
            d += 1
        elif c in ")]}":
            d -= 1
        if c == "," and d == 0:
            args.append(cur.strip())
            cur = ""
        else:
            cur += c
    if cur.strip():
        args.append(cur.strip())
    return args


def _object_fx_start(expr):
    """C# 起始帧表达式 → Rust 字段。**未支持的形式直接报错**，绝不静默丢条目。"""
    e = re.sub(r"\s+", " ", expr.strip())
    if e.isdigit():
        return {"start": int(e)}
    m = re.fullmatch(r"(\d+) \+ \(\(int\)p\.EffectType \* (\d+)\)", e)
    if m:
        return {"start": int(m.group(1)), "step_effect_type": int(m.group(2))}
    m = re.fullmatch(r"(\d+) \+ \(\(int\)ob\.Direction \* (\d+)\)", e)
    if m:
        return {"start": int(m.group(1)), "dir_step": int(m.group(2))}
    m = re.fullmatch(r"(\d+) \+ \(CMain\.Random\.Next\((\d+)\) \* (\d+)\)", e)
    if m:
        return {
            "start": int(m.group(1)),
            "rand_step": int(m.group(3)),
            "rand_count": int(m.group(2)),
        }
    m = re.fullmatch(r"CMain\.Random\.Next\((\d+)\) == 0 \? (\d+) : (\d+)", e)
    if m:
        n, lo, hi = int(m.group(1)), int(m.group(2)), int(m.group(3))
        return {"start": lo, "rand_step": hi - lo, "rand_count": n}
    raise ValueError("GameScene.ObjectEffect 出现未支持的起始帧表达式: %r" % expr)


def parse_csharp_monster_values(text):
    """`public enum Monster : ushort { ... }` → `{名字: C# 值}`（= 资产索引）。"""
    # 注意别写成 "public enum Monster" —— 那会先匹配到 `public enum MonsterType : byte`
    i = text.index("public enum Monster :")
    j = text.index("{", i)
    depth = 0
    body = None
    for k in range(j, len(text)):
        if text[k] == "{":
            depth += 1
        elif text[k] == "}":
            depth -= 1
            if depth == 0:
                body = text[j + 1 : k]
                break
    if body is None:
        raise ValueError("Shared/Enums.cs 里找不到 Monster 枚举的闭合花括号")
    out = {}
    val = 0
    for line in body.split("\n"):
        line = line.split("//")[0].strip().rstrip(",")
        if not line:
            continue
        m = re.match(r"(\w+)\s*=\s*(\d+)", line)
        if m:
            out[m.group(1)] = int(m.group(2))
            val = int(m.group(2)) + 1
        elif re.match(r"^\w+$", line):
            out[line] = val
            val += 1
    return out


def _build_object_entry(case, stmt, cond, cs_monsters, race=None):
    """一条 `new Effect(...)` / `new DelayedExplosionEffect(...)` → Rust 字段串。"""
    race_field = [] if race is None else ["race: FxRace::%s" % race]
    call = (
        "DelayedExplosionEffect"
        if "new DelayedExplosionEffect(" in stmt
        else "Effect"
    )
    args = _call_args(stmt, call)
    if args is None or len(args) < 5:
        raise ValueError("无法解析 %s 的 %s 实参: %r" % (case, call, stmt))
    m = re.search(r"Libraries\.Monsters\[\(ushort\)Monster\.(\w+)\]", stmt)
    if m:
        monster = m.group(1)
        if monster not in cs_monsters:
            raise ValueError(
                "C# `Monster` 枚举里没有 %s（无法确定资产索引，禁止猜）" % monster
            )
        fields = [
            "lib: FxLib::Monster { rust: Monster::%s, lib: %d }"
            % (monster, cs_monsters[monster])
        ]
    else:
        ml = re.search(r"Libraries\.(\w+)", stmt)
        if not ml:
            raise ValueError("无法解析库: %r" % stmt)
        fields = ["lib: FxLib::Flat(%s)" % ml.group(1)]
    fields += ["%s: %s" % (k, v) for k, v in _object_fx_start(args[1]).items()]
    fields += race_field
    if not args[2].isdigit():
        raise ValueError("帧数不是字面量: %r" % args[2])
    fields.append("frames: %s" % args[2])
    if args[3].isdigit():
        fields.append("interval_ms: %s" % args[3])
    elif "Frame.Count * FrameInterval" in args[3]:
        fields.append("interval_ms: 0")
    else:
        raise ValueError("时长表达式未支持: %r" % args[3])
    if "Blend = false" in stmt:
        fields.append("blend: false")
    if cond == "zero":
        fields.append("when: FxWhen::EffectTypeZero")
    elif cond == "non_zero":
        fields.append("when: FxWhen::EffectTypeNonZero")
    if "Repeat = true" in stmt:
        group = {
            "MagicShieldUp": "MagicShield",
            "ElementalBarrierUp": "ElementalBarrier",
        }.get(case)
        if group is None:
            raise ValueError("Repeat = true 但光环分组未知: %s" % case)
        fields.append("repeat: FxRepeat::UntilDown(AuraGroup::%s)" % group)
    if "Repeat = p.Time > 0" in stmt:
        fields.append("repeat: FxRepeat::PacketTime")
    if call == "DelayedExplosionEffect":
        fields.append("repeat: FxRepeat::StageNot2")
    if "ob2.Effects.Add" in stmt:
        fields.append("target: FxTarget::EffectType")
    elif "ob.CurrentLocation" in stmt:
        fields.append("target: FxTarget::OwnerLocation")
    if "CMain.Time + p.DelayTime" in stmt:
        fields.append("delay_from_packet: true")
    fields.append("..ObjectFx::DEFAULT")
    return ", ".join(fields)


def parse_object_effects(text, cs_monsters):
    """抓 `GameScene.cs` 的 `ObjectEffect(S.ObjectEffect p)` switch。

    返回 `([(case 名, [字段串...])...], {case 名: 备注})`。

    两个 C# 分支形态在这里被显式处理（不处理就报错）：
    - `if (p.EffectType == 0) { ... } else { ... }`（KingGuard 的 753/763）→ `FxWhen`；
    - `DelayedExplosion` 的 `if (effectid < 0)` / `else if (effectid >= 0)` 两条语句
      **是二选一**（不是两条同播）：只保留按 stage 取帧段的那条，另一条是 stage=0 的等价表现。
    """
    lines = text.split("\n")
    start = next(i for i, l in enumerate(lines) if OBJ_BEGIN_MARK in l)
    end = next(i for i, l in enumerate(lines) if i > start and OBJ_END_MARK in l)
    body = lines[start:end]
    cases = []
    notes = {}
    cur = None
    cond = None
    race = None
    i = 0
    while i < len(body):
        # 先剥行尾注释：Critical 的 `//ob.Effects.Add(new Effect(...));` 是**被注释掉的**
        # 代码，不剥会把一条 C# 明确不画的特效抓成表项（且 CustomEffects 不是合法库名）。
        ln = body[i].split("//", 1)[0]
        m = re.search(r"case SpellEffect\.(\w+):", ln)
        if m:
            cur = m.group(1)
            cond = None
            race = None
            cases.append((cur, []))
            i += 1
            continue
        # 种族过滤（原版 `ob.Race != ObjectType.X ... return;`）：
        #   MagicShieldUp/Down        `!= Player && != Hero` → 玩家或英雄
        #   ElementalBarrierUp/Down   `!= Player`            → 仅玩家
        # 机制照抄、不猜：出现**没见过的** `ob.Race` 判据就报错，让它显式补规则。
        if "ob.Race" in ln:
            if re.search(
                r"if \(ob\.Race != ObjectType\.Player && ob\.Race != ObjectType\.Hero\) return;",
                ln,
            ):
                race = "PlayerOrHero"
            elif re.search(r"if \(ob\.Race != ObjectType\.Player\) return;", ln):
                race = "PlayerOnly"
            else:
                raise ValueError("未识别的 ob.Race 判据（禁止猜语义）: %r" % ln.strip())
            i += 1
            continue
        if re.search(r"if \(p\.EffectType == 0\)", ln):
            cond = "zero"
            i += 1
            continue
        if ln.strip() == "else" and cond == "zero":
            cond = "non_zero"
            i += 1
            continue
        if "if (effectid < 0)" in ln:
            cond = "delayed_no_prior"
            i += 1
            continue
        if "else if (effectid >= 0)" in ln:
            cond = "delayed_prior"
            i += 1
            continue
        if "new Effect(" in ln or "new DelayedExplosionEffect(" in ln:
            stmt = ln.strip()
            depth = stmt.count("(") - stmt.count(")")
            braces = stmt.count("{") - stmt.count("}")
            while depth > 0 or braces > 0 or not stmt.endswith(";"):
                i += 1
                if i >= len(body):
                    raise ValueError("ObjectEffect 语句未闭合: %r" % stmt)
                nxt = body[i].split("//", 1)[0].strip()
                stmt += " " + nxt
                depth += nxt.count("(") - nxt.count(")")
                braces += nxt.count("{") - nxt.count("}")
            if cur is not None:
                if cond == "delayed_no_prior":
                    notes[cur] = (
                        "C# 的 `effectid < 0` 支路是同一段动画的 stage=0"
                        "（本端按 stage 取帧段，effect_type=0 时帧段相同），故只保留 stage 那条"
                    )
                else:
                    cases[-1][1].append(
                        _build_object_entry(cur, stmt, cond, cs_monsters, race)
                    )
            i += 1
            continue
        i += 1
    return cases, notes


def render_object_effects(cases, notes):
    lines = [
        OUT_OBJECT_BEGIN
        + "（由 Client-Bevy/tools/spell_effects_from_csharp.py 生成，勿手改）===="
    ]
    lines.append(
        "/// 原版对象特效表（`Client/MirScenes/GameScene.cs` 的 `ObjectEffect` switch 机械生成）"
    )
    lines.append(
        "/// 空切片 = C# 明确不画；表里没有的名字 = C# 没有这个 case（调用方才退回占位表现）"
    )
    lines.append("#[rustfmt::skip]  // 生成块：保持每条一行，便于 diff 与 --write 幂等")
    lines.append("pub const OBJECT_FX: &[(&str, &[ObjectFx])] = &[")
    for name, entries in cases:
        if name in notes:
            lines.append("    // %s：%s" % (name, notes[name]))
        if not entries:
            lines.append('    ("%s", &[]),' % name)
            continue
        lines.append('    ("%s", &[' % name)
        for e in entries:
            lines.append("        ObjectFx { %s }," % e)
        lines.append("    ]),")
    lines.append("];")
    lines.append(OUT_OBJECT_END)
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
    ranges = parse_range_missiles(text)
    range_block = render_range_missiles(ranges)
    gs_text = GS.read_text(encoding="utf-8", errors="replace")
    cs_monsters = parse_csharp_monster_values(ENUMS_CS.read_text(encoding="utf-8", errors="replace"))
    obj_cases, obj_notes = parse_object_effects(gs_text, cs_monsters)
    obj_block = render_object_effects(obj_cases, obj_notes)
    obj_entries = sum(len(v) for _, v in obj_cases)
    print(
        "# C# MirAction.Spell 分支共 %d 条 Effect，其中魔法库条目 %d 条；施法弹道 %d 条；远程攻击弹道 %d 条"
        % (len(rows), len(entries), len(missiles), len(ranges))
    )
    print(
        "# C# GameScene.ObjectEffect 分支共 %d 个 case、%d 条 Effect"
        % (len(obj_cases), obj_entries)
    )
    if not a.write:
        print(block)
        print(missile_block)
        print(range_block)
        print(obj_block)
        return
    out = Path("Client-Bevy/src/game/spell_effects.rs")
    txt = out.read_text(encoding="utf-8")
    i = txt.index(OUT_MARK_BEGIN)
    j = txt.index(OUT_MARK_END) + len(OUT_MARK_END)
    txt = txt[:i] + block + txt[j:]
    i = txt.index(OUT_MISSILE_BEGIN)
    j = txt.index(OUT_MISSILE_END) + len(OUT_MISSILE_END)
    out.write_text(txt[:i] + missile_block + txt[j:], encoding="utf-8")
    txt = out.read_text(encoding="utf-8")
    i = txt.index(OUT_RANGE_BEGIN)
    j = txt.index(OUT_RANGE_END) + len(OUT_RANGE_END)
    out.write_text(txt[:i] + range_block + txt[j:], encoding="utf-8")
    txt = out.read_text(encoding="utf-8")
    i = txt.index(OUT_OBJECT_BEGIN)
    j = txt.index(OUT_OBJECT_END) + len(OUT_OBJECT_END)
    out.write_text(txt[:i] + obj_block + txt[j:], encoding="utf-8")
    print(
        "# 已写回 %s（特效 %d 条 / 施法弹道 %d 条 / 远程弹道 %d 条 / 对象特效 %d 条）"
        % (out, len(entries), len(missiles), len(ranges), obj_entries)
    )


if __name__ == "__main__":
    main()
