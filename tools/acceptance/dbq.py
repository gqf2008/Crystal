"""只读查询服务端 DB（ServerRust/Data/crystal.db），供验收夹具当判据来源。

用法：python dbq.py "select mail_id, collected from mail where mail_id=45"
（只读打开，绝不写库——验收脚本不许改服务端状态。）
"""
import sqlite3
import sys

DB = r"E:\Users\gxh\Documents\GitHub\Crystal\ServerRust\Data\crystal.db"


def main() -> int:
    if len(sys.argv) < 2:
        print("用法: dbq.py <SQL>")
        return 2
    con = sqlite3.connect(f"file:{DB.replace(chr(92), '/')}?mode=ro", uri=True)
    try:
        for row in con.execute(sys.argv[1]):
            print(row)
    except Exception as exc:
        print(f"SQL 失败: {exc}", file=sys.stderr)
        return 1
    finally:
        con.close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
