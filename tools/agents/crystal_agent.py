#!/usr/bin/env python3
"""crystal_agent.py — Crystal 的多 agent 协作工人（walgit D1 协作层）。

一套机制，三件事：

1. **身份**：每个 agent 一个 Ed25519 principal（`~/.crystal-agents/<名>.key`，
   32 字节 seed 的十六进制）。公私钥由 walgit 从 seed 派生，注册进
   `refs/collab/meta/principals/<名>`（D1 §5），此后该 agent 的每条条目都被验签。
2. **协调**：所有沟通都是 `refs/collab/*` 上的**签名条目**，没有中心状态。
   工作单元 = 一个 thread（`--id` 相同）；认领/进行/待审/完成 = 往该 thread 追加
   一条 `kind=status` 的条目。看板 `.walgit/board.toml` 再由条目**确定性投影**出来。
3. **干活**：认领后调用 headless Claude Code（`claude -p`）在本检出里完成任务，
   再 push 分支、开 PR，并把结果写回 thread。

用法
----
    python tools/agents/crystal_agent.py init  <agent>              # 建身份并注册 principal
    python tools/agents/crystal_agent.py new   <thread> "<标题>"     # 开工作单元（kind=issue）
    python tools/agents/crystal_agent.py claim <thread>             # 认领（status=in-progress）
    python tools/agents/crystal_agent.py run   <thread> "<任务描述>"  # claim → 干活 → 待审（一条龙）
    python tools/agents/crystal_agent.py status <thread> <状态>      # 手动置状态
    python tools/agents/crystal_agent.py board                      # 打印看板
    python tools/agents/crystal_agent.py watch --agents 3           # 常驻：自动认领并干活

`--agent` 默认取环境变量 `CRYSTAL_AGENT`，再默认本机主机名。

为什么认领要落锁：collab 日志是 append-only，两台机器同时认领同一张卡时
「先读后写」有竞态。同机多 worker 用文件锁彻底串行化；跨机竞态窗口靠
**回读 seq**（自己那条不是最小 seq 就退出）收敛——见 `_claim_guard`。
"""

from __future__ import annotations

import argparse
import json
import os
import secrets
import socket
import subprocess
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
AGENT_HOME = Path(os.path.expanduser("~/.crystal-agents"))
LOCK_DIR = AGENT_HOME / "locks"
WALGIT = os.environ.get(
    "WALGIT_BIN",
    str(Path(os.environ.get("LOCALAPPDATA", "")) / "Programs/walgit/walgit.exe"),
)
# collab 条目推 walgit（不是 GitHub）。设成空串 = 只写本地 refs，便于离线验证。
REMOTE = os.environ.get("CRYSTAL_COLLAB_REMOTE", "walgit")


# --------------------------------------------------------------------------- #
# 基础：调 walgit
# --------------------------------------------------------------------------- #
def _run(argv: list[str], *, stdin: str | None = None, timeout: int = 300) -> subprocess.CompletedProcess:
    return subprocess.run(
        argv, cwd=REPO, input=stdin, capture_output=True, text=True,
        encoding="utf-8", errors="replace", timeout=timeout,
    )


def walgit(*args: str, timeout: int = 300) -> subprocess.CompletedProcess:
    return _run([WALGIT, *args], timeout=timeout)


def _json_from(p: subprocess.CompletedProcess):
    """walgit 的输出是「人读摘要 + 一行行 JSON 日志」的混合；取第一条能解析的 JSON。"""
    for line in p.stdout.splitlines():
        line = line.strip()
        if line.startswith("{"):
            try:
                return json.loads(line)
            except json.JSONDecodeError:
                continue
    return None


# --------------------------------------------------------------------------- #
# 身份
# --------------------------------------------------------------------------- #
def key_path(agent: str) -> Path:
    return AGENT_HOME / f"{agent}.key"


def ensure_identity(agent: str, *, push: bool = True) -> str:
    """确保该 agent 的身份文件存在并已注册 principal；返回密钥文件路径。

    注意 walgit 的 `--key` 收的是**密钥文件路径**（32 字节 seed 的十六进制文本），
    不是内联的十六进制串——传内联串会报 `read key <串>` + os error 2。"""
    AGENT_HOME.mkdir(parents=True, exist_ok=True)
    p = key_path(agent)
    if p.exists():
        key = p.read_text(encoding="utf-8").strip()
    else:
        # Ed25519 的 seed 就是 32 字节随机数——不需要真的 keygen，
        # walgit 从 seed 派生公钥，随机 32 字节天然合法。
        key = secrets.token_hex(32)
        p.write_text(key, encoding="utf-8")
        p.chmod(0o600)
        print(f"[identity] 新身份 {agent} → {p}")
    args = ["collab", "principal-register", "--principal", agent, "--key", str(key_path(agent))]
    if push and REMOTE:
        args += ["--push", REMOTE]
    r = walgit(*args)
    if r.returncode != 0 and "already" not in (r.stdout + r.stderr).lower():
        print(f"[identity] 注册 principal 返回非零（可能已存在）：{r.stderr.strip()[:200]}")
    return str(p)


