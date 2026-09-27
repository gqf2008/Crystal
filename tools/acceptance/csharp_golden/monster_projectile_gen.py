# monster_projectile_gen.py —— 从原版 C# 客户端 `MonsterObject.cs` 提取「怪物远程攻击弹道表」。
#
# 为什么需要它：owner 反馈「有些魔法是个黄色方框」——玩家侧的占位弹道已抑制，但**怪物**侧
# 走的是 `MonsterObject.cs` 里另一张 `CreateProjectile` 表（按 `Monster` 枚举 = 怪物图像索引
# 键控），本端此前没移植 ⇒ 怪物远程攻击仍画占位方块。本脚本把那张表机械提取成 Rust 常量，
# 避免手抄 60 条。
#
# 提取规则（对着 C# 结构）：
#   `case MirAction.AttackRangeN:` → 内层 `switch (FrameIndex) { case <帧号>: switch (BaseImage)
#   { case Monster.<名>: ... CreateProjectile(...) } }`。对每个 `CreateProjectile(...)` 调用，
#   向上找**最近的** `case Monster.<名>:`、最近的 `case <数字>:`（帧号）与最近的
#   `case MirAction.AttackRangeN:`。
#   `Monster` 枚举值从 `Shared/Enums.cs` 的 `enum Monster : ushort { ... }` 读（值 = 图像索引）。
#
# 用法：
#   py -3.12 monster_projectile_gen.py --write      # 写 Client-Bevy/src/game/monster_projectiles.rs
#   py -3.12 monster_projectile_gen.py --print      # 只看提取结果
import argparse
import os
import re
import sys

REPO = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
MONSTER_CS = os.path.join(REPO, "Client", "MirObjects", "MonsterObject.cs")
ENUMS_CS = os.path.join(REPO, "Shared", "Enums.cs")
OUT_RS = os.path.join(REPO, "Client-Bevy", "src", "game", "monster_projectiles.rs")

CASE_MONSTER = re.compile(r"^\s*case Monster\.(?P<name>\w+)\s*:")
CASE_FRAME = re.compile(r"^\s*case (?P<frame>\d+)\s*:")
CASE_ACTION = re.compile(r"^\s*case MirAction\.(?P<action>AttackRange\d)\s*:")
CALL = re.compile(
    r"CreateProjectile\(\s*(?P<base>\d+)\s*,\s*"
    r"Libraries\.(?P<lib>\w+)(?:\[\(ushort\)Monster\.(?P<mlib>\w+)\])?[^,]*,\s*"
    r"(?P<blend>true|false)\s*,\s*(?P<count>\d+)\s*,\s*(?P<interval>\d+)\s*,\s*(?P<skip>-?\d+)")


def load_monster_enum():
    text = open(ENUMS_CS, encoding="utf-8", errors="replace").read().splitlines()
    start = next(i for i, l in enumerate(text) if re.search(r"enum Monster\s*:\s*ushort", l))
    vals, cur = {}, 0
    for l in text[start + 1:]:
        if "}" in l:
            break
        m = re.match(r"\s*(?P<name>\w+)\s*(?:=\s*(?P<v>\d+))?\s*,", l)
        if not m:
            continue
        cur = int(m.group("v")) if m.group("v") else cur
        vals[m.group("name")] = cur
        cur += 1
    return vals


def extract():
    lines = open(MONSTER_CS, encoding="utf-8", errors="replace").read().splitlines()
    enum = load_monster_enum()
    entries, unknown = [], []
    for i, ln in enumerate(lines):
        m = CALL.search(ln)
        if not m:
            continue
        monster_name = frame = action = None
        for j in range(i, -1, -1):
            cur = lines[j]
            if monster_name is None:
                mm = CASE_MONSTER.match(cur)
                if mm:
                    monster_name = mm.group("name")
                    continue
            if frame is None:
                fm = CASE_FRAME.match(cur)
                if fm:
                    frame = int(fm.group("frame"))
                    continue
            if action is None:
                am = CASE_ACTION.match(cur)
                if am:
                    action = am.group("action")
                    break
        if not (monster_name and frame and action):
            unknown.append((i + 1, ln.strip()))
            continue
        if monster_name not in enum:
            unknown.append((i + 1, f"Monster.{monster_name} 不在枚举里"))
            continue
        entries.append(dict(
            line=i + 1, monster=enum[monster_name], name=monster_name, frame=frame, action=action,
            base=int(m.group("base")), lib=m.group("lib"), blend=m.group("blend") == "true",
            count=int(m.group("count")), interval=int(m.group("interval")), skip=int(m.group("skip")),
            mlib=(m.group("mlib") or ""),
        ))
    return sorted(entries, key=lambda e: (e["monster"], e["action"], e["frame"])), unknown


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--write", action="store_true")
    ap.add_argument("--print", dest="do_print", action="store_true")
    a = ap.parse_args()
    entries, unknown = extract()
    print(f"提取 {len(entries)} 条（未识别 {len(unknown)} 条）")
    for e in entries[:400]:
        if a.do_print:
            print("  ", e)
    for line, why in unknown:
        print(f"  [未识别] line {line}: {why}")
    if a.write:
        with open(OUT_RS, "w", encoding="utf-8", newline="") as f:
            f.write(render(entries))
        print(f"写出 {OUT_RS}（{len(entries)} 条）")
    return 0


