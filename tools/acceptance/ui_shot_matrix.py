#!/usr/bin/env python3
"""逐对话框截图矩阵：每个窗开→截图→关，供与 C# 逐功能比对。

用法: ui_shot_matrix.py [outdir]
前提：客户端已起、已在游戏内（本脚本只驱动 control RPC，不自起客户端）。
"""
import json
import os
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from rpc import rpc

KINDS = ("inventory character quest_log settings menu game_shop minimap npc group friend trade "
         "inspect npc_goods guild mail ranking mentor relationship mount report hero_inventory "
         "hero_equipment hero_skill creature trust_merchant item_rental guild_territory help notice "
         "buff fishing socket refine craft dura_status npc_drop roll npc_awake timer keyboard_layout "
         "big_map chat_notice market storage item_rental_browse hero_manage quest_detail input_box").split()

OUT = sys.argv[1] if len(sys.argv) > 1 else "E:/Users/gxh/Documents/GitHub/Crystal/tools/acceptance/shots/matrix"
os.makedirs(OUT, exist_ok=True)

def shot(name):
    p = f"{OUT}/{name}.png"
    rpc("screenshot", {"path": p})
    time.sleep(0.9)
    return p

def rect(kind):
    r = rpc("dialog_rect", {"kind": kind}).get("result", {})
    return r if r.get("ok") else None

rows = []
for k in KINDS:
    try:
        rpc("dialog", {"kind": k, "action": "open"})
    except Exception as e:
        rows.append({"kind": k, "open": "rpc-fail", "err": str(e)}); continue
    time.sleep(0.9)
    d = rpc("dialogs").get("result", {}).get("dialogs", [])
    opened = any(k.replace("_", "").lower() in x.replace("_", "").lower() for x in d)
    rc = rect(k)
    p = shot(k) if opened else None
    rows.append({"kind": k, "open": opened, "dialogs": d, "rect": rc, "shot": p})
    if opened:
        rpc("dialog", {"kind": k, "action": "close"})
        time.sleep(0.5)

json.dump(rows, open(f"{OUT}/manifest.json", "w", encoding="utf-8"), ensure_ascii=False, indent=1)
ok = sum(1 for r in rows if r.get("open"))
print(f"截图 {ok}/{len(KINDS)} 个窗；manifest: {OUT}/manifest.json")
for r in rows:
    if not r.get("open"):
        print("  未打开:", r["kind"], r.get("err", ""))