# --------------------------------------------------------------------------- #
# 条目
# --------------------------------------------------------------------------- #
def thread_head(thread: str) -> str:
    """该 thread 当前 head 的 oid（无条目返回空串）。"""
    r = walgit("collab", "thread-heads")
    obj = _json_from(r)
    if isinstance(obj, dict):
        m = obj.get("heads") or obj.get("thread_heads") or obj
        if isinstance(m, dict):
            return str(m.get(thread, "") or "")
    # 退路：逐行 "thread oid"
    for line in r.stdout.splitlines():
        parts = line.split()
        if len(parts) == 2 and parts[0] == thread:
            return parts[1]
    return ""


def post(agent: str, thread: str, kind: str, body: dict, *, base: str = "", head: str = "", push: bool = True) -> bool:
    key = ensure_identity(agent, push=False)
    args = [
        "collab", "entry",
        "--kind", kind,
        "--id", thread,
        "--actor", agent,
        "--body", json.dumps(body, ensure_ascii=False),
        "--key", str(key_path(agent)),
        "--parent", thread_head(thread),
    ]
    if base:
        args += ["--base", base]
    if head:
        args += ["--head", head]
    if push and REMOTE:
        args += ["--push", REMOTE]
    r = walgit(*args)
    ok = r.returncode == 0
    if not ok:
        print(f"[entry] 失败：{(r.stderr or r.stdout).strip()[:300]}", file=sys.stderr)
    return ok


def set_status(agent: str, thread: str, status: str, **extra) -> bool:
    body = {"status": status, **extra}
    return post(agent, thread, "status", body)


# --------------------------------------------------------------------------- #
# 看板 / 卡片
# --------------------------------------------------------------------------- #
def board() -> dict:
    r = walgit("collab", "board", "--board", ".walgit/board.toml", "--format", "json")
    obj = _json_from(r)
    return obj if isinstance(obj, dict) else {}


def cards_in(column: str) -> list[dict]:
    b = board()
    for c in b.get("columns", []):
        if c.get("name") == column:
            return c.get("cards", [])
    return []


def card_status(thread: str) -> str | None:
    for col in ("待认领", "进行中", "待审", "已完成", "受阻"):
        for c in cards_in(col):
            if (c.get("thread") or c.get("id")) == thread:
                return col
    return None


# --------------------------------------------------------------------------- #
# 认领（含竞态收敛）
# --------------------------------------------------------------------------- #
def _claim_guard():
    """同机多 worker 的串行化闸门：一次只允许一个进程走「读看板 → 写认领」。"""
    LOCK_DIR.mkdir(parents=True, exist_ok=True)
    lock = LOCK_DIR / "claim.lock"
    while True:
        try:
            fd = os.open(lock, os.O_CREAT | os.O_EXCL | os.O_WRONLY)
            os.write(fd, str(os.getpid()).encode())
            os.close(fd)
            return lock
        except FileExistsError:
            # 陈旧锁（进程已死）自动接管
            try:
                pid = int(lock.read_text(encoding="utf-8") or 0)
                os.kill(pid, 0)
            except (ValueError, ProcessLookupError, PermissionError, OSError):
                try:
                    lock.unlink()
                except OSError:
                    pass
                continue
            time.sleep(0.5)


def claim(agent: str, thread: str) -> bool:
    lock = _claim_guard()
    try:
        cur = card_status(thread)
        if cur not in ("待认领", None):
            print(f"[claim] {thread} 已在「{cur}」列，不重复认领")
            return cur == "进行中"
        if not set_status(agent, thread, "in-progress", claimed_by=agent):
            return False
        # 跨机竞态收敛：回读，若最早那条 in-progress 不是自己就让出
        time.sleep(1.5)
        r = walgit("collab", "thread", thread)
        obj = _json_from(r)
        entries = (obj or {}).get("entries", []) if isinstance(obj, dict) else []
        claimants = [
            e.get("actor") for e in entries
            if isinstance(e.get("body"), dict) and e["body"].get("status") == "in-progress"
        ]
        if claimants and claimants[0] != agent:
            print(f"[claim] {thread} 已被 {claimants[0]} 先认领，让出")
            return False
        print(f"[claim] {agent} 认领 {thread}")
        return True
    finally:
        try:
            lock.unlink()
        except OSError:
            pass


