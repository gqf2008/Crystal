#!/usr/bin/env python3
"""精炼 E2E（`--refine-test`）专项准备/还原 —— 让精炼全流程能一键进常规回归（#2887）。

为什么需要：
  精炼是唯一「要站在 NPC 旁 + 背包带可精炼武器 + 精炼材料 + 结算耗时可调」的用例。
  缺任何一项时客户端只会报「未收到 NPCRefine / 未收到精炼结果」，看起来像功能坏了，
  实际是测试前置没做（此前靠 temp 目录里的手工脚本，未入库）。

用法（`scripts/run_real_e2e.ps1` 会自动调用，也可手工跑）：
  python scripts/e2e_refine_prep.py config      <server.toml> <out.toml>   # 开服前
  python scripts/e2e_refine_prep.py prepare-db  <crystal.db>              # 精炼用例前
  python scripts/e2e_refine_prep.py restore-db  <crystal.db>              # 精炼用例后
  python scripts/e2e_refine_prep.py prepare <db> <server.toml> <out.toml> # = config + prepare-db（手工一把跑）

**为什么要拆开**：改库会把角色挪到 246 图铁匠旁，而钓鱼/坐骑/商城等用例要求角色在 1 图钓鱼点。
配置必须在**开服前**就位（服务端只在启动时读一次配置），改库则必须紧贴精炼用例前后，
否则会把前面的用例一起带偏（实测：开服前就改库 → fishing/mount 双 FAIL）。

prepare 做的事：
  1. `PRAGMA wal_checkpoint(TRUNCATE)` —— 库是 WAL 模式，不 checkpoint 会拿到不一致快照（见经验条目）
  2. bevychar → 246 图 (13,10)：`Blacksmith_Carlos` 在 (12,10)，服务端 `CallNPC` 距离校验 ≤2 格
  3. 背包格 0 = 可精炼武器（客户端脚本固定存入「背包第一件物品」）
  4. `refine_log.materials_json` = 3 件属性材料 + 1 块矿石：结算才走「应用属性」分支（否则必碎）
  5. 金币兜底（开始精炼按 `(RequiredAmount*10)*RefineCost` 扣金）
  6. 生成 out.toml = server.toml 覆盖 `[refine] base_chance=100 / time_minutes=0`
     —— **不改仓库里的 server.toml**：服务端支持 `mir2_server <config>` 启动参数（main.rs:55-57），
     用一份临时配置跑 e2e，跑完删掉即可，避免「跑崩了配置留在改过的状态」。

restore 做的事：角色回 1 图钓鱼点（其余用例的位置前置）、清 refine_log、删掉本次的精炼测试武器。
"""
import json
import os
import re
import sqlite3
import sys

# 246 图铁匠旁（Blacksmith_Carlos (12,10)，距离 1）
REFINE_MAP, REFINE_X, REFINE_Y, REFINE_DIR = 246, 13, 10, 0
# 回 1 图钓鱼点（与 e2e_setup_db.py 的 SAFE 点一致：bevychar 朝左，前方 3 格是水）
BACK_MAP, BACK_X, BACK_Y, BACK_DIR = 1, 171, 667, 6

WEAPON_UID = 777001          # 精炼测试武器（背包格 0）
MAT_UIDS = [777011, 777012, 777013, 777014]
# 材料：三属性各一件 + 一块矿石（纯度 =  Dura/1000）；索引见 item_infos
MAT_ITEMS = [(10, 1000), (8, 1000), (18, 1000), (828, 5000)]
MATERIAL_SLOTS = 16          # = ServerRust `REFINE_MATERIAL_SLOTS`（服务端清空后写回的长度）


def wal_checkpoint(db_path: str) -> None:
    con = sqlite3.connect(db_path)
    con.execute("PRAGMA wal_checkpoint(TRUNCATE)")
    con.commit()
    con.close()


def item_from_template(template: dict, uid: int, item_index: int, dura: int) -> dict:
    d = json.loads(json.dumps(template))
    d["unique_id"] = uid
    d["item_index"] = item_index
    d["count"] = 1
    d["current_dura"] = dura
    d["max_dura"] = dura
    d["info"] = None  # 服务端登录时按 item_index 补全（UserInformation enrich）
    d["slots"] = [None] * 5
    return d


def find_template(cur: sqlite3.Cursor) -> dict | None:
    """取一件现成物品当模板（保证 JSON 字段与服务端同构）。"""
    for owner in ("bevychar", "bevy2char"):
        row = cur.execute(
            "SELECT item_json FROM inventory_backpack WHERE character_name=? "
            "ORDER BY grid LIMIT 1",
            (owner,),
        ).fetchone()
        if row:
            return json.loads(row[0])
        row = cur.execute(
            "SELECT item_json FROM inventory_equipment WHERE character_name=? "
            "ORDER BY slot LIMIT 1",
            (owner,),
        ).fetchone()
        if row:
            return json.loads(row[0])
    return None


