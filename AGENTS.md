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
