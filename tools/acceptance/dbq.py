r"""只读查询服务端 DB，供验收夹具当判据来源。

用法：python dbq.py "select mail_id, collected from mail where mail_id=45"
      python dbq.py --db <path> "select …"     # 指定库（**受测服务端**用的那个）
      CRYSTAL_DB_PATH=<path> python dbq.py "select …"
（只读打开，绝不写库——验收脚本不许改服务端状态。）

为什么要有 `--db`：本脚本原先**写死**仓库 dev 库（`ServerRust/Data/crystal.db`），
而验收把服务端起在别的工作目录（例如 `%TEMP%\e2e_workdir`，库是 `<wd>/Data/crystal.db`）时，
夹具的判据来源就与**受测服务端**不是同一个库——实测 l5g 因此把「已完成任务」集合读错、
选中一个已交付的任务，接取被服务端正确拒绝，报成假 FAIL。夹具的判据必须能指向受测实例。
"""
import os
import sqlite3
import sys

DB_DEFAULT = r"E:\Users\gxh\Documents\GitHub\Crystal\ServerRust\Data\crystal.db"


def main() -> int:
    args = sys.argv[1:]
    db = os.environ.get("CRYSTAL_DB_PATH") or DB_DEFAULT
    if args and args[0] in ("--db", "-d"):
        if len(args) < 3:
            print("用法: dbq.py [--db <path>] <SQL>", file=sys.stderr)
            return 2
        db, args = args[1], args[2:]
    elif args and args[0].startswith("--db="):
        db, args = args[0][len("--db="):], args[1:]
    if not args:
        print("用法: dbq.py [--db <path>] <SQL>", file=sys.stderr)
        return 2
    # 库不存在就**大声失败**：静默读到一个空/别的库会让夹具给出假绿或假红。
    if not os.path.exists(db):
        print(f"库不存在: {db}", file=sys.stderr)
        return 2
    con = sqlite3.connect(f"file:{db.replace(chr(92), '/')}?mode=ro", uri=True)
    try:
        for row in con.execute(args[0]):
            # 单列直接打印裸值（`('0106',)` 这种元组形式在 PowerShell 侧解析容易出错，
            # 夹具里已经因此把地图名读成 "('0106',)" 去过一次错误坐标）
            print(row[0] if len(row) == 1 else row)
    except Exception as exc:
        print(f"SQL 失败: {exc}", file=sys.stderr)
        return 1
    finally:
        con.close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