# --------------------------------------------------------------------------- #
# 干活：headless Claude Code
# --------------------------------------------------------------------------- #
def work(agent: str, thread: str, task: str, *, model: str | None = None, timeout: int = 3600) -> tuple[bool, str]:
    """在检出里跑一次 headless Claude；返回 (是否成功, 摘要)。"""
    prompt = (
        f"你是 Crystal 项目的 agent「{agent}」，正在处理工作单元 `{thread}`。\n\n"
        f"## 任务\n{task}\n\n"
        "## 铁律（必须遵守）\n"
        "- 先读 `AGENTS.md` 与 `~/.agents/rules/RULE_*.md`，按其中的流程做。\n"
        "- 一律从 master 开分支；改动要能通过 `cargo fmt -- --check` 与 `cargo test --lib`。\n"
        "- 提交用 Conventional Commits，结尾带 `Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>`。\n"
        "- 完成后开 PR（不是直接推 master），把 PR 链接写进总结。\n"
        "- 如果发现任务本身有问题，直接说明并停手，不要硬做。\n\n"
        "## 产出\n最后用一段话总结：做了什么、PR 链接、验证方式、还剩什么没做。"
    )
    argv = ["claude", "-p", prompt, "--dangerously-skip-permissions", "--output-format", "text"]
    if model:
        argv += ["--model", model]
    print(f"[work] {thread} 开工（timeout={timeout}s）…")
    try:
        r = _run(argv, timeout=timeout)
    except subprocess.TimeoutExpired:
        return False, f"超时（{timeout}s）"
    out = (r.stdout or "").strip()
    if r.returncode != 0 and not out:
        return False, (r.stderr or "").strip()[:2000]
    return True, out[-4000:]


def run_one(agent: str, thread: str, task: str, *, model: str | None = None) -> bool:
    if not claim(agent, thread):
        return False
    ok, summary = work(agent, thread, task, model=model)
    head = subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=REPO, capture_output=True, text=True
    ).stdout.strip()
    set_status(
        agent, thread, "needs-review" if ok else "open",
        ok=ok, summary=summary[:1500], head_commit=head,
    )
    print(f"[run] {thread} → {'待审' if ok else '退回待认领'}")
    return ok


# --------------------------------------------------------------------------- #
# 常驻：自动认领并干活
# --------------------------------------------------------------------------- #
def watch(agent: str, *, poll: int = 20, model: str | None = None, once: bool = False) -> None:
    print(f"[watch] {agent} 常驻：每 {poll}s 扫一次「待认领」")
    while True:
        try:
            for c in cards_in("待认领"):
                thread = c.get("thread") or c.get("id")
                title = (c.get("title") or c.get("subject") or "").strip()
                if not thread:
                    continue
                body = c.get("body") or {}
                task = body.get("task") or title or f"完成工作单元 {thread}"
                print(f"[watch] 发现待认领 {thread}：{title[:60]}")
                run_one(agent, thread, task, model=model)
                if once:
                    return
        except Exception as e:  # 常驻进程不该因为一次失败退出
            print(f"[watch] 一轮出错：{e}", file=sys.stderr)
        if once:
            return
        time.sleep(poll)


# --------------------------------------------------------------------------- #
def main() -> int:
    ap = argparse.ArgumentParser(description="Crystal 多 agent 协作工人")
    ap.add_argument("--agent", default=os.environ.get("CRYSTAL_AGENT") or socket.gethostname())
    sub = ap.add_subparsers(dest="cmd", required=True)

    sub.add_parser("init")
    p_new = sub.add_parser("new"); p_new.add_argument("thread"); p_new.add_argument("title"); p_new.add_argument("--task", default="")
    p_claim = sub.add_parser("claim"); p_claim.add_argument("thread")
    p_run = sub.add_parser("run"); p_run.add_argument("thread"); p_run.add_argument("task"); p_run.add_argument("--model", default=None)
    p_st = sub.add_parser("status"); p_st.add_argument("thread"); p_st.add_argument("status")
    sub.add_parser("board")
    p_watch = sub.add_parser("watch"); p_watch.add_argument("--poll", type=int, default=20); p_watch.add_argument("--model", default=None); p_watch.add_argument("--once", action="store_true")

    a = ap.parse_args()
    if a.cmd == "init":
        ensure_identity(a.agent); print(f"[init] {a.agent} 就绪")
    elif a.cmd == "new":
        ok = post(a.agent, a.thread, "issue", {"title": a.title, "task": a.task or a.title, "status": "open"})
        print("已开工作单元" if ok else "失败")
    elif a.cmd == "claim":
        claim(a.agent, a.thread)
    elif a.cmd == "run":
        run_one(a.agent, a.thread, a.task, model=a.model)
    elif a.cmd == "status":
        set_status(a.agent, a.thread, a.status)
    elif a.cmd == "board":
        print(walgit("collab", "board", "--board", ".walgit/board.toml", "--format", "text").stdout)
    elif a.cmd == "watch":
        watch(a.agent, poll=a.poll, model=a.model, once=a.once)
    return 0


if __name__ == "__main__":
    sys.exit(main())
