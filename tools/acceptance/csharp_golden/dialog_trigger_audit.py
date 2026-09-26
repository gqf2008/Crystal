# dialog_trigger_audit.py — 「窗口存在 ≠ 功能存在」审计：状态驱动窗的**开字段必须有人写 true**
#
# 为什么单做这一条：2026-09-27 逐窗核 `chat_notice` 时发现——窗、面板、RPC 开窗、自检夹具都齐，
# 但 `ChatNoticeState.visible` **全仓没有任何 `= true` 写入方**（只有计时归零时写 false）⇒
# 顶部公告横幅在实机上永远不会出现。这类「只有壳、没接线」的缺陷，尺寸审计/几何对表/交互门禁
# 都看不见（它们只检查"窗能开、控件几何对不对"），只有查「谁把它打开」才看得见。
#
# 判据（启发式，输出必须人工过一眼）：
#   1. 找 `pub struct <X>State` 里**开语义的 bool 字段**（`visible/open/managing/shown/composing`）；
#   2. 该字段必须至少有一个 `.<field> = true` 写入方，来源限三种：
#      a. `impl <X>State` 内的 `self.<field> = true`（状态自带 `show()/open()`）；
#      b. 参数里取到该 state 的 `ResMut<… XState>` / `&mut XState` 的函数体；
#      c. 提到该 state 类型的文件里（兜住"局部变量由类型推断"的写法，如 `npc.visible = true`）。
#      三者都找不到 → 报一行。
#   3. 清单 `dialog_trigger_audit_known.txt` 内的命中只提示、不判红；**新增命中才 FAIL**。
#
# 用法：
#   py -3.12 dialog_trigger_audit.py --repo <Rust 仓库根> [--known <文件>]
#   py -3.12 dialog_trigger_audit.py --repo <仓库根> --selftest     # 正/负对照自证
#   py -3.12 dialog_trigger_audit.py --repo <仓库根> --openers      # 附：每个 DialogKind 的玩法侧打开方（**报表，不判红**）
#
# `--openers` 为什么与门禁分开：状态驱动窗（`Storage`/`InputBox`/`HeroManage`/`ChatNotice`…）不走
# `mgr.open`，只看"有没有 `mgr.open(Kind)`"会把它们全判成"没入口"。这张表给人看，不当门禁。
import argparse
import os
import re
import shutil
import sys
import tempfile

SUBDIR = os.path.join("Client-Bevy", "src")
# 开语义字段：窗"被打开"的那一位
OPEN_FIELDS = ("visible", "open", "managing", "shown", "composing")
# `--openers` 里不算"玩法侧入口"的文件（RPC 探针 / 自检夹具 / 热键表 / 窗口枚举与交互护栏）
OPENER_EXCLUDE = ("control.rs", "auto" + os.sep, "keyboard_nav.rs", "interact_gate.rs",
                  "dialogs" + os.sep + "mod.rs")
# **不算触发**的文件：`control.rs` 是 RPC 探针（"只有探针能开"本身就是本条要抓的反模式）、
# `auto/*` 是自检夹具。二者都不能当成"玩家能打开这个窗"。
WRITER_EXCLUDE = ("control.rs", "auto" + os.sep)


def _close(text, i, open_ch, close_ch):
    depth = 0
    while i < len(text):
        if text[i] == open_ch:
            depth += 1
        elif text[i] == close_ch:
            depth -= 1
            if depth == 0:
                return i
        i += 1
    return len(text) - 1


def _rust_files(root):
    for dirpath, _dirs, names in os.walk(root):
        for n in sorted(names):
            if n.endswith(".rs"):
                yield os.path.join(dirpath, n)


def collect_states(texts):
    """{state: (owner_rel, [open_fields])}"""
    states = {}
    for p, t in texts.items():
        for m in re.finditer(r"pub struct (\w+State)\b[^;{]*\{", t):
            name = m.group(1)
            end = _close(t, m.end() - 1, "{", "}")
            body = t[m.end():end]
            fields = [f.group(1) for f in re.finditer(r"pub\s+(\w+)\s*:\s*bool\b", body)]
            fields = [f for f in fields if f in OPEN_FIELDS]
            if fields:
                states[name] = (p, fields)
    return states


def has_true_writer(state, field, texts):
    """三种来源里找 `.<field> = true`。返回命中的文件相对路径列表。"""
    hits = []
    decl = re.compile(r"\b" + state + r"\b")
    assign = re.compile(r"\.\s*" + field + r"\s*=\s*true\b")
    for p, t in texts.items():
        if not decl.search(t):
            continue
        rel = _rel(p)
        probe_only = any(x in rel for x in WRITER_EXCLUDE)
        # a) impl <State> { ... self.<field> = true ... }
        for im in re.finditer(r"impl\s+" + state + r"\b[^{]*\{", t):
            end = _close(t, im.end() - 1, "{", "}")
            if assign.search(t[im.end():end]):
                hits.append(p)
                break
        else:
            # b) 取到该 state 可变引用的函数体
            found = False
            if not probe_only:
                for fm in re.finditer(r"fn\s+\w+\s*(?:<[^>]*>)?\s*\(", t):
                    ps = fm.end() - 1
                    pe = _close(t, ps, "(", ")")
                    params = t[ps + 1:pe]
                    if not re.search(r"(?:ResMut\s*<[^>]*\b" + state + r"\b|&mut\s+" + state + r"\b)", params):
                        continue
                    body_start = t.find("{", pe)
                    if body_start < 0:
                        continue
                    body_end = _close(t, body_start, "{", "}")
                    if assign.search(t[body_start:body_end]):
                        found = True
                        break
                if found:
                    hits.append(p)
                # c) 兜底：该文件提到过这个 state 类型，且有 `.<field> = true`（局部变量/推断类型）
                elif assign.search(t):
                    hits.append(p)
    return hits


