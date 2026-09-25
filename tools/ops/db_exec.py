"""对**指定** Crystal SQLite 库执行单条 SQL（演练/运维用；与只读的 tools/acceptance/dbq.py 区分）。

用法：python db_exec.py <db_path> "update ..."        # 写
      python db_exec.py <db_path> "select ..." --read # 读（打印首列）
"""
import sqlite3
import sys

db, sql = sys.argv[1], sys.argv[2]
readonly = "--read" in sys.argv[3:]
if readonly:
    con = sqlite3.connect(f"file:{db.replace(chr(92), '/')}?mode=ro", uri=True)
    for row in con.execute(sql):
        print(row[0] if len(row) == 1 else row)
else:
    con = sqlite3.connect(db)
    con.execute(sql)
    con.commit()
    print("ok")
