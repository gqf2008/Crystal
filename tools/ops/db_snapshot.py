"""读部署目录里那份 DB 的角色快照（只读），供回滚演练做"数据未变"判据。

刻意不依赖 tools/acceptance/dbq.py：ops 工具要能在**任何**检出/部署目录里跑
（本轮就踩过——Crystal 主检出是旧 HEAD，acceptance/dbq.py 根本不在那儿，
结果快照读成空串、判据"空 == 空"给了假 PASS）。

用法：python db_snapshot.py <db 路径> <角色名,角色名>
输出：JSON {"bevychar": {"gold":..,"level":..,"map_index":..,"x":..,"y":..}, ...}
找不到的角色写成 null（调用方据此判失败）。
"""
import json
import sqlite3
import sys


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
            out[name] = (
                {"gold": row[0], "level": row[1], "map_index": row[2], "x": row[3], "y": row[4]}
                if row
                else None
            )
        con.close()
    except Exception as exc:
        print(json.dumps({"error": f"{type(exc).__name__}: {exc}"}))
        return 1
    print(json.dumps(out, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
