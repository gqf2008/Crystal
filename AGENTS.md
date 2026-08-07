# Agent 工作约定（Codex / Claude Code 等协作 agent 通用）

## 提交与协作规则（2026-08-03 起生效）
- 所有代码/文档改动**必须先创建分支并提交 Pull Request（PR）到 `master`**，不得直接推送到 master。
- **本机可能多个 agent 并行改代码：任何修改必须在独立的 `git worktree` 中完成**（如 `git worktree add <路径> -b <分支>`），禁止直接改动主工作区 checkout（`Crystal`），避免互相覆盖未提交改动。
- PR 合并前需完成验证：`cargo check` 通过 + 相关测试通过（客户端 `Client-Bevy`：`cargo test`；服务端 `ServerRust`：`cargo test`）。
- PR 合并前需经 review（人工或协作 agent）确认通过。
- PR 描述需写明：改了什么、为什么改、验证了什么。
- 多个 agent 协作时，各自在独立分支/PR 上工作，避免互相覆盖未提交改动。

## 项目参考原则（Client-Bevy 迁移）
- **UI 逻辑**：参考原版 C#（`Client/MirScenes/` + `Client/MirControls/`）
- **游戏绘制 / 网络**：参考 Rust（`Client-Macroquad/src/`、`SharedRust/`、`ServerRust/`）

## Issue 管理规则（2026-08-07 起生效）

- **批次化**：同类/同机制任务（同文件、同模式，如“补齐某系列怪物 AI”）≥ 3 条时，必须合并为**一个批次 Issue + checklist**（`- [ ]` 逐项），一个 worktree / 一个 PR 收一批；禁止为每个小项单独建 Issue。
- **建前查重**：新建 Issue 前先用 `gh issue list -R gqf2008/Crystal --search "关键词"` 检索，能并入已有批次/已有 Issue 的，追加 checklist 项或评论，不新建。
- **重构类硬验收**：拆文件、收敛 `#[allow]` 等纯重构必须附“行为等价”证明（同一组 e2e/快照输出 diff 为空）+ `cargo test` 全绿；无行为验证的重构优先级不高于 P3。
- **并行上限**：同一时刻活跃主线 ≤ 4 个（worktree 数量对齐），每个 worktree 只领批次 Issue，不随手拆分新 Issue。
- **僵尸清理**：超过 7 天无活动且无关联 PR 的 open Issue，自动评论“14 天后无进展将关闭”；14 天仍未动则关闭。
