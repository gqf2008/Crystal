"""给 fresh_mirdb_deploy.ps1 用：打印定义表 / 运行态表的行数 JSON（只读）。"""
import json
import sqlite3
import sys

db = sys.argv[1]
runtime = "--runtime" in sys.argv[2:]
accounts_mode = "--accounts" in sys.argv[2:]
con = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
cur = con.cursor()
names = [n for (n,) in cur.execute("select name from sqlite_master where type='table'")]


def count(table):
    if table not in names:
        return 0
    return cur.execute(f"select count(*) from {table}").fetchone()[0]


if accounts_mode:
    # 迁移演练用：账号/角色/背包抽样 + 从 `pbkdf2_sha1$<b64salt>$…` 里解出 salt（十六进制）。
    import base64

    out = {
        "accounts": count("accounts"),
        "characters": count("characters"),
        "inventory_rows": count("inventory_backpack"),
    }
    if "characters" in names:
        out["character_names"] = [
            r[0]
            for r in cur.execute("select name from characters order by name")
        ]
        out["character_sample"] = [
            {"name": r[0], "level": r[1], "map": r[2]}
            for r in cur.execute(
                "select name, level, map_index from characters order by name limit 3"
            )
        ]
    if "accounts" in names:
        # `--accounts <username>` 时取该账号的 salt（演练要用它重置密码）；不给则取第一行
        idx = sys.argv.index("--accounts")
        wanted = (
            sys.argv[idx + 1]
            if idx + 1 < len(sys.argv) and not sys.argv[idx + 1].startswith("--")
            else ""
        )
        row = (
            cur.execute(
                "select username, password_hash from accounts where username=?", (wanted,)
            ).fetchone()
            if wanted
            else cur.execute("select username, password_hash from accounts limit 1").fetchone()
        )
        if row:
            out["reset_account"] = row[0]
            parts = (row[1] or "").split("$")
            if len(parts) == 3 and parts[0] == "pbkdf2_sha1":
                try:
                    out["reset_salt"] = base64.b64decode(parts[1]).hex()
                except Exception:
                    pass
    print(json.dumps(out))
elif runtime:
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
if not accounts_mode:
    print(json.dumps(out))
