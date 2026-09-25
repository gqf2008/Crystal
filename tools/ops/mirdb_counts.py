"""给 fresh_mirdb_deploy.ps1 用：打印定义表 / 运行态表的行数 JSON（只读）。"""
import json
import sqlite3
import sys

db = sys.argv[1]
runtime = "--runtime" in sys.argv[2:]
con = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
cur = con.cursor()
names = [n for (n,) in cur.execute("select name from sqlite_master where type='table'")]


def count(table):
    if table not in names:
        return 0
    return cur.execute(f"select count(*) from {table}").fetchone()[0]


if runtime:
    out = {t: count(t) for t in ("accounts", "characters")}
else:
    out = {
        t: count(t)
        for t in (
            "item_infos",
            "monster_infos",
            "npc_infos",
            "map_infos",
            "magic_infos",
            "quest_infos",
            "map_respawns",
            "map_movements",
        )
    }
print(json.dumps(out))
