"""持有一个 SQLite 写锁 N 秒（运行中存储故障注入用）。

为什么用它：`StorageDegradeDrill` 要验的是「服务端跑着的时候存储写不进去」，
最贴近真实的故障是**别的进程占着写锁**（备份/导出任务），而不是把文件设只读
（只读只影响新连接，且 SQLite 对已打开的句柄常常照写不误）。
`BEGIN IMMEDIATE` 会立刻拿 RESERVED 写锁，服务端随后的写请求会撞
`busy_timeout=5000`（ServerRust/src/db/mod.rs），5s 后以 `database is locked` 失败。

用法：python db_write_lock.py --db <path> --seconds 20
退出码：0=按预期持有并释放；2=打不开/拿不到写锁（注入失败，别当成"服务端没问题"）
"""
import argparse
import sqlite3
import sys
import time

ap = argparse.ArgumentParser()
ap.add_argument("--db", required=True)
ap.add_argument("--seconds", type=float, default=20.0)
ap.add_argument("--report", default="")
a = ap.parse_args()

try:
    conn = sqlite3.connect(a.db, timeout=1.0, isolation_level=None)
    conn.execute("PRAGMA busy_timeout=1000")
    conn.execute("BEGIN IMMEDIATE")
except Exception as exc:  # noqa: BLE001 - 注入失败要如实报出来
    print(f"LOCK_FAILED: {exc}")
    sys.exit(2)

t0 = time.time()
print(f"LOCK_HELD db={a.db} seconds={a.seconds}", flush=True)
time.sleep(a.seconds)
try:
    conn.execute("ROLLBACK")
    conn.close()
except Exception as exc:  # noqa: BLE001
    print(f"UNLOCK_WARN: {exc}")
print(f"LOCK_RELEASED after={time.time() - t0:.1f}s")
if a.report:
    with open(a.report, "w", encoding="utf-8") as fh:
        fh.write(f"held_seconds={time.time() - t0:.1f}\n")
