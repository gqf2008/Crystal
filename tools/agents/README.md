# tools/agents — Crystal 的多 agent 协作

不需要人盯着的自动化：**工作单元进看板 → 工人自己认领 → 干活 → 待审 → 合并**。
协调状态全部是 walgit `refs/collab/*` 上的**签名条目**，没有中心服务、没有共享数据库。

## 为什么是 walgit 而不是"再写一个调度器"

- **身份**：每个工人一个 Ed25519 principal，条目按 principal 验签 —— 谁做了什么可追溯、不可抵赖。
- **协调**：认领/进行/待审/完成就是往同一个 thread 追加一条 `kind=status` 条目；
  `.walgit/board.toml` 是**纯函数投影**（同一份 refs，任何客户端算出字节一致的看板）——
  所以"现在谁在做什么"不需要任何额外的一致性问题。
- **门禁**：`.walgit/ci.toml` 声明命令，`walgit ci run` 在被测提交树里执行并**签名发布结果**。

## 一次性准备

```bash
# 1) 仓库接到 walgit（本机 127.0.0.1:8081，R2 后端）
git remote add walgit http://127.0.0.1:8081/gqf2008/Crystal.git

# 2) 每个工人注册自己的身份（生成本地密钥 + 注册 principal）
for a in agent-alpha agent-beta agent-gamma; do
  python tools/agents/crystal_agent.py --agent "$a" init
done
```

`~/.crystal-agents/<名>.key` 是 32 字节 seed 的十六进制文本。
**注意 walgit 的 `--key` 收的是文件路径，不是内联十六进制串**（传串会报 `read key …` + os error 2）。

## 日常：开一张卡，让它自己跑

```bash
# 开工作单元（kind=issue，落进「待认领」列）
python tools/agents/crystal_agent.py new fix-quest-detail "任务详情面板行距" \
       --task "按 C# QuestDetailDialog 对齐行距；改完跑门禁并开 PR"

# 拉起 3 个常驻工人（各自一个 principal）
tools/agents/supervisor.sh start 3
tools/agents/supervisor.sh status     # 看谁在跑 + 当前看板
tools/agents/supervisor.sh logs agent-alpha
tools/agents/supervisor.sh stop
```

工人被 `watch` 唤醒后：**认领**（写 `status=in-progress`）→ 调 headless Claude
（`claude -p … --dangerously-skip-permissions`，提示词里要求它「从 master 开分支、
过门禁、开 PR、别硬做」）→ 写 `status=needs-review` 并附 `head_commit` 与总结。
失败则退回 `status=open`，下一轮可以被别人捡走。

## 并发安全

- **同机多工人**：`~/.crystal-agents/locks/claim.lock` 文件锁把「读看板 → 写认领」整段串行化；
  锁归属进程已死时自动接管（不会被崩溃留下死锁）。
- **跨机竞态**：collab 日志是 append-only，「先读后写」仍有窗口。写完之后**回读 thread**，
  若最早那条 `in-progress` 不是自己就主动让出（`_claim_guard` 之后的收敛段）。
  窗口很小且只影响一次认领，不会造成两次改同一处。

## 门禁（`.walgit/ci.toml`）

| 任务 | 命令 | 说明 |
|---|---|---|
| `client-fmt` | `cd Client-Bevy && cargo fmt -- --check` | |
| `client-test` | `cargo test --lib` + `b0001_smoke` + `ui_alignment` | 需透传 `PATH`/`LIBPINYIN_DIR`/`PKG_CONFIG_PATH` |
| `client-check-no-assets` | `CRYSTAL_NO_DATA_ASSETS=1 cargo test --lib` | 复现 CI 无资产路径 |
| `server-fmt` / `server-clippy` / `server-test` | `cd ServerRust && …` | clippy 带 `-D warnings` |

校验：`walgit ci validate`；跑一次：`walgit ci run --once`（订阅 ref 更新后自动认领执行）。

## 与 GitHub 的关系

两者并存、各司其职：**GitHub 仍是 PR / code review 的地方**（`gh pr create`、
`gh pr merge --admin --squash`），walgit 负责**协作状态与门禁证据**。工人干的活
照旧开 GitHub PR；walgit 这边记录「谁认领、做到哪一步、门禁过没过」。
