# window_rect_table.py — 「逐窗几何对表」：从原版 C# 常量推出每个窗口的**期望矩形**，再与本站实机矩形比
#
# 为什么需要它（2026-09-26）：原版客户端的**点击通道不可用**（见 README §3.1：注入消息不产生
# `MouseControl`、真实点击也点不动页签/关闭钮），于是"让某个窗口出现再截图对拍"对多数窗口做不到。
# 但窗口**几何**是纯常量：原版每扇窗的背景图 = `Library` + `Index`，位置 = `Location`，
# 尺寸 = 背景图原生尺寸（或显式 `Size`）。把这三样从 C# 里读出来，就是可比的期望矩形。
#
# 本工具做两件事：
#   1. `--src <repo>`：扫 `Client/MirScenes/Dialogs/*.cs`，输出每扇窗的 (lib, index, x, y, w, h)
#      （尺寸取自**同一套 .Lib** 的图头，不是我方数值，避免"以我方为准"的循环论证）；
#   2. `--compare <probe.json>`：读 `probe_ui_nodes.ps1` 产出的 JSON（含 `rect_<kind>`），
#      按下面的映射逐窗打「期望 vs 实际 vs 差值」表。
#
# 注意：这是**几何**对表，不是像素对拍；它能抓到"窗口开在错位置/尺寸不对"，抓不到窗内控件错位
# （那种要用 `ui_nodes_at` 逐点，或像素 A/B）。
import argparse
import json
import os
import re
import struct
import sys

# C# 对话框类 → 本站 DialogKind（probe JSON 里的 rect_<kind> 键）
CLASS_TO_KIND = {
    "InventoryDialog": "inventory",
    "CharacterDialog": "character",
    # `quest_log` 对应原版的 **QuestDiaryDialog**（Prguse[961] @ (ScreenWidth/2-300-20, 60) = (192,60)），
    # 不是 QuestListDialog（Prguse[950] @ (NPCDialog.Width+47, 0) = (487,0)，那是"贴在 NPC 窗右边"的
    # 任务列表变体）。两窗美术同尺寸 316x466，只有位置能区分——实测我方 (192,60) 与 Diary 一致。
    "QuestDiaryDialog": "quest_log",
    "QuestDialog": "quest_log",
    "MenuDialog": "menu",
    "OptionDialog": "settings",
    "GroupDialog": "group",
    "FriendDialog": "friend",
    "TradeDialog": "trade",
    "GuildDialog": "guild",
    "MiniMapDialog": "minimap",
    "RankingDialog": "ranking",
    "MailListDialog": "mail",
    "MailComposeDialog": "mail_compose",
    "MentorDialog": "mentor",
    "RelationshipDialog": "relationship",
    "MountDialog": "mount",
    "GameShopDialog": "game_shop",
    "GameshopDialog": "game_shop",
    "HeroInventoryDialog": "hero_inventory",
    # 其余 sweep 名单里的窗口（按原版类名 ↔ RPC kind 一一对应）
    "IntelligentCreatureDialog": "creature",
    "GuildTerritoryDialog": "guild_territory",
    "HelpDialog": "help",
    "NoticeDialog": "notice",
    "FishingDialog": "fishing",
    "CraftDialog": "craft",
    "NPCAwakeDialog": "npc_awake",
    "KeyboardLayoutDialog": "keyboard_layout",
    "BigMapDialog": "big_map",
    "TrustMerchantDialog": "market",
    "StorageDialog": "storage",
    "SocketDialog": "socket",
    "QuestDetailDialog": "quest_detail",
    "MailComposeLetterDialog": "mail_compose",
    # 两个"租借"窗按**尺寸**区分：400x174 的 ItemRentalDialog 是我方的 `item_rental_browse`，
    # 204x109 的 ItemRentingDialog 才是 `item_rental`（实测我方 item_rental=(718,287,204,109)
    # = C# `ItemRentingDialog` 的 (ScreenWidth-W-W/2, H*2+H/2+15)）。
    "ItemRentalDialog": "item_rental_browse",
    "ItemRentingDialog": "item_rental",
}

