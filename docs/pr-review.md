# PR 实时跟踪评审（scripts/pr-review.ps1）

在 GitHub 仓库开放 PR 时自动完成 AGENTS.md 要求的评审与验证，并把结构化评审意见
通过 `gh pr review` 发回 PR。

## 前置条件
- Windows + PowerShell 7（`pwsh`），已安装 [gh CLI](https://cli.github.com/) 并登录（`gh auth login`）
- 本机已配置 Rust 工具链（`cargo`），且主仓库 `Client-Macroquad/Data` 存在游戏数据
  （Client-Bevy 的 `resolve_data_path` 会回退到主仓库数据目录）

## 用法
```powershell
# 一轮评审当前所有开放 PR（默认只发 comment，不自动 approve）
pwsh scripts/pr-review.ps1 -Repo gqf2008/Crystal

# 实时持续跟踪：每 60 秒轮询一次，新 PR / 新提交（head SHA 变化）自动评审
pwsh scripts/pr-review.ps1 -Repo gqf2008/Crystal -Watch

# 持续跟踪且验证通过后自动 approve（自己的 PR 无法 approve，会自动降级为 comment）
pwsh scripts/pr-review.ps1 -Repo gqf2008/Crystal -Watch -Approve -IntervalSec 120

# 强制重新评审所有开放 PR（忽略已评审记录）
pwsh scripts/pr-review.ps1 -Force

# 关闭"唤醒 Codex"（只自动回帖）
pwsh scripts/pr-review.ps1 -Watch -NoWake

# 额外发飞书群通知（需 bot 权限正常）
pwsh scripts/pr-review.ps1 -Watch -FeishuChatId oc_xxxxxxxx
```

## 行为
1. `gh pr list` 拉取开放 PR（base=master，默认跳过 draft）
2. 在 `主仓库同级/Crystal-prreview-worktrees/pr-<n>` 创建独立 worktree 检出 PR head
   （不触碰主工作区与其他 agent 的 worktree）
3. 按改动涉及范围执行验证：
   - Client-Bevy：`cargo check` + `cargo test --lib`
     （`--lib` 跑 47 个单测，避免依赖外部 Data 目录的集成测试 `ui_alignment`）
   - ServerRust / SharedRust：`cargo test`（SharedRust 仅改动时另跑 `cargo test --lib`）
   - Client-Macroquad（仅当 Client-Bevy 未涉及）：`cargo check`
4. 生成评审意见：
   - 验证失败 → `gh pr review --request-changes`
   - 验证通过 + PR 描述完整 → `--comment`（`-Approve` 时自动 approve）
5. 记录已评审 head SHA 到 `Crystal-prreview-worktrees/state.json`，避免重复评审；
   PR 有新提交（SHA 变化）会自动重新评审
6. **通知闭环**：发现新 PR / 新提交后，默认用
   `codex exec resume <CODEX_THREAD_ID>` 唤醒当前 Codex 会话接管处理（每个 head SHA
   只唤醒一次；`-NoWake` 关闭）。可选 `-FeishuChatId oc_xxx` 发飞书通知（需 bot 权限正常）

## 说明
- 评审账号与 PR 作者相同时，GitHub 不允许 approve，脚本自动降级为 comment。
- 仓库现有 CI 的 `cargo fmt --check` 为历史遗留失败（master 上同样红），
  与单个 PR 无关；本脚本的本地验证以 AGENTS.md 要求（cargo check + cargo test）为准。
