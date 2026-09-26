r"""夹具**前置**：往某个角色的仓库里塞一件「客户端本地名表里没有」的物品。

**这是前置（准备被测状态），不是判据仪器**——判据一律走客户端状态（`storage_probe` 的
`occupied[].display` 与客户端日志的 `🏬/📦` 两行）。只读查询请用 `dbq.py`（那边明确
「绝不写库」）；本脚本存在的唯一理由是：`#782` 的「仓库按需请求」分支只有在
**仓库里存在一个当前会话本地物品名表里没有的索引**时才会走到，而这条状态没法用客户端
自己造出来——见下面「为什么不能用客户端自己造」。

## 为什么不能用客户端自己造（本轮实测踩过）

本地名表（`StorageState.item_names` 等）由 `UserInformation` 灌入，而 `UserInformation`
只带**背包 + 装备 + 任务格**里那些物品的名字。所以：

* 用 `@MAKE` 造一件新物品 → 它进背包 → 服务端下发 `UserInformation` → 名表里立刻有了它；
* 再把它 `StoreItem` 进仓库 → `StorageOpened` 时名表命中 ⇒ **不会**发 `RequestItemInfo`；
* 只有**换一个客户端会话**（名表从头建，且该索引不在背包/装备里）开仓库，才会走到按需请求。

那要跑两个客户端会话 + 两遍 NPC/密码闸门；本脚本走等价但确定性的做法：直接按受测服务端
**自己的**存库格式（`inventory_storage` 表，`item_json` = `UserItem` 的 serde JSON）插一行。

## 幂等

该角色仓库里已经有同 `item_index` 的物品就直接退出（`exit 0`，打印 `already-present`）——
反复跑不会把仓库塞满。默认只**新增到第一个空格**（不改动任何既有行）。

用法：
    py -3.12 seed_storage_probe_item.py --db <path> --character bevychar --item-index 221 \
        --unique-id 990221 [--dry-run]
"""

import argparse
import json
import sqlite3
import sys


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--db", required=True, help="受测服务端的库（ServerRust/Data/crystal.db）")
    ap.add_argument("--character", required=True)
    ap.add_argument("--item-index", type=int, required=True)
    ap.add_argument("--unique-id", type=int, default=990221)
    ap.add_argument("--dry-run", action="store_true")
    a = ap.parse_args()

    con = sqlite3.connect(a.db, timeout=10)
    con.execute("PRAGMA busy_timeout = 10000")
    rows = con.execute(
        "SELECT grid, item_json FROM inventory_storage WHERE character_name = ? ORDER BY grid",
        (a.character,),
    ).fetchall()
    if not rows:
        print(f"FAIL: {a.character} 的仓库是空的——无法借既有行的格式（先跑一次 l5e 建仓）", file=sys.stderr)
        return 2
    for grid, raw in rows:
        try:
            idx = json.loads(raw).get("item_index")
        except Exception:
            continue
        if idx == a.item_index:
            print(f"already-present character={a.character} item_index={a.item_index} grid={grid}")
            return 0

    template_grid, template_raw = rows[0]
    item = json.loads(template_raw)
    item["item_index"] = a.item_index
    item["unique_id"] = a.unique_id
    item["count"] = 1
    free = sorted(set(range(80)) - {g for g, _ in rows})[0]
    if a.dry_run:
        print(f"dry-run: would insert character={a.character} grid={free} item_index={a.item_index}")
        return 0
    con.execute(
        "INSERT INTO inventory_storage (character_name, grid, item_json) VALUES (?, ?, ?)",
        (a.character, free, json.dumps(item, separators=(",", ":"))),
    )
    con.commit()
    print(
        f"seeded character={a.character} grid={free} item_index={a.item_index} "
        f"(模板取自 grid={template_grid}，只改 item_index/unique_id/count)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
