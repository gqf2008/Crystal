"""给实机夹具造/重置一个英雄（幂等），用于经验曲线门禁 l5zf_hero_exp_curve.ps1。

为什么需要：本机 `heroes` 表长期为 0 行，而英雄只能经 NPC/任务流程创建 ——
"登录时按曲线重算英雄 max_experience"就永远没有端到端证据。
这里直接写测试存档（与 `seed_storage_probe_item.py` 同一套路）：

- 删除该角色已有的同名探针英雄（幂等），再插一行 `heroes`：
  `hero_index=1, level=<--level>, experience=0, max_experience=<-stale-max-exp>`；
- 顺手把 `characters.hero_index` 指到 1（`@SUMMONHERO` 的前置：`state.hero_index != 0`；
  服务端英雄槽位是 **1 基**：`next_index = (1..=maximum_hero_count).find(...)`，0 表示"无英雄"）；
- 用**故意过期的** `max_experience`（默认 100）写入，这样"服务端登录后按曲线重算"一旦失效，
  夹具读到的就是 100，而不是预期曲线值 —— 判据才有区分度。

用法：`py -3.12 seed_hero_probe.py --db <crystal.db> --character bevychar --level 100`
      `py -3.12 seed_hero_probe.py --db <crystal.db> --cleanup`（收尾：删探针英雄 + hero_index 归 0）
退出码：0 成功 / 2 参数或写库失败。
"""
import argparse
import sqlite3
import sys


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--db", required=True)
    ap.add_argument("--character", default="bevychar")
    ap.add_argument("--level", type=int, default=100)
    ap.add_argument("--name", default="ProbeHero")
    ap.add_argument("--stale-max-exp", type=int, default=100)
    ap.add_argument("--cleanup", action="store_true", help="删除探针英雄并把 characters.hero_index 归 0")
    args = ap.parse_args()

    try:
        con = sqlite3.connect(args.db)
        cur = con.cursor()
        if args.cleanup:
            cur.execute(
                "DELETE FROM heroes WHERE character_name = ? AND name = ?",
                (args.character, args.name),
            )
            cur.execute(
                "UPDATE characters SET hero_index = 0 WHERE name = ?",
                (args.character,),
            )
            con.commit()
            con.close()
            print(f"cleaned hero probe: character={args.character} name={args.name}")
            return 0
        cur.execute(
            "DELETE FROM heroes WHERE character_name = ? AND name = ?",
            (args.character, args.name),
        )
        cur.execute(
            """
            INSERT INTO heroes
                (character_name, hero_index, name, level, class, gender,
                 dead, sealed, autopot, experience, max_experience, hp, mp)
            VALUES (?, 1, ?, ?, 0, 0, 0, 0, 0, 0, ?, -1, -1)
            """,
            (args.character, args.name, args.level, args.stale_max_exp),
        )
        cur.execute(
            "UPDATE characters SET hero_index = 1 WHERE name = ?",
            (args.character,),
        )
        con.commit()
        row = cur.execute(
            "SELECT hero_index, name, level, experience, max_experience FROM heroes "
            "WHERE character_name = ? AND name = ?",
            (args.character, args.name),
        ).fetchone()
        con.close()
    except Exception as exc:  # noqa: BLE001 - 脚本面向夹具，任何失败都要带原因退出
        print(f"seed hero failed: {exc}")
        return 2

    print(
        "seeded hero: character={} index={} name={} level={} experience={} max_experience={}（故意过期）".format(
            args.character, row[0], row[1], row[2], row[3], row[4]
        )
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
