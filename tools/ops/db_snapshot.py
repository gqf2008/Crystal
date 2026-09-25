"""读部署目录里那份 DB 的角色快照（只读），供回滚演练做"数据未变"判据。

刻意不依赖 tools/acceptance/dbq.py：ops 工具要能在**任何**检出/部署目录里跑
（2026-09-23 踩过——主检出是旧 HEAD，acceptance/dbq.py 根本不在那儿，快照读成空串、
判据"空 == 空"给了假 PASS）。

用法：python db_snapshot.py <db 路径> <角色名,角色名>
输出：JSON，每个角色一个对象：
  {"gold":..,"level":..,"map_index":..,"x":..,"y":..,          # 原有 5 个标量（向后兼容）
   "tables": {"inventory_backpack": {"n": 12, "sha256": "ab12…"}, ...}}
找不到的角色写成 null（调用方据此判失败）。

**2026-09-25 扩展**：原来只快照 5 个标量，回滚若把背包/宠物/任务弄坏看不出来。
现在按表补「行数 + 规范化哈希」。为避免"合法漂移"造成假红，刻意**排除易变列**：
  - creatures：排除 active_hunger / active_expire_at / active_blackstone_time（会随时间走）
  - mail：排除 timestamp / body（时间戳与正文不影响"档有没有被回滚弄坏"）
  - friends：只取 friend_name / memo（friend_object_id 是**会话内**对象 id，登录后会变）
  - heroes：只取身份与成长字段（不含随登录变化的 hp/mp/autopot 之类）
其余表按排序键规范化后整行哈希（排序键见 _TABLES）。
"""
import hashlib
import json
import sqlite3
import sys

# (表名, 归属列, 参与哈希的列, 排序键)
_TABLES = [
    ("inventory_backpack", "character_name", ["grid", "item_json"], ["grid"]),
    ("inventory_equipment", "character_name", ["slot", "item_json"], ["slot"]),
    ("inventory_storage", "character_name", ["grid", "item_json"], ["grid"]),
    ("hero_inventory_backpack", "character_name", ["grid", "item_json"], ["grid"]),
    ("hero_inventory_equipment", "character_name", ["slot", "item_json"], ["slot"]),
    ("hero_magics", "character_name", ["spell", "level", "experience", "key", "toggled"], ["spell"]),
    ("heroes", "character_name",
     ["hero_index", "name", "level", "class", "gender", "sealed", "experience"], ["hero_index"]),
    ("creatures", "character_name",
     ["active_type", "active_custom_name", "active_pickup_mode", "active_enabled", "owned_json",
      "active_level"],
     ["active_type", "active_custom_name"]),
    ("completed_quests", "character_name", ["quest_index"], ["quest_index"]),
    ("friends", "character_name", ["friend_name", "memo"], ["friend_name"]),
    ("mail", "character_name",
     ["mail_id", "sender_name", "subject", "read_flag", "collected", "locked", "gold", "items_json"],
     ["mail_id"]),
]


def _table_digest(con: sqlite3.Connection, table: str, owner_col: str, cols, order_by, owner: str) -> dict:
    sql = f"select {', '.join(cols)} from {table} where {owner_col}=? order by {', '.join(order_by)}"
    rows = con.execute(sql, (owner,)).fetchall()
    canonical = json.dumps([list(r) for r in rows], ensure_ascii=False, separators=(",", ":"), default=str)
    return {"n": len(rows), "sha256": hashlib.sha256(canonical.encode("utf-8")).hexdigest()[:16]}


def main() -> int:
    if len(sys.argv) < 3:
        print("用法: db_snapshot.py <db> <name,name>", file=sys.stderr)
        return 2
    db, names = sys.argv[1], [n for n in sys.argv[2].split(",") if n]
    uri = f"file:{db.replace(chr(92), '/')}?mode=ro"
    out: dict = {}
    try:
        con = sqlite3.connect(uri, uri=True)
        for name in names:
            row = con.execute(
                "select gold, level, map_index, x, y from characters where name=?",
                (name,),
            ).fetchone()
            if not row:
                out[name] = None
                continue
            tables = {}
            for table, owner_col, cols, order_by in _TABLES:
                try:
                    tables[table] = _table_digest(con, table, owner_col, cols, order_by, name)
                except sqlite3.Error as exc:   # 表/列与当前 schema 不一致时也出快照，如实标注
                    tables[table] = {"error": str(exc)}
            out[name] = {
                "gold": row[0], "level": row[1], "map_index": row[2], "x": row[3], "y": row[4],
                "tables": tables,
            }
        con.close()
    except Exception as exc:
        print(json.dumps({"error": f"{type(exc).__name__}: {exc}"}))
        return 1
    print(json.dumps(out, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
