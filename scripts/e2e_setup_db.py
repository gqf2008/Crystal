#!/usr/bin/env python3
"""Crystal 真实服 E2E 测试库准备（在 run_real_e2e.ps1 启动服务端前调用）。

背景：#990 怪物 AI 对齐 C# 后，测试地图 BichonProvince 城镇出生点 (288,616)
会被大群怪物围杀（野生怪物对安全区玩家可攻击，C# 语义），导致配对/精炼用例失败。
本脚本：
1. 把测试角色 bevychar/bevy2char 移到远离刷怪点的河边钓鱼点（fishing-test 需要
   前方 3 格为水格 FishingAttribute>=0，C# FishingCast 语义；#1217 数据驱动钓鱼）
2. bevychar 背包为空时恢复 2 件物品（精炼/交易用例需要）
"""
import json
import os
import sqlite3
import sys

DB = os.path.normpath(os.path.join(
    os.path.dirname(os.path.abspath(__file__)), "..", "ServerRust", "data", "crystal.db"))
# map 1（file 0，BichonProvince 700x700）西北角河流边 (170,667)：可走、距最近刷怪点
# 100 格（全图水域扫描实测最安全档），且 bevychar 面向左(6) 时前方 3 格 (167,667)
# 是水格（FishingAttribute>=0，fishing-test 需要；#1217）。其余用例不依赖位置。
SAFE_X, SAFE_Y = 170, 667  # map 1 (BichonProvince) 西北河流钓鱼点


def make_item(template, uid: int, item_index: int) -> str:
    d = json.loads(json.dumps(template))
    d["unique_id"] = uid
    d["item_index"] = item_index
    d["count"] = 1
    d["is_gm_made"] = True
    return json.dumps(d, ensure_ascii=False)


def main() -> int:
    db_path = sys.argv[1] if len(sys.argv) > 1 else DB
    con = sqlite3.connect(db_path)
    cur = con.cursor()
    # 1) 安全点 + 足够金币（gameshop 购买 #1268 需 165000；交易用例会搬金）
    cur.execute(
        "UPDATE characters SET x=?, y=?, gold=1000000 WHERE name IN ('bevychar','bevy2char')",
        (SAFE_X, SAFE_Y),
    )
    # 2) 配对摆位（#1166）：交易邀请要求目标在正前方一格且面对面（C# 语义）。
    #    bevychar（发起方）(170,667) 朝左(6) → 正前方 (169,667)；bevy2char（接受方）(169,667) 朝右(2)。
    #    其余配对用例（组队/私聊/邮件/好友）按名称/在线表，不受相邻影响。
    cur.execute("UPDATE characters SET direction=6 WHERE name='bevychar'")
    cur.execute("UPDATE characters SET x=169, y=667, direction=2 WHERE name='bevy2char'")
    # 2) bevychar 装备恢复（fishing/mount 用例依赖）：
    #    Weapon 槽 = BlueFishingRod(793)、Mount 槽 = BengalTiger(764)
    cur.execute("SELECT COUNT(*) FROM inventory_equipment WHERE character_name='bevychar' AND slot IN (0,10)")
    if cur.fetchone()[0] < 2:
        cur.execute("SELECT item_json FROM inventory_backpack WHERE character_name='bevy2char' AND grid=0 LIMIT 1")
        row = cur.fetchone()
        template = json.loads(row[0]) if row else {}
        def eq_item(uid, item_index, mount=False):
            d = json.loads(json.dumps(template))
            d['unique_id'] = uid; d['item_index'] = item_index; d['count'] = 1
            d['info'] = None  # 服务端 UserInformation enrich
            if mount:
                # 坐骑需 5 孔且 slots[2]=鞍（C# Ride 校验，social.rs RIDE has_saddle）
                saddle = json.loads(json.dumps(d))
                saddle['unique_id'] = uid + 1; saddle['item_index'] = 782; saddle['slots'] = []
                d['slots'] = [None, None, saddle, None, None]
            return json.dumps(d, ensure_ascii=False)
        for slot, uid, idx in ((0, 79301, 793), (10, 76401, 764)):
            cur.execute("SELECT COUNT(*) FROM inventory_equipment WHERE character_name='bevychar' AND slot=?", (slot,))
            if cur.fetchone()[0] == 0:
                cur.execute(
                    "INSERT INTO inventory_equipment (character_name, slot, item_json) VALUES ('bevychar', ?, ?)",
                    (slot, eq_item(uid, idx, mount=(slot == 10))),
                )
    # 3) bevychar 背包为空则恢复（item 194 BraceletOfAgony / 430 BoundlessRing）
    cur.execute("SELECT COUNT(*) FROM inventory_backpack WHERE character_name='bevychar'")
    if cur.fetchone()[0] == 0:
        cur.execute(
            "SELECT item_json FROM inventory_backpack WHERE character_name='bevy2char' AND grid=0 LIMIT 1"
        )
        row = cur.fetchone()
        template = json.loads(row[0]) if row else {
            "unique_id": 1, "item_index": 194, "count": 1,
            "current_dura": 100, "max_dura": 100,
        }
        cur.execute(
            "INSERT INTO inventory_backpack (character_name, grid, item_json) VALUES ('bevychar', 1, ?)",
            (make_item(template, 3001, 194),),
        )
        cur.execute(
            "INSERT INTO inventory_backpack (character_name, grid, item_json) VALUES ('bevychar', 2, ?)",
            (make_item(template, 3002, 430),),
        )
    con.commit()
    cur.execute("SELECT name, x, y FROM characters WHERE name IN ('bevychar','bevy2char')")
    print("E2E db setup:", cur.fetchall())
    con.close()
    return 0


if __name__ == "__main__":
    sys.exit(main())

