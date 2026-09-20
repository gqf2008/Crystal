# Agent 工作约定（Codex / Claude Code 等协作 agent 通用）

## 提交与协作规则（2026-08-03 起生效）
- 所有代码/文档改动**必须先创建分支并提交 Pull Request（PR）到 `master`**，不得直接推送到 master。
- **本机可能多个 agent 并行改代码：任何修改必须在独立的 `git worktree` 中完成**（如 `git worktree add <路径> -b <分支>`），禁止直接改动主工作区 checkout（`Crystal`），避免互相覆盖未提交改动。
- PR 合并前需完成验证：`cargo check` 通过 + 相关测试通过（客户端 `Client-Bevy`：`cargo test`；服务端 `ServerRust`：`cargo test`）。
  **服务端另需 `cargo fmt -- --check` 与 `cargo clippy --lib -- -D warnings`**（2026-09-19 补：CI 的 ServerRust job 就按这四步跑，而这四项此前不在本清单里，导致 fmt/clippy 的债攒到 CI 连续 27 次红才被发现）。
  **UI 相关改动另需过交互巡回门禁**：`pwsh tools/acceptance/ui_interact_sweep.ps1 -ManageServer`
  （或 `pwsh scripts/run_real_e2e.ps1 -IncludeInteractSweep`）——**退出码非 0 就不合**。
  离线测试只能证明「布局数值对」，证明不了「点得动」：#2953（扩容钮吞关闭钮点击）/#2955（关闭钮漏标记、
  根节点没接显隐、仓库窗漏 StorageWidget）那批全是布局断言全绿而交互失效。覆盖清单见
  `tools/acceptance/interact_sweep_manifest.json`（新增窗口漏登记会被 `cargo test --lib` 拦下），
  用法与退出码见 `docs/DELIVERY.md` §5.3。
- PR 合并前需经 review（人工或协作 agent）确认通过。
- **合并前必须核对该 PR 自身那次 `pull_request` run 的结论；红了就不合。**
  - 查法：`gh run list --commit <40 位完整 head_sha> --json name,event,conclusion`（认 `event == "pull_request"`）。
    **同一 SHA 会有多行**——`CI` 与 `Build & Release` 各一条，必须按 `name` 分辨，否则可能读到 `Build & Release` 的 success 而误判；
    **短 SHA 会静默返回 `[]`**，别把它当成"没有红灯"。也可用 `gh pr checks <n>`（红 exit 1 / pending 8 / 全绿 0）。
  - **该 head SHA 的 run 尚无结论（pending）时**，以**最近一次有结论**的 `pull_request` run 为准；**pending 不豁免**（本次事故的合并时刻正是 pending），绿也不强制等待（与 `RULE_CI常规以本地门禁为准` 一致）。
  - 红了就**先修红**，或本地按 CI 全步骤补齐门禁。注意 CI 是两步：`cargo test --lib` **加** `cargo test --test b0001_smoke --test ui_alignment`（`.github/workflows/ci.yml:84/89`）——只跑 `cargo test --lib` 拦不住 `b0001_smoke` 这类集成测试。
  - 本仓 master **未配置 required status checks**（`enforcement_level=off`），**红灯不会自动拦人**；`gh pr merge --admin` 按设计会绕过审批位与 required checks，**它是为「作者不能自批」准备的，不是「跳过检查」的快捷方式**。删掉 `--admin` 也不会让 CI 变成闸门——闸门只有"人核对"这一道。
  - 边界：`RULE_合并规范` 的「CI 不作为常规合并前置」指**不必等 CI 绿**，不等于**可以红着合**。
  - （2026-09-19 补：商城那个「一进游戏即崩」的 B0001 P0 就是这样进 master 的——该 PR 自身的 `pull_request` CI 自首推起一路红（合并前约 8 小时即红），三次 push 无一绿，合并时无人核对该结论。见 `LESSON_admin合并绕过红灯_必先查该PR自身run结论`。）
- PR 描述需写明：改了什么、为什么改、验证了什么。
- 多个 agent 协作时，各自在独立分支/PR 上工作，避免互相覆盖未提交改动。

## 项目参考原则（Client-Bevy 迁移）
- **UI 逻辑**：参考原版 C#（`Client/MirScenes/` + `Client/MirControls/`）
- **游戏绘制 / 网络**：参考 Rust（`SharedRust/`、`ServerRust/`）

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
- 验证基线：ServerRust `cargo check --tests` + `cargo test` + `cargo fmt -- --check` + `cargo clippy --lib -- -D warnings`（当前 368 passed）；Client-Bevy `cargo check --tests` + `cargo test`（当前 179 passed）；SharedRust `cargo test`（172+11）。
- **UI 交互门禁（2026-09-20 补）**：`Client-Bevy` 的 `cargo test --lib` 内含「40 窗点 X 关」headless 巡回
  （`src/game/dialogs/interact_gate.rs`）——名单与 `tools/acceptance/ui_interact_sweep.ps1` 对齐（漂移即红），
  窗口缺标准关闭钮 / 按压后不关栈都会红。改任何对话框、`spawn_close_button` 或关闭路径时它必跑。
  它用合成最小 `.Lib`（无 `Data/` 也跑得动，故 CI 有效），**只守接线**；像素级命中区/遮挡（如 #2953 扩容钮吞点击）
  仍须实机跑该脚本——真机侧的门禁语义见上文「UI 相关改动另需过交互巡回门禁」（脚本退出码 `0` 全过 / `1` 有用例 FAIL / `2` 前置失败）。
- 改 SharedRust 包结构需同步 `MapEditor/SharedRust` 副本 + 各客户端引用处（Client-Bevy）；协议以 Rust 客户端+服务端自洽为准（网络参考 Rust，不强制 C# 线格式）。