def scan(root, subdir):
    texts = {}
    for p in _rust_files(os.path.join(root, subdir)):
        with open(p, encoding="utf-8", errors="replace") as fh:
            texts[p] = fh.read()
    states = collect_states(texts)
    rows = []
    for state, (owner, fields) in sorted(states.items()):
        for field in fields:
            hits = has_true_writer(state, field, texts)
            if not hits:
                rows.append({
                    "state": state,
                    "field": field,
                    "file": _rel(owner),
                })
    return rows


def _rel(path):
    marker = os.sep + os.path.normpath(SUBDIR) + os.sep
    i = path.find(marker)
    return path[i + 1:] if i >= 0 else os.path.relpath(path)


def load_known(path):
    known = set()
    if path and os.path.exists(path):
        for line in open(path, encoding="utf-8"):
            line = line.split("#", 1)[0].strip()
            if line:
                parts = line.split()
                if len(parts) >= 2:
                    known.add((parts[0], parts[1]))
    return known


def openers_report(root, subdir):
    texts = {}
    for p in _rust_files(os.path.join(root, subdir)):
        with open(p, encoding="utf-8", errors="replace") as fh:
            texts[p] = fh.read()
    kinds = sorted({k for t in texts.values() for k in re.findall(r"DialogKind::(\w+)", t)})
    print("DialogKind → 玩法侧入口文件数（已排除 RPC 探针 / 自检夹具 / 热键表 / 交互护栏；报表不判红）")
    for k in kinds:
        files = []
        for p, t in texts.items():
            rel = _rel(p)
            if any(x in rel for x in OPENER_EXCLUDE):
                continue
            if re.search(r"DialogKind::" + k + r"\b", t) and re.search(
                r"mgr\s*\.\s*(?:open|toggle)\s*\(\s*DialogKind::" + k, t
            ):
                files.append(rel)
        flag = "   ← 无 mgr.open/toggle 入口（状态驱动窗属正常）" if not files else ""
        print(f"  {k:<18} {len(files)}  {files[:2]}{flag}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--repo", required=True, help="Rust 仓库根（含 Client-Bevy/src）")
    ap.add_argument("--subdir", default=SUBDIR)
    ap.add_argument("--known", default="", help="已知待核清单（`state<TAB>field`，`#` 注释）")
    ap.add_argument("--selftest", action="store_true", help="正/负对照：去掉触发必须报出来")
    ap.add_argument("--openers", action="store_true", help="附：每个 DialogKind 的玩法侧入口（报表）")
    a = ap.parse_args()
    if a.selftest:
        return selftest(a)
    if a.openers:
        openers_report(a.repo, a.subdir)
    known_path = a.known or os.path.join(os.path.dirname(os.path.abspath(__file__)),
                                         "dialog_trigger_audit_known.txt")
    known = load_known(known_path)
    rows = scan(a.repo, a.subdir)
    new_rows = [r for r in rows if (r["state"], r["field"]) not in known]
    for r in rows:
        mark = "已知待核" if (r["state"], r["field"]) in known else "**新增**"
        print(f"  [{mark}] {r['state']}.{r['field']}  @{r['file']}")
    print(f"合计 {len(rows)} 处「开字段没有任何 `= true` 写入方」，其中已知 {len(rows) - len(new_rows)}、"
          f"**新增 {len(new_rows)}**")
    if new_rows:
        print("VERDICT=FAIL：这些窗/字段没有任何地方把它打开——要么补真实触发（网络事件/按钮/热键），"
              "要么确认是死字段并从状态里删掉，再登记进 known 表并写明理由")
        return 1
    print("VERDICT=PASS：所有开语义字段都有 `= true` 写入方（或已在 known 表登记理由）")
    return 0


def selftest(a):
    """正/负对照：① 原样扫必须 0 新增；② 把 `chat_notice` 的触发摘掉（`show()` 不再置 visible）
    必须报出 `ChatNoticeState.visible`。这两条分别证明「不误报」与「真的会红」。"""
    ok = True
    known_path = a.known or os.path.join(os.path.dirname(os.path.abspath(__file__)),
                                         "dialog_trigger_audit_known.txt")
    known = load_known(known_path)
    neg = [r for r in scan(a.repo, a.subdir) if (r["state"], r["field"]) not in known]
    print(f"[负对照] 原样扫描的新增命中 {len(neg)}（期望 0；known {len(known)} 条）")
    ok &= not neg
    with tempfile.TemporaryDirectory() as tmp:
        dst = os.path.join(tmp, a.subdir)
        shutil.copytree(os.path.join(a.repo, a.subdir), dst)
        target = os.path.join(dst, "game", "dialogs", "chat_notice.rs")
        text = open(target, encoding="utf-8").read()
        anchor = "        self.text = text.into();\n        self.visible = true;\n"
        if anchor not in text:
            print("[正对照] 找不到锚点（`ChatNoticeState::show` 里的 `self.visible = true;`）——先更新 selftest")
            return 1
        open(target, "w", encoding="utf-8").write(text.replace(anchor, "        self.text = text.into();\n"))
        pos = [r for r in scan(tmp, a.subdir) if (r["state"], r["field"]) not in known]
        hit = [r for r in pos if r["state"] == "ChatNoticeState"]
        print(f"[正对照] 摘掉横幅触发后命中 {len(pos)} 条（期望 ≥1），其中 ChatNoticeState "
              f"{len(hit)} 条：{[(r['state'], r['field']) for r in pos][:4]}")
        ok &= len(hit) >= 1
    print("VERDICT=" + ("PASS" if ok else "FAIL") + "（正/负对照）")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
