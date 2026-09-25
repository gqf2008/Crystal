"""walgit 记账助手：把 JSON 文件当 `--body` 传给 walgit CLI，并给条目链加一道完整性守卫。

为什么需要它（两条都实测踩过）：

1. `walgit collab entry --body` 只收 JSON 字符串。在 PowerShell 里把脚本变量中继给 native 进程时，
   多行字符串会被拆成多个参数（实测：一个「ref oid」两行输出被当成 `--parent` 的值传进去），
   条目照旧签名成功、看板照旧统计 —— **只有 parent 静默变成一句人话，链上看不见父子关系**。
   把 body 落成文件、由 Python 的 argv 传递可稳定落地。
2. `--parent auto` 自动取该线程当前 head（不再手工粘 oid）。
   **发出后还会回读 thread 校验**这条新条目的 parent 确实等于预期值，不等就报错退出 3 ——
   手工粘错 oid / 变量被拆参数这类问题，当场就被抓住，不用等下一轮看板对账。

用法：

    py -3.12 tools/acceptance/walgit_entry.py --kind patch --id crystal-xxx --actor crystal-impl \
        --body-file body.json --key ~/.walgit/keys/crystal-impl.ed25519 --parent auto

    # 门禁：校验某线程的条目链完整（无悬空 parent、单根）；退出码 0 完整 / 1 链断裂 / 2 前置失败
    py -3.12 tools/acceptance/walgit_entry.py --verify --id crystal-xxx
"""
import argparse
import json
import os
import shutil
import subprocess
import sys


def find_walgit(explicit: str = "") -> str:
    cands = [
        explicit,
        os.environ.get("WALGIT_BIN", ""),
        os.path.expandvars(r"%LOCALAPPDATA%\Programs\walgit\walgit.exe"),
        os.path.expanduser("~/walgit/walgit.exe"),
        shutil.which("walgit") or "",
    ]
    for c in cands:
        if c and os.path.exists(c):
            return c
    return ""


def find_config(explicit: str = "") -> str:
    cands = [explicit, os.environ.get("WALGIT_CONFIG", ""), os.path.expanduser("~/.walgit/walgit.toml")]
    for c in cands:
        if c and os.path.exists(c):
            return c
    return ""


def run(argv):
    return subprocess.run(argv, capture_output=True, text=True, encoding="utf-8", errors="replace")


def read_thread(walgit, config, repo, thread_id):
    r = run([walgit, "collab", "--config", config, "thread", thread_id, "--repo", repo])
    if r.returncode != 0:
        raise RuntimeError(f"读线程失败（exit={r.returncode}）：{(r.stderr or r.stdout).strip()[:400]}")
    return json.loads(r.stdout)


