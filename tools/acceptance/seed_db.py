#!/usr/bin/env python3
"""给测试角色塞 UI 验证数据（行会成员/职务/任务/宠物）。

服务器必须先停：行会是在启动时整体载入缓存的，跑着的时候写库会被存档覆盖。
用法: seed_db.py [<char> <guild>]
"""
import json
import sqlite3
import sys

DB = "ServerRust/Data/crystal.db"
CHAR = sys.argv[1] if len(sys.argv) > 1 else "bevychar"
GUILD = sys.argv[2] if len(sys.argv) > 2 else "TestGuild2"

RANK_DEFS = [
    {"index": 0, "name": "会长", "options": 255},
    {"index": 1, "name": "副会长", "options": 127},
    {"index": 2, "name": "成员", "options": 2},
]
# 取名要能一眼看出是造的：Seed 前缀 + 两位序号
NAMES = ["会长本人"] + [f"副会长{i:02d}" for i in range(1, 3)] + [
    f"成员Seed{i:02d}" for i in range(1, 25)
]

c = sqlite3.connect(DB)

# --- 1. 行会成员：1 会长 + 2 副会长 + 24 成员 = 27 行（> 18 行视窗 → 可滚动）---
c.execute("DELETE FROM guild_members WHERE guild_name = ?", (GUILD,))
rows = [(GUILD, CHAR, 0, 0, 0)]
for i, n in enumerate(NAMES):
    rank = 0 if n == "会长本人" else (1 if n.startswith("副会长") else 2)
    # last_login_ms：0 = 从未上线（离线）；非 0 = 在线/有登录记录
    rows.append((GUILD, n, rank, rank if rank < 2 else 2, 0 if rank == 2 else 1_700_000_000_000))
c.executemany(
    "INSERT OR REPLACE INTO guild_members (guild_name, member_name, rank, rank_index, last_login_ms)"
    " VALUES (?,?,?,?,?)",
    rows,
)

# --- 2. 职务定义（成员行下拉的选项来源）---
c.execute(
    "UPDATE guilds SET rank_defs_json = ?, notice_json = ?, gold = ? WHERE name = ?",
    (
        json.dumps(RANK_DEFS, ensure_ascii=False),
        json.dumps(
            [
                "欢迎来到测试行会！",
                "本行会仅用于 UI 验证。",
                "公告第二段用来撑出滚动条。",
                "", "", "", "", "", "", "", "", "", "",
                "第九行",
            ],
            ensure_ascii=False,
        ),
        123_456,
        GUILD,
    ),
)

# --- 3. 已接任务（任务日记的"已接"段 + 组头）---
c.execute("DELETE FROM quests WHERE character_name = ?", (CHAR,))
qrows = [
    (142, "Boar Tooth", "InProgress"),
    (27, "Errands", "InProgress"),
    (46, "Skeleton Bones", "InProgress"),
    (29, "Deliver Repair Oil", "InProgress"),
    (86, "Uniform Order Status", "InProgress"),
]
c.executemany(
    "INSERT INTO quests (character_name, quest_index, title, status, progress_json,"
    " exp_reward, gold_reward, credit_reward, start_time, time_limit_seconds, quest_type)"
    " VALUES (?,?,?,?,'[]',1000,500,0,1700000000,0,0)",
    [(CHAR, qi, ti, st) for qi, ti, st in qrows],
)

# --- 4. 宠物（宠物面板：图标格 + 名字 + 三行信息）---
# 注意：`CreatureType`/`PickupMode` 是带 derive(Serialize/Deserialize) 的**无字段枚举**，
# serde 默认按**变体名字符串**表示（不是数字）。写数字会被 `from_str().unwrap_or_default()`
# 静默吞成空表，然后下一次自动存档把空表写回去——宠物凭空消失。
# 只用 C# 静态表里**真有**图标/规则的 5 种（Panda/Oma/Sheep/Gorilla 在原版表里没有对应，
# 客户端按设计跳过图标绘制，不是缺陷）
TYPES = ["BabyPig", "BabyChicken", "BabyKitten", "BabySkeleton", "BabyBabyDragon"]
owned = [
    {
        "creature_type": t,
        "custom_name": f"测试宠{i + 1}",
        "pickup_mode": "All",
        "hunger": 50 + i * 5,
        "enabled": True,
    }
    for i, t in enumerate(TYPES)
]
c.execute(
    "INSERT OR REPLACE INTO creatures (character_name, active_type, active_custom_name,"
    " active_pickup_mode, active_hunger, active_enabled, owned_json, request_updates)"
    " VALUES (?,?,?,?,?,?,?,0)",
    (CHAR, 2, "测试宠1", 3, 55, 1, json.dumps(owned, ensure_ascii=False)),
)

c.commit()
print("members:", c.execute("select count(*) from guild_members where guild_name=?", (GUILD,)).fetchone()[0])
print("quests :", c.execute("select count(*) from quests where character_name=?", (CHAR,)).fetchone()[0])
print("creature owned:", len(owned))
print("rank_defs:", c.execute("select rank_defs_json from guilds where name=?", (GUILD,)).fetchone()[0])
c.close()