def prepare_db(db_path: str) -> int:
    if not os.path.exists(db_path):
        print(f"refine prep: db not found: {db_path}")
        return 1
    wal_checkpoint(db_path)

    con = sqlite3.connect(db_path)
    cur = con.cursor()
    template = find_template(cur)
    if template is None:
        print("refine prep: 找不到可作模板的物品（bevychar/bevy2char 背包与装备都空）")
        con.close()
        return 1

    # 1) 站到铁匠旁
    cur.execute(
        "UPDATE characters SET map_index=?, x=?, y=?, direction=? WHERE name='bevychar'",
        (REFINE_MAP, REFINE_X, REFINE_Y, REFINE_DIR),
    )
    # 2) 背包格 0 = 可精炼武器（客户端 --refine-test 存入背包第一件物品）
    cur.execute("DELETE FROM inventory_backpack WHERE character_name='bevychar' AND grid=0")
    weapon = item_from_template(template, WEAPON_UID, 1, 20000)
    cur.execute(
        "INSERT INTO inventory_backpack (character_name, grid, item_json) VALUES ('bevychar', 0, ?)",
        (json.dumps(weapon, ensure_ascii=False),),
    )
    # 3) 精炼材料格：三属性材料 + 矿石 ⇒ settle 走「应用属性」分支（无材料必碎）
    mats = [None] * MATERIAL_SLOTS
    for i, (item_index, dura) in enumerate(MAT_ITEMS):
        mats[i] = item_from_template(template, MAT_UIDS[i], item_index, dura)
    cur.execute("DELETE FROM refine_log WHERE character_name='bevychar'")
    cur.execute(
        "INSERT INTO refine_log (character_name, materials_json) VALUES ('bevychar', ?)",
        (json.dumps(mats, ensure_ascii=False),),
    )
    # 4) 金币兜底（开始精炼扣 (RequiredAmount*10)*RefineCost）
    cur.execute(
        "UPDATE characters SET gold=MAX(gold, 1000000) WHERE name='bevychar'"
    )
    con.commit()
    con.close()

    print(
        "refine prep-db: bevychar@%d(%d,%d) weapon uid=%d, %d material slots"
        % (REFINE_MAP, REFINE_X, REFINE_Y, WEAPON_UID, len(MAT_ITEMS))
    )
    return 0


def write_config(server_toml: str, out_toml: str) -> int:
    """生成 e2e 专用配置（不改仓库里的 server.toml）。"""
    with open(server_toml, "r", encoding="utf-8", newline="") as f:
        toml = f.read()
    section = ""
    out_lines = []
    for line in re.split(r"(\r?\n)", toml):
        stripped = line.strip()
        if stripped.startswith("[") and stripped.endswith("]"):
            section = stripped.strip("[]")
        if section == "refine":
            if stripped.startswith("base_chance"):
                line = re.sub(r"=.*$", "= 100", line)
            elif stripped.startswith("time_minutes"):
                line = re.sub(r"=.*$", "= 0", line)
        out_lines.append(line)
    with open(out_toml, "w", encoding="utf-8", newline="") as f:
        f.write("".join(out_lines))

    print(f"refine prep-config: {out_toml} ([refine] base_chance=100 time_minutes=0)")
    return 0


def restore_db(db_path: str) -> int:
    """还原到「其余用例」的默认状态：位置 + 清精炼状态 + 移除本次的测试武器。"""
    if not os.path.exists(db_path):
        print(f"refine restore: db not found: {db_path}")
        return 1
    wal_checkpoint(db_path)
    con = sqlite3.connect(db_path)
    cur = con.cursor()
    cur.execute(
        "UPDATE characters SET map_index=?, x=?, y=?, direction=? WHERE name='bevychar'",
        (BACK_MAP, BACK_X, BACK_Y, BACK_DIR),
    )
    cur.execute("DELETE FROM refine_log WHERE character_name='bevychar'")
    cur.execute(
        "DELETE FROM inventory_backpack WHERE character_name='bevychar' AND grid=0 "
        "AND item_json LIKE ?",
        (f'%"unique_id": {WEAPON_UID}%',),
    )
    cur.execute(
        "DELETE FROM inventory_backpack WHERE character_name='bevychar' AND grid=0 "
        "AND item_json LIKE ?",
        (f'%"unique_id":{WEAPON_UID}%',),
    )
    con.commit()
    con.close()
    print(
        "refine restore: bevychar@%d(%d,%d), refine_log cleared, test weapon removed"
        % (BACK_MAP, BACK_X, BACK_Y)
    )
    return 0


def main(argv: list[str]) -> int:
    if len(argv) >= 4 and argv[1] == "config":
        return write_config(argv[2], argv[3])
    if len(argv) >= 3 and argv[1] == "prepare-db":
        return prepare_db(argv[2])
    if len(argv) >= 3 and argv[1] == "restore-db":
        return restore_db(argv[2])
    if len(argv) >= 5 and argv[1] == "prepare":
        rc = write_config(argv[3], argv[4])
        return rc if rc != 0 else prepare_db(argv[2])
    print(__doc__)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv))