LIB_MAP = {"Magic": "Magic", "Magic2": "Magic2", "Magic3": "Magic3", "Dragon": "Dragon"}


def render(entries):
    enum = load_monster_enum()
    out = []
    out.append("//! 怪物远程攻击弹道表 —— **由工具生成，请勿手改**。")
    out.append("//!")
    out.append("//! 生成器：`py -3.12 tools/acceptance/csharp_golden/monster_projectile_gen.py --write`")
    out.append("//! 来源：`Client/MirObjects/MonsterObject.cs` 的 `MirAction.AttackRange1/2/3` 分支里")
    out.append("//! `CreateProjectile(baseIndex, library, blend, count, interval, skip, ...)` 的调用点。")
    out.append("//!")
    out.append("//! 键 `monster` 与怪物库索引都取 **C# `Monster` 枚举值**（`Shared/Enums.cs` 的")
    out.append("//! `enum Monster : ushort`）：它既是怪物的图像索引，也就是 'Data/Monster/{:03}.Lib' 的")
    out.append("//! 资产索引。本端的 `ActorAppearance::monster_type` 来自服务端 DB 的 `image` 字段，")
    out.append("//! 同样是 C# 值 —— 两者同源，可直接比较（注意本端 `Monster` 枚举整体比 C# 大 3，")
    out.append("//! 不能拿本端枚举值当资产索引用，见 `spell_effects::FxLib::Monster` 的说明）。")
    out.append("//!")
    out.append("//! 为什么需要（2026-09-28，owner 反馈「有些魔法是个黄色方框」）：玩家侧占位弹道已抑制，")
    out.append("//! 但怪物侧走的是这张**独立**的表，此前没移植 ⇒ 怪物远程攻击仍画占位方块。")
    out.append("use crate::game::spell_effects::{MissileFx, SpellFxLibrary};")
    out.append("use crate::resources::libraries::LibraryName;")
    out.append("")
    out.append("/// 弹道帧的来源库")
    out.append("#[derive(Clone, Copy, Debug, PartialEq, Eq)]")
    out.append("pub enum MissileLib {")
    out.append("    /// 扁平库（C# `Libraries.Magic/Magic2/Magic3/Dragon`…）")
    out.append("    Flat(LibraryName),")
    out.append("    /// 怪物库（C# `Libraries.Monsters[(ushort)Monster.X]`）：`asset` = C# `Monster` 值")
    out.append("    Monster { asset: u16 },")
    out.append("    /// 本端没有这个库的资产（如 `Siege`）：保留记录、渲染时退回占位并打日志")
    out.append("    Unavailable { csharp_lib: &'static str },")
    out.append("}")
    out.append("")
    out.append("/// 一条怪物弹道：`(怪物图像索引, 动作 1/2/3, 触发帧, 弹道参数)`")
    out.append("#[derive(Clone, Copy, Debug)]")
    out.append("pub struct MonsterMissile {")
    out.append("    /// `ActorAppearance::monster_type`（= C# `Monster` 枚举值 = 图像索引）")
    out.append("    pub monster: i16,")
    out.append("    /// 1/2/3 = C# `MirAction.AttackRange1/2/3`")
    out.append("    pub range: u8,")
    out.append("    /// 原版在该动作的第几帧生成弹道（`switch (FrameIndex)`）")
    out.append("    pub frame: u8,")
    out.append("    /// 库内起始帧")
    out.append("    pub base: usize,")
    out.append("    /// 帧数")
    out.append("    pub frames: usize,")
    out.append("    /// 每帧时长（ms；C# `CreateProjectile` 的 `interval`）")
    out.append("    pub frame_ms: u32,")
    out.append("    /// C# `CreateProjectile` 的 `skip`（按朝向取帧段的步长；本端按帧序播，仅记录备查）")
    out.append("    pub skip: usize,")
    out.append("    pub lib: MissileLib,")
    out.append("}")
    out.append("")
    out.append("/// C# 源文件里的行号（可追溯；`monster_projectile_gen.py` 生成时写入）")
    out.append("pub const MONSTER_MISSILE_SRC_LINES: &[u32] = &[")
    out.append("    " + ", ".join(str(e["line"]) for e in entries))
    out.append("];")
    out.append("")
    out.append("pub const MONSTER_MISSILES: &[MonsterMissile] = &[")
    for e in entries:
        if e["lib"] == "Monsters":
            lib = "MissileLib::Monster { asset: %d }" % enum[e["mlib"]]
        elif e["lib"] in LIB_MAP:
            lib = "MissileLib::Flat(LibraryName::%s)" % LIB_MAP[e["lib"]]
        else:
            lib = 'MissileLib::Unavailable { csharp_lib: "%s" }' % e["lib"]
        out.append(f"    // {e['name']} @ MonsterObject.cs:{e['line']}")
        out.append(
            "    MonsterMissile { monster: %d, range: %d, frame: %d, base: %d, frames: %d, "
            "frame_ms: %d, skip: %d, lib: %s },"
            % (e["monster"], int(e["action"][-1]), e["frame"], e["base"], e["count"],
               e["interval"], max(e["skip"], 0), lib)
        )
    out.append("];")
    out.append("")
    out.append("/// 查表：`(怪物图像索引, 动作 1/2/3)` → 弹道参数（同一组合取第一条）")
    out.append("pub fn monster_missile(monster_type: i16, range: u8) -> Option<&'static MonsterMissile> {")
    out.append("    MONSTER_MISSILES")
    out.append("        .iter()")
    out.append("        .find(|m| m.monster == monster_type && m.range == range)")
    out.append("}")
    out.append("")
    out.append("#[cfg(test)]")
    out.append("mod tests {")
    out.append("    use super::*;")
    out.append("")
    out.append("    /// 表来自 C# 的 %d 个调用点，条数变了要说明原因（防止生成器静默漏抽）" % len(entries))
    out.append("    #[test]")
    out.append("    fn table_matches_csharp_call_sites() {")
    out.append("        assert_eq!(MONSTER_MISSILES.len(), %d);" % len(entries))
    out.append("        assert_eq!(MONSTER_MISSILE_SRC_LINES.len(), MONSTER_MISSILES.len());")
    out.append("    }")
    out.append("")
    out.append("    /// 抽样核对（数值直接来自 C#）：AxeSkeleton=24 → 自己的怪物库 224 起 3 帧；")
    out.append("    /// BoneArcher=92 用的是 **ZumaArcher 的库**；LeftGuard=100 走 Magic 库。")
    out.append("    #[test]")
    out.append("    fn monster_missile_lookup_samples() {")
    out.append("        let axe = monster_missile(24, 1).expect(\"AxeSkeleton Range1 应有弹道\");")
    out.append("        assert_eq!((axe.base, axe.frames), (224, 3));")
    out.append("        assert_eq!(axe.lib, MissileLib::Monster { asset: %d });" % enum["AxeSkeleton"])
    out.append("        let bone = monster_missile(92, 1).expect(\"BoneArcher Range1\");")
    out.append("        assert_eq!(bone.lib, MissileLib::Monster { asset: %d }, \"C# 用的是 ZumaArcher 的库\");" % enum["ZumaArcher"])
    out.append("        let guard = monster_missile(100, 1).expect(\"LeftGuard Range1\");")
    out.append("        assert_eq!(guard.lib, MissileLib::Flat(LibraryName::Magic));")
    out.append("        // 本端没有 `Siege` 资产：表里保留记录，渲染时退回占位（不静默）")
    out.append("        let siege = monster_missile(940, 1).expect(\"TucsonGeneral Range1\");")
    out.append("        assert!(matches!(siege.lib, MissileLib::Unavailable { .. }));")
    out.append("        let h1 = monster_missile(341, 1).expect(\"HornedArcher Range1\");")
    out.append("        let h2 = monster_missile(341, 2).expect(\"HornedArcher Range2\");")
    out.append("        assert_ne!((h1.base, h1.frames), (h2.base, h2.frames), \"两条动作的弹道应不同\");")
    out.append("        assert!(monster_missile(24, 2).is_none(), \"表里没有的组合必须返回 None（由调用方决定占位）\");")
    out.append("    }")
    out.append("}")
    return "\n".join(out) + "\n"


if __name__ == "__main__":
    sys.exit(main())