def verify_thread(walgit, config, repo, thread_id) -> int:
    """链完整性：每条 parent 要么为空（根）要么指向本线程内已存在的 oid；且只许有一个根。"""
    try:
        entries = read_thread(walgit, config, repo, thread_id)
    except Exception as e:
        print(f"FAIL(前置)：{e}")
        return 2
    if not entries:
        print(f"FAIL(前置)：线程 {thread_id} 没有任何条目")
        return 2
    oids = {e["oid"] for e in entries}
    roots, dangling, unverified = [], [], []
    for e in entries:
        en = e["entry"]
        p = en.get("parent") or ""
        if not p:
            roots.append(e["oid"])
        elif p not in oids:
            dangling.append((e["oid"], en.get("kind"), p))
        if not e.get("verified"):
            unverified.append(e["oid"])
    print(f"线程 {thread_id}：{len(entries)} 条（根 {len(roots)}、未验签 {len(unverified)}）")
    for oid, kind, p in dangling:
        print(f"  [链断裂] {kind} oid={oid[:12]} 的 parent={p[:40]!r} 不在本线程内 —— 该条目是孤立条目")
    if len(roots) > 1:
        print(f"  [多根] {len(roots)} 条没有 parent（{', '.join(o[:12] for o in roots)}）—— 同一线程出现多条独立链")
    if unverified:
        print(f"  [未验签] {', '.join(o[:12] for o in unverified)}")
    if dangling or len(roots) > 1:
        return 1
    print("结果：条目链完整 ✅")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--kind")
    ap.add_argument("--id", required=True)
    ap.add_argument("--actor")
    ap.add_argument("--parent", default="auto", help="上一条条目的 oid，或 auto（默认，取该线程当前 head）")
    ap.add_argument("--body-file")
    ap.add_argument("--key")
    ap.add_argument("--base")
    ap.add_argument("--head")
    ap.add_argument("--push", default="origin", help="推送到哪个远端（默认 origin=walgit）；none/- 表示只写本地 inbox ref")
    ap.add_argument("--repo", default=".")
    ap.add_argument("--verify", action="store_true", help="只校验该线程的条目链，不发条目")
    ap.add_argument("--walgit", default="")
    ap.add_argument("--config", default="")
    a = ap.parse_args()

    walgit = find_walgit(a.walgit)
    config = find_config(a.config)
    if not walgit:
        print("FAIL(前置)：找不到 walgit.exe（可用 --walgit 或 WALGIT_BIN 指定）")
        return 2
    if not config:
        print("FAIL(前置)：找不到 walgit.toml（可用 --config 或 WALGIT_CONFIG 指定）")
        return 2

    if a.verify:
        return verify_thread(walgit, config, a.repo, a.id)

    missing = [n for n, v in (("--kind", a.kind), ("--actor", a.actor), ("--body-file", a.body_file), ("--key", a.key)) if not v]
    if missing:
        print(f"FAIL(前置)：缺少 {', '.join(missing)}")
        return 2

    with open(a.body_file, "r", encoding="utf-8") as fh:
        body = json.load(fh)

    parent = a.parent
    if parent == "auto":
        r = run([walgit, "collab", "--config", config, "thread-heads", "--repo", a.repo])
        if r.returncode != 0:
            print(f"FAIL(前置)：取 thread-heads 失败（exit={r.returncode}）：{(r.stderr or r.stdout).strip()[:400]}")
            return 2
        try:
            parent = json.loads(r.stdout.strip()).get(a.id, "")
        except Exception as e:
            print(f"FAIL(前置)：thread-heads 输出不是 JSON：{e}")
            return 2
        print(f"[parent auto] {a.id} 当前 head = {parent or '(空 → 本条是根)'}")

    argv = [walgit, "collab", "--config", config, "entry", "--kind", a.kind, "--id", a.id, "--actor", a.actor,
            "--body", json.dumps(body, ensure_ascii=False), "--key", os.path.expanduser(a.key)]
    # 空 parent（线程根）不能以空串传参：PowerShell 5.1 会把空参数整个丢掉，argparse 随即吃错位置。
    if parent:
        argv += ["--parent", parent]
    for opt, val in (("--base", a.base), ("--head", a.head)):
        if val:
            argv += [opt, val]
    if a.push and a.push not in ("none", "-"):
        argv += ["--push", a.push]
    argv += ["--repo", a.repo]

    r = run(argv)
    out = r.stdout.strip()
    print(out)
    if r.stderr.strip():
        print("STDERR:", r.stderr.strip()[:1500], file=sys.stderr)
    if r.returncode != 0:
        return r.returncode

    # 发出后回读校验：新条目的 parent 必须等于预期值（这正是「变量被拆参数/粘错 oid」的现场）
    new_oid = out.split()[-1] if out else ""
    if new_oid:
        try:
            entries = read_thread(walgit, config, a.repo, a.id)
            got = next((e["entry"].get("parent") or "" for e in entries if e["oid"] == new_oid), None)
            if got is None:
                print(f"FAIL：回读线程没找到刚发的条目 {new_oid} —— 条目可能没落地")
                return 3
            if got != parent:
                print(f"FAIL：新条目 {new_oid[:12]} 的 parent={got[:48]!r} ≠ 预期 {parent[:48]!r} —— 条目链断了")
                print("      用 --verify --id " + a.id + " 看整条链，必要时删掉这条再用正确 parent 重发")
                return 3
            print(f"[校验] 新条目 {new_oid[:12]} 的 parent 与预期一致 ✅")
        except Exception as e:
            print(f"WARN：回读校验没跑成（不影响已发出的条目）：{e}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