# 位置由**调用点动态锚定**的窗口：构造器里写的 (0,0) 不是真值，真值在 Show()/打开路径里算。
# 期望值按「背包在 (0,0)、尺寸 316x236」求值——`probe_ui_nodes.ps1` 正是先开背包再逐窗开，
# 所以这个假设在探针里是确定的（换了别的前置就得改这里）。
DYNAMIC_ANCHOR = {
    # NPCDialogs.cs:2450-2452 `CraftDialog.Show()`：Location = (InventoryDialog.X - 12, Y + 236)
    "CraftDialog": lambda w, h: (-12.0, 236.0),
    # SocketDialog.cs:107-110：x = bag.X + (bag.W - w)/2, y = bag.Y + bag.H + 5（背包 316x236 ⇒ (117.5, 241)）
    # 注意 C# 是**整数除法**：(316-81)/2 = 117（不是 117.5 四舍五入成 118）
    "SocketDialog": lambda w, h: (int((316 - w) // 2), 236 + 5),
}

# 面板随**状态**换图的窗口：期望尺寸有两个合法值，比对时任一命中即算 OK（别当成缺陷）
STATE_DEPENDENT = {
    "MountDialog": [(324, 377), (272, 378)],  # Prguse[167] 五孔 / Prguse[160] 四孔（C# SwitchType）
}


def lib_header_size(path, index):
    """只读图头拿 (w,h)：`i16 w, i16 h, …` @ offset（与 libextract.py 同格式）。"""
    with open(path, "rb") as f:
        data = f.read()
    version, count = struct.unpack_from("<ii", data, 0)
    if version < 2:
        return None
    base = 8 + (4 if version >= 3 else 0)
    if index < 0 or index >= count:
        return None
    (offset,) = struct.unpack_from("<i", data, base + index * 4)
    w, h = struct.unpack_from("<hh", data, offset)
    return (w, h)


def class_blocks(text):
    """粗切类块：`class X : Base` 到配对大括号结束（C# 无嵌套类时的常用写法够用）。"""
    out = []
    for m in re.finditer(r"\b(?:sealed\s+|public\s+|internal\s+)*class\s+(\w+)\s*:\s*([\w\.]+)", text):
        name, base = m.group(1), m.group(2)
        i = text.find("{", m.end())
        if i < 0:
            continue
        depth = 0
        for j in range(i, len(text)):
            if text[j] == "{":
                depth += 1
            elif text[j] == "}":
                depth -= 1
                if depth == 0:
                    out.append((name, base, i + 1, j, m.start()))
                    break
    return out


def first_assign(body, pattern):
    m = re.search(pattern, body)
    if not m:
        return None
    return m


def own_prefix(head):
    """截到「对话框自己那几行属性赋值」为止。

    原版风格是：构造器先给**自己**赋 `Index/Library/Location/Size/Visible/...`，之后才开始
    `WeightBar = new MirImageControl{...}` 这类**子控件**赋值。子控件里也有 `Location`/`Size`，
    直接取"第一个出现的"会把子控件的位置当成窗口位置（实测：InventoryDialog 被读成
    (182,217) 72x23 —— 那是 WeightBar 和 ItemButton 的值）。所以先切到第一个
    `xxx = new Mir*` / `xxx = new Client.*` 之前。
    """
    # 对话框自己的属性赋值里只有 `new Point(...)` / `new Size(...)`；**任何其它 `= new X`**
    # 都是开始建子控件（`WeightBar = new MirImageControl{…}`、`Grid[(int)EquipmentSlot.Weapon]
    # = new MirItemCell{…}`、`Rows[i] = new RankingRow{…}` …）。按"第一个非 Point/Size 的 new"
    # 切边界 —— 只按行首 `^\s*\w+\s*=` 切会漏掉数组下标左值那种写法，子控件的位置就被当成窗口位置
    # （实测：BigMapDialog 被读成 (132,134)、HelpDialog (244,129)、FishingDialog (412,240)）。
    for m in re.finditer(r"=\s*new\s+([A-Za-z_][\w\.]*)", head):
        if m.group(1) not in ("Point", "Size"):
            return head[:m.start()]
    return head


def find_point_ctor(text):
    """找 `Location = new Point(A, B)` 并返回 (A, B)。

    用**配平括号**扫描而不是正则：参数里常带括号（如
    `new Point((Settings.ScreenWidth - Size.Width) / 2, (Settings.ScreenHeight - Size.Height) / 2)`，
    OptionDialog 就是这一行），`[^\\)]+` 这类正则会停在**内层**括号，整个赋值就漏掉了
    （实测：OptionDialog 被读成 (0,0)，与"居中"的真相正好相反）。
    """
    m = re.search(r"\bLocation\s*=\s*new\s+Point\s*\(", text)
    if not m:
        return None
    i = m.end()
    depth = 1
    while i < len(text) and depth:
        if text[i] == "(":
            depth += 1
        elif text[i] == ")":
            depth -= 1
        i += 1
    inside = text[m.end():i - 1]
    depth = 0
    for k, ch in enumerate(inside):
        if ch == "(":
            depth += 1
        elif ch == ")":
            depth -= 1
        elif ch == "," and depth == 0:
            return inside[:k], inside[k + 1:]
    return None


def scan(src):
    dialogs = os.path.join(src, "Client", "MirScenes", "Dialogs")
    rows = []
    for fn in sorted(os.listdir(dialogs)):
        if not fn.endswith(".cs"):
            continue
        path = os.path.join(dialogs, fn)
        text = open(path, encoding="utf-8", errors="replace").read()
        # **必须先去掉注释行**：RankingDialog 里 `//Size = new Size(288, 324);` 是注释掉的历史值
        # （真值 = 背景图原生 324x441）。不去注释会把井号里的值当真值，直接给出一条假 DIFF
        # （实测：ranking 被读成 288x324，与我方 324x441 判成不一致）。
        text = re.sub(r"(?m)//.*$", "", text)
        text = re.sub(r"/\*.*?\*/", "", text, flags=re.S)
        for name, base, start, end, _ls in class_blocks(text):
            if "Dialog" not in name:
                continue
            body = text[start:end]
            # 只看构造器区间：从 `public XxxDialog(` 起 200 行内（背景/位置都在那儿设）
            cm = re.search(r"public\s+%s\s*\(" % re.escape(name), body)
            head = body[cm.start():] if cm else body
            head = head[:12000]
            own = own_prefix(head)
            mi = first_assign(own, r"\bIndex\s*=\s*(\d+)\s*;")
            ml = first_assign(own, r"\bLibrary\s*=\s*(?:Libraries\.)?(\w+)\s*;")
            # 位置可以是字面量，也可以是 `Settings.ScreenWidth - 264` 这类表达式
            # （CharacterDialog 就是 `ScreenWidth - 264` ⇒ 1024-264 = 760）。按原版分辨率
            # 1024x768（金标准客户端 Mir2Config 的 Resolution）求值；解不出的记 ? 而不是 0。
            mloc_parts = find_point_ctor(own)
            # 原版大量对话框写 `Location = Center;`（MirControl.Center = ((ScreenWidth-Size.Width)/2,
            # (ScreenHeight-Size.Height)/2)，MirControl.cs:643）——这不是"没有位置"，而是**居中**。
            # 不认它就会把一群居中窗判成 DIFF（实测 8 个）。
            mcenter = first_assign(own, r"\bLocation\s*=\s*(Center|Left|Top|Right|Bottom)\s*;")
            msize = first_assign(own, r"\bSize\s*=\s*new\s+Size\s*\(\s*(-?\d+)\s*,\s*(-?\d+)\s*\)")
            if not (mi and ml):
                continue
            def eval_expr(expr):
                e = expr.strip()
                e = e.replace("Settings.ScreenWidth", "1024").replace("Settings.ScreenHeight", "768")
                e = re.sub(r"\bMath\.Max\b|\bMath\.Min\b", "", e)
                if not re.fullmatch(r"[\d\s\+\-\*/\(\)]+", e):
                    return None
                try:
                    return int(eval(e))  # noqa: S307 —— 只允许数字与四则运算（上面已白名单过滤）
                except Exception:
                    return None

            lx = eval_expr(mloc_parts[0]) if mloc_parts else 0
            ly = eval_expr(mloc_parts[1]) if mloc_parts else 0
            rows.append({
                "file": fn,
                "class": name,
                "base": base,
                "library": ml.group(1),
                "index": int(mi.group(1)),
                "x": lx,
                "y": ly,
                "loc_expr": [mloc_parts[0].strip(), mloc_parts[1].strip()] if mloc_parts else None,
                "loc_symbol": mcenter.group(1) if mcenter else None,
                "declared_size": [int(msize.group(1)), int(msize.group(2))] if msize else None,
            })
    return rows


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--src", required=True, help="原版仓库根（含 Client/MirScenes/Dialogs）")
    ap.add_argument("--data", default="Data", help="含 *.Lib 的数据目录")
    ap.add_argument("--out", default="", help="把期望表写成 JSON")
    ap.add_argument("--compare", default="", help="probe_ui_nodes.ps1 产出的 JSON，做逐窗对比")
    a = ap.parse_args()

    rows = scan(a.src)
    # 先补 size（居中位置要用到 Size），再解位置
    for r in rows:
        libfile = os.path.join(a.data, r["library"] + ".Lib")
        size = lib_header_size(libfile, r["index"]) if os.path.exists(libfile) else None
        r["art"] = list(size) if size else None
        r["expect"] = r["declared_size"] or r["art"]

    def eval_expr(expr, size):
        e = expr.strip()
        e = e.replace("Settings.ScreenWidth", "1024").replace("Settings.ScreenHeight", "768")
        e = e.replace("GameScene.Scene.NPCDialog.Size.Width", "440")  # NPCDialog Prguse[995] = 440x224
        e = e.replace("Size.Width", str(size[0]) if size else "0")
        e = e.replace("Size.Height", str(size[1]) if size else "0")
        e = re.sub(r"[A-Za-z_][\w\.]*", "0", e)  # 其余符号名一律 0（解不出的会被下面白名单挡掉）
        if not re.fullmatch(r"[\d\s\+\-\*/\(\)]+", e):
            return None
        try:
            return int(eval(e))  # noqa: S307 —— 只留数字与四则运算
        except Exception:
            return None

    for r in rows:
        w, h = (r["expect"] or [0, 0])
        # 扫描阶段的表达式求值不认识 `Size.Width`（那时还没解析出尺寸）——这里用最终尺寸再补一次
        if r.get("loc_expr") and (r["x"] is None or r["y"] is None):
            r["x"] = eval_expr(r["loc_expr"][0], r["expect"])
            r["y"] = eval_expr(r["loc_expr"][1], r["expect"])
        sym = r.get("loc_symbol")
        if sym == "Center":
            r["x"], r["y"] = (1024 - w) // 2, (768 - h) // 2
        elif sym == "Left":
            r["x"], r["y"] = 0, (768 - h) // 2
        elif sym == "Top":
            r["x"], r["y"] = (1024 - w) // 2, 0
        elif sym == "Right":
            r["x"], r["y"] = 1024 - w, (768 - h) // 2
        elif sym == "Bottom":
            r["x"], r["y"] = (1024 - w) // 2, 768 - h
        if r["class"] in DYNAMIC_ANCHOR:
            dx, dy = DYNAMIC_ANCHOR[r["class"]](w, h)
            r["x"], r["y"] = int(round(dx)), int(round(dy))
        r["kind"] = CLASS_TO_KIND.get(r["class"])
    if a.out:
        with open(a.out, "w", encoding="utf-8") as f:
            json.dump(rows, f, ensure_ascii=False, indent=2)
        print("wrote", a.out, len(rows), "rows")

    if not a.compare:
        print(f"{'class':28s} {'lib':10s} {'idx':>5s} {'loc':>10s} {'declared':>10s} {'art':>10s} {'kind':12s} file")
        for r in rows:
            print("%-28s %-10s %5d %10s %10s %10s %-12s %s" % (
                r["class"], r["library"], r["index"], f"{r['x']},{r['y']}",
                str(r["declared_size"] or "-"), str(r["art"] or "-"), r["kind"] or "-", r["file"]))
        return 0

    probe = json.load(open(a.compare, encoding="utf-8"))
    print(f"{'kind':16s} {'class':26s} {'期望(x,y,w,h)':>20s} {'实际(x,y,w,h)':>20s}  判定")
    bad = 0
    skipped = 0
    seen = set()
    for r in rows:
        kind = r["kind"]
        if not kind or kind in seen or not r["expect"]:
            continue
        seen.add(kind)
        act = probe.get(f"rect_{kind}") or {}
        if not act.get("ok"):
            skipped += 1
            print(f"{kind:16s} {r['class']:26s} {str(tuple(r['expect'])):>20s} {str(act.get('error') or '(未取到)'):>20s}  SKIP")
            continue
        if r["x"] is None or r["y"] is None:
            print(f"{kind:16s} {r['class']:26s} {'位置是表达式，未能求值':>20s} {str(r.get('loc_expr')):>20s}  SKIP")
            skipped += 1
            continue
        exp_rect = (r["x"], r["y"], r["expect"][0], r["expect"][1])
        act_rect = (round(act.get("rx", -1)), round(act.get("ry", -1)),
                    round(act.get("rw", -1)), round(act.get("rh", -1)))
        # 状态相关的面板：任一合法尺寸命中即算一致（尺寸不同但位置相同）
        alt = STATE_DEPENDENT.get(r["class"], [])
        same = exp_rect == act_rect or (
            act_rect[0] == r["x"] and act_rect[1] == r["y"] and (act_rect[2], act_rect[3]) in alt)
        bad += 0 if same else 1
        tag = "OK" if same else "DIFF"
        if same and exp_rect != act_rect:
            tag = "OK(状态换图)"
        print(f"{kind:16s} {r['class']:26s} {str(exp_rect):>20s} {str(act_rect):>20s}  {tag}")
    print(f"不一致：{bad}；跳过（未取到/表达式）：{skipped}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
