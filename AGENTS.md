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

## 认领与身份标记（2026-08-10 起生效）
- 处理 Issue 前先创建 `wt/<worktree名>` label（描述注明「由 worktree <name>（本 agent）认领处理，请其他 agent 勿认领/勿改相关文件」），**Issue 与 PR 都打该 label**，避免与其他 agent 冲突。
- 一个 worktree 处理**一个批次 Issue**（一批 = 一个 PR）；新批次用新 worktree + 新分支 `feat/bevy-<批次>-batch`，完成后整批清理。
- 不认领已有 label 的 Issue（`wt/xxx` 即他人领地）；自己领地内的文件改动先确认无他人 label 交叉。

## PR 流程与清理（2026-08-10 起生效）
- 合并用 `gh pr merge --merge`（**不要 `--delete-branch`**：master 在主 worktree 检出时会报错）；合并后手动清理：
  `git push origin --delete <分支>` → `git worktree remove --force <worktree路径>` → `git branch -D <分支>` → `git fetch origin --prune && git pull --ff-only`
- 每次改动前先 `git fetch origin --prune && git pull --ff-only`（其他 agent 会持续合并）。
- PR 自审：`gh pr diff <n> --name-only` 检查无构建产物/临时文件混入；合并后确认远端分支已删、worktree 已清。

## 技术注意（2026-08-10 起生效）
- **ServerRust 源码为 CRLF**：编辑用 Python `open(path, "r", encoding="utf-8", newline="")` 读写并保持 `\r\n`，避免整文件 diff。
- PowerShell 下 `gh pr create --body` 含反引号会失败：用 Python 写 `pr_body.md`，`--body-file pr_body.md`。
- 提交用 `git add <具体文件>`，**不要 `git add -A`**（会混入 `pr_body.md`、`target/.rustc_info.json` 等临时/构建文件）。
- 验证基线：ServerRust `cargo check --tests` + `cargo test`（当前 368 passed）；Client-Bevy `cargo check --tests` + `cargo test`（当前 179 passed）；SharedRust `cargo test`（172+11）。
- 改 SharedRust 包结构需同步 `MapEditor/SharedRust` 副本 + 各客户端引用处（Client-Bevy / Client-Macroquad / ClientRust）；协议以 Rust 客户端+服务端自洽为准（网络参考 Rust，不强制 C# 线格式）。
- Client-Macroquad 存在与 SharedRust 字段漂移的预存在编译错误（~27 个），非本批次引入不要顺手修（避免与其他 agent 冲突）。
