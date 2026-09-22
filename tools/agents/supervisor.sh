#!/usr/bin/env bash
# supervisor.sh — 在本机拉起 N 个 agent 工人，各自一个 principal，常驻扫「待认领」。
#
# 用法:
#   tools/agents/supervisor.sh start [N]     # 默认 3 个：alpha / beta / gamma
#   tools/agents/supervisor.sh stop
#   tools/agents/supervisor.sh status
#   tools/agents/supervisor.sh logs [名字]
#
# 为什么要多个 principal 而不是一个：walgit 的条目是**按 principal 验签**的，
# 一个 principal 一把钥匙 = 一个可追溯的行动者。同名字重复拉起会争同一把钥匙，
# 反而分不清谁做的，所以每个工人一个独立名字。
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RUN_DIR="$HOME/.crystal-agents/run"
NAMES=(agent-alpha agent-beta agent-gamma agent-delta agent-epsilon)
POLL="${CRYSTAL_POLL:-20}"

mkdir -p "$RUN_DIR"

start() {
  local n="${1:-3}"
  for ((i = 0; i < n; i++)); do
    local name="${NAMES[$i]}"
    if [ -f "$RUN_DIR/$name.pid" ] && kill -0 "$(cat "$RUN_DIR/$name.pid")" 2>/dev/null; then
      echo "[supervisor] $name 已在跑（pid $(cat "$RUN_DIR/$name.pid")）"
      continue
    fi
    ( cd "$REPO" && CRYSTAL_AGENT="$name" \
        python tools/agents/crystal_agent.py --agent "$name" watch --poll "$POLL" \
        > "$RUN_DIR/$name.log" 2>&1 & echo $! > "$RUN_DIR/$name.pid" )
    echo "[supervisor] 起 $name（pid $(cat "$RUN_DIR/$name.pid")，日志 $RUN_DIR/$name.log）"
  done
}

stop() {
  for f in "$RUN_DIR"/*.pid; do
    [ -e "$f" ] || continue
    local name; name="$(basename "$f" .pid)"
    local pid; pid="$(cat "$f")"
    if kill -0 "$pid" 2>/dev/null; then kill "$pid" && echo "[supervisor] 停 $name（$pid）"; fi
    rm -f "$f"
  done
}

status() {
  for f in "$RUN_DIR"/*.pid; do
    [ -e "$f" ] || { echo "[supervisor] 没有在跑的工人"; return; }
    local name; name="$(basename "$f" .pid)"
    local pid; pid="$(cat "$f")"
    if kill -0 "$pid" 2>/dev/null; then echo "  $name  运行中  pid=$pid"; else echo "  $name  已退出  pid=$pid"; fi
  done
  echo
  ( cd "$REPO" && python tools/agents/crystal_agent.py board | head -20 )
}

logs() { tail -n "${2:-40}" "$RUN_DIR/${1:-agent-alpha}.log"; }

case "${1:-status}" in
  start)   shift; start "${1:-3}" ;;
  stop)    stop ;;
  status)  status ;;
  logs)    shift; logs "${1:-agent-alpha}" "${2:-40}" ;;
  *) echo "用法: $0 {start [N]|stop|status|logs [名字]}" >&2; exit 2 ;;
esac
