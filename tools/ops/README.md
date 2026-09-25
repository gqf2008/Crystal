# 运营就绪最小集（部署 / 回滚 / 压测 / 可观测性）

为什么单独一条线：玩家视角五闭环都已拿到端到端状态证据（覆盖可信度 9.0/10），
但 **「可交付上线运营」是另一条更严的门**（jev 判定 p=0.030）。闭环绿 ≠ 能运营。

本目录四个脚本各对应一项，全部输出 JSON（可进流水线），判据是**确定性的**而不是「跑过了」。

## 0. 前置

- 服务端：`ServerRust/target/release/mir2_server.exe`，工作目录决定数据路径
  （`config/server.toml` 的 `network.listen_addr`，默认 `0.0.0.0:7000`；数据在 `Data/`、`Daneo1989/`）。
- 协议：外层 `[u16 长度][XOR 0xAA]`（`gate/codec.rs`），内层 `[u16 长度][i16 opcode][body]`，
  字符串是 .NET 7-bit 前缀 —— 压测机器人 `bot.py` 说的就是这个协议（不依赖 Bevy 客户端）。
- 客户端版本哈希：16 字节，服务端只校验长度（`gate/actor.rs` ClientVersion 分支），
  压测用全 0 即可。

**演练夹具前置**：多数演练（回滚 / 存储降级 / 容量标定 / 登录时延）都需要一个**能起服的最小部署目录**
（`mir2_server.exe` + `config/server.toml` + `Data/crystal.db`，`Daneo1989` 用目录联接即可）。一条命令生成：

```powershell
pwsh tools/ops/make_deploy_dir.ps1 -SourceRoot ServerRust `
    -ExePath ServerRust/target/debug/mir2_server.exe -OutDir $env:TEMP\deploy_run -Port 7200 -Force
```

> 为什么固化它：2026-09-25 我手工造夹具时漏了 `config/`，服务端于是退化成
> `Config not found → 默认配置（listen 7000 + 内存库）`，演练报告只体现成 `ready=false`——
> 读起来像"服务端起不来"，实际是夹具残缺。本脚本会**校验四件套齐全**才退出 0，
> 且 `-Force` 拒绝用于盘根/顶层目录（防误删）。

## 1. 部署：`deploy_smoke.ps1`

**判据**：① 产物存在且记录 sha256（二进制 + 客户端 exe）；② 在**全新目录**里从零起服（不是复用开发机数据目录）
→ 端口在 60s 内可连；③ 登录冒烟成功（协议级 `Login` 得到回执）；④ 报告落 JSON。

```powershell
pwsh tools/ops/deploy_smoke.ps1 -ServerDir target/release -DataRoot .. -OutDir ops_out
```

**为什么必须"全新目录"**：本轮实机验证踩过一次——从新 worktree 起服时数据目录是空的，
客户端登录后「角色 0 个」。这类**环境假设**只有从零部署才暴露得出来。

## 2. 回滚：`rollback_drill.ps1`

**判据**：部署新二进制 → 登录冒烟 + 记录数据快照（角色 gold/level/map）→ 换回**上一版二进制**
→ 再次起服 → ① 登录仍成功 ② 数据快照**逐字段不变** ③ 报告落 JSON。

```powershell
pwsh tools/ops/rollback_drill.ps1 -DeployDir <部署目录（含 Data/crystal.db、config/，以及新版 mir2_server.exe）> `
    -PrevBinary <上一版 mir2_server.exe> -Port 7200 -Account <有角色的账号> -OutFile tools/ops/out/rollback.json
```

> 参数说明（2026-09-25 更正：本文档此前写的是 `-NewBinary/-ServerDir`，脚本里根本没有这两个参数）。
> `-DeployDir` 里放的**新版**二进制就是回滚对象；脚本会把它复制成 `mir2_server.prev.exe`、
> 跑完第一轮后用它覆盖回 `mir2_server.exe` 再跑第二轮，以此证明"回滚可用且玩家档不变"。
> `-Port` 与部署配置 `[network].listen_addr` 不一致时，脚本会生成临时配置并按 `mir2_server <config>` 启动
> （与 `storage_degrade_drill` 同一套端口/超时契约）；冒烟 bot 有 `-BotTimeoutSec` 超时，超时按失败处理。

**2026-09-25 复跑（master `a5c014835` 构建 + 上一版 = 2026-09-23 release 构建 `ABD8D212…`）**：
`ok=true`、退出码 0 —— ① 新版起服 + 登录冒烟通过；② **回滚到 2026-09-23 的二进制后仍能起服 + 登录**；
③ 角色档逐字段不变（`present=true, unchanged=true, diff=[]`）。

**2026-09-25 晚复跑（扩判据之后，同一对二进制）**：同样 `ok=true`、退出码 0、`diff=[]` ——
这次比对的**关联表 11 张**都在：bevychar 背包 11 / 装备 2 / 仓库 12 / 英雄法术 1 / 宠物 1 /
已完成任务 4 / 好友 2 / 邮件 22（另有 3 张空表 n=0），并已验证这套哈希的**敏感性与稳定性**：
同一库连续两次快照完全一致；删一封邮件 → `mail n 22→21` 且哈希变化；给一条背包 item_json 追加一个字符
→ `inventory_backpack` 哈希变化而**未动表（宠物）哈希不变**。

**2026-09-25 深夜复跑（再加行会/拍卖块 + 夹具改成一键生成）**：夹具由
`make_deploy_dir.ps1` 一条命令造出（`-Port 7200` 顺带改写副本配置），回滚演练 `ok=true`、退出码 0、
两阶段 `ready/login_ok` 均 true、`diff=[]`；快照除 11 张表外还含
`guild`（TestGuild2 / 行会 sha / 成员 28 人 sha）与 `auctions`（该角色作为卖家或买家的行）。
新增块的判据同样做了双向对照：**稳定性**（含行会/拍卖在内的两次快照完全一致）+ **敏感性**
（行会金币+1 → 行会 sha 变；成员 rank_index 改 → 成员 sha 变；拍卖价+7 → 拍卖 sha 变；
而未动的背包 sha 不变）。

**回滚要验的不是"能起来"，而是"起来之后玩家的档还在"**：二进制回退 + DB 结构兼容（本仓表创建走
`IF NOT EXISTS`，但字段语义变化不在守卫内）。

**数据判据（2026-09-25 扩展）**：原来只比 `characters.gold/level/map_index/x/y` 五个标量——
回滚把背包/宠物/任务弄坏也看不出来。现在 `db_snapshot.py` 还比对 **11 张关联表**的「行数 + 规范化哈希」：
`inventory_backpack / inventory_equipment / inventory_storage / hero_inventory_backpack /
hero_inventory_equipment / hero_magics / heroes / creatures / completed_quests / friends / mail`。
易变列已在脚本头写明并排除（宠物饥饿/到期时间、邮件时间戳与正文、friends 的会话内 object_id、
角色的 hp/mp 之类），避免"合法漂移"造成假红。

**前置自检（2026-09-25 补）**：`-DeployDir` 必须是**能起服的最小部署目录**（`mir2_server.exe` +
`config/server.toml` + `Data/crystal.db`）。缺任何一个，服务端会退化成
`Config not found → 默认配置（listen 7000 + 内存库）`，报告里只体现成 `ready=false`
——看起来像产品起不来、实际是夹具残缺（本轮实测踩到，现已前置 `exit 2` 并列出缺项）。

## 3. 压测基线：`load_baseline.ps1`（驱动 `bot.py`）

**判据**（阈值都在脚本里，可覆盖）：

| 指标 | 阈值 | 为什么 |
|---|---|---|
| 并发登录成功率 | 100% | 有任何一条失败就说明账号/门闸路径有并发问题 |
| 登录 p95 延迟 | < 1000 ms | 登录是运营第一印象，且是纯门闸路径 |
| 世界会话保持期错误 | 0 | 会话被踢/断线都算 |
| 服务器 tick 间隔抖动 | 均值 ±50% | 本仓历史上出现过 KeepAlive 乒乓、tick 内联大 Future 打爆栈 |
| 进程 RSS 增长 | < 200 MB / 窗口 | 泄漏的第一信号 |

```powershell
pwsh tools/ops/load_baseline.ps1 -ServerDir <运行目录> -GateSessions 30 -WorldSessions 4 -HoldSec 20
```

**规模诚实说明**：这是**单机基线**，不是生产容量结论。要出容量结论需要独立压测机 + 生产拓扑，
本脚本产出的是「N 会话下同一台机器上服务端是否稳」的可复现基线，以及一个可扩展的机器人。

## 4. 可观测性最小集：`health_report.ps1`

上线后「靠什么判断正在变坏」——本脚本把日志与进程指标折成一份 JSON + PASS/WARN 判定：

- **tick 节拍**：解析 `World tick #N` 行，算实际间隔与抖动（服务端卡死/半死最先在这里现形）；
- **错误计数**：`ERROR` 行数、`WARN` 分类计数；
- **已知坏味道**（逐条给判据，不是"看看日志"）：
  - `send buffer full ... kicking slow reader`（慢读者被踢——人还在掉线）
  - `gate mailbox full`（背压丢包）
  - `frame decode failed` / `read error`（协议层异常）
  - `NPC call for unknown object_id`（客户端发错地址，之前踩过）
- **进程指标**：RSS / CPU 时间 / 句柄数。

```powershell
pwsh tools/ops/health_report.ps1 -LogFile ops_out/server.log -ProcessName mir2_server -OutFile ops_out/health.json
```

## 5. 容量标定：`capacity_ramp.ps1` + `CAPACITY.md`

单机阶梯标定（每档独立重启服务端）：会话成功率、登录 p95、`gate mailbox full` 丢包、
慢读者踢线、tick 心跳滞后、RSS/会话。账号靠 `seed_load_accounts.py` **离线播种**
（服务端对账号/角色创建有 IP 防刷，单机注册不出几百会话；账号名必须纯字母数字）。

实测结论（详见 `CAPACITY.md`）：登录路径 200 并发仍零失败（p95 2.96s）；
**世界路径拐点在 10→20 会话之间**（20 会话起 `gate mailbox full` 上万条 = 世界→客户端扇出打满背压）。

## 5b. 内存阶梯：`memory_ramp.ps1`

**只按 PID 管控自己启动的服务端**（不按进程名 `Stop-Process`），所以可以在同一台机器上保留别的
服务端实例；采样每 2s 一次、取保持期最大值（`Process` 属性必须 `Refresh()`，否则整轮都读到同一个
陈旧值——踩过）。
（2026-09-25 更新：`capacity_ramp.ps1` 原本是「按进程名清场 + 按进程名取 RSS」，已改齐到同一口径，
并由静态门禁 `check_process_scope.ps1` 防回流。）

```powershell
# A/B：同一份代码 stash 前后各 build 一次 release，分别跑
pwsh tools/ops/memory_ramp.ps1 -DeployDir <deploy> -ExePath <exe> -StepsCsv 10,20 `
     -Tag before -OutFile tools/ops/out/memory_before.json
```

用途与结论见 `CAPACITY.md` §4.2（7.5MB/会话拆解）。

## 5c. 运行中存储故障降级：`storage_degrade_drill.ps1` + `db_write_lock.py`

`fault_injection.ps1` 的 `db_readonly` 只覆盖「**起服前**把 DB 置只读」；本项注入**运行中**的写故障：
服务端已有会话时，另一个进程用 `BEGIN IMMEDIATE` 占住 SQLite 写锁 20s（比 `busy_timeout=5000`
长），期间走一个完整会话，它的落库必然撞锁。

```powershell
pwsh tools/ops/storage_degrade_drill.ps1 -DeployDir <deploy> -OutFile tools/ops/out/storage_degrade.json
```

**端口与超时契约（2026-09-25 修，两条都是"静默挂死"类缺陷）**：

- `-Port`（默认 7100）**现在同时决定服务端监听口**。服务端读的是 `<DeployDir>/config/server.toml` 的
  `[network].listen_addr`；两者不一致时（例如部署配置写 7000、而 `-Port` 传 7100），本脚本会生成一份
  临时配置（只改 `listen_addr`）并按仓库既有约定 `mir2_server <config>` 启动，同时在输出里写明用的是哪份。
  修复前该场景会**静默挂死**：服务端绑不上 7100、bot 却往 7100 连，整轮无任何输出（实测挂了 6 分钟以上）。
- 单次 bot 会话有 `-BotTimeoutSec`（默认 120s）超时；超时按该次采样失败处理并**立刻**返回。
  阳性对照：`-BotTimeoutSec 1` → 3 秒内输出 `WARN: bot.py 超过 1s 未退出…` + `FAIL(J0)…`，退出码 3
  （不是挂死）。
- 就绪判据从「日志里出现 `Gate listening`」改为「`-Port` 真的在监听」——日志字符串可能来自别的实例。

判据（J1–J4，缺一不可）：写失败后**进程仍在** / 失败被**明确记录** / 故障期间**读路径不受影响**
（新会话仍能登录）/ 释放锁后**恢复**（新会话下线重新出现 `saved to database on logout`）。

**2026-09-23 实测结果：J1–J4 全绿**（`ok=true`，`save_failure_lines=2`）。同一份日志里的时间线：

**2026-09-25 复跑（master `343e6f70e`，独立部署副本 + 当前构建）：J1–J5 全绿**（`ok=true`，退出码 0）——
写锁 22s（`LOCK_HELD … LOCK_RELEASED after=22.0s`）、`save_failure_lines=8`、
故障窗口内 `PERSIST_LOST account_save phase=login` / `player_pets` / `player_character phase=logout` 三类都被明确记录、
读路径不受影响、释放锁后 `saved to database on logout` 由 1 行增至 2 行、
且客户端**真的看到**提示：`存档失败：账号信息未能写入数据库（phase=login）。请联系管理员；本次改动可能不会被保存。`
（与 owner 2026-09-24 拍板一致：失败一律直接反馈到客户端，服务器侧不做保护/补偿）。

| 时刻 | 事件 |
|---|---|
| 11:33:16.478 | `WARN account: Failed to save account 'opsload1' on login: ... database is locked` |
| 11:33:16.481 | `Player OpsLoad1 entered world`（**写失败没有阻断登录**） |
| 11:33:26.069 | `WARN world: Failed to save player pets for OpsLoad1: ... database is locked`（撞满 5s busy_timeout） |
| 11:33:30.026 | `Player OpsLoad1 saved to database on logout`（锁释放后落库成功） |
| 11:33:36.197 | 恢复后的新会话同样 `saved to database on logout` |

**如实记的缺口（后续项，不是"已完成"）**：撞锁失败时是 **warn 即放弃、没有重试/补偿**——
实测这一轮里**宠物持久化那一步是直接丢掉的**（`Failed to save player pets` 之后没有重试），
角色本体那次是等到锁释放后才落库成功。也就是说：长时间写锁 > busy_timeout 时，
下线路径上的部分持久化数据（本例是 pets）会静默丢失。要做的是给下线/落库路径补**有界重试
+ 失败补偿（例如失败入队、重连后补写）**，并把它做成可注入的门禁。

### 5c-1. 缺口修复：静默丢失 → 响亮报告（**不做重试**，附实测依据）

代码侧改动：`db::persist_report(what, ctx, op)`（`ServerRust/src/db/mod.rs`）——**一次尝试**，
失败时按「是否瞬时锁错误」（`is_transient_db_error`：`database is locked` / `code: 5|6`）分级：
瞬时 → `ERROR PERSIST_LOST <what> <ctx> err=...`（可 grep / 可告警）；非瞬时 → `WARN persist failed ...`。
落点：下线/断线的 `save_character` / `player_pets` / `save_heroes` / `update_last_access`，
以及账号的 `save_account` / `set_account_offline`（共 7 处）。

**为什么不做重试**（这是本轮实做 A/B 之后的结论，不是省事）：

| 方案 | 14s 写锁 | 22s 写锁 | 登录读路径（J3） |
|---|---|---|---|
| master（单次尝试 + warn） | 1 条写失败 | 2 条写失败 | 1.6s / 9.5s（勉强成功） |
| 加重试（1 次退避，实测） | **0 条写失败** ✅ | PERSIST_LOST | 7.1s，22s 锁下**超时失败** ❌ |
| 重试 + 账号写挪后台任务 | 0 条写失败 | PERSIST_LOST | 22s 锁下仍失败 ❌ |
| **本轮采用：单次 + 响亮报告** | 1 条失败（但 ERROR + 可告警） | PERSIST_LOST | **1.6s（与 master 一致）** ✅ |

机理：每次尝试都要占住一条连接等 `busy_timeout=5000`；重试把占用从 5s 拉到 ~10s，
而**登录读路径**要排队等连接/等锁 → 为了救写把**读**挤坏了（22s 锁下同一会话直接超时）。
所以本轮只落「不再静默」这一半；**重试留到能同时约束连接占用的方案再做**（见下）。

**已验证**：本地 `cargo test --lib` 828 passed（含 `transient_db_error_classification`、
`persist_report_passes_result_through` 两条门禁，阳性对照实做：把 `is_transient_db_error`
改成恒 true → 后两条断言立红）；`cargo fmt -- --check` / `cargo clippy --lib -- -D warnings` 干净；
drill A/B：14s 锁下与 master **行为等价**（同样的 1 条失败），差别只在日志级别与固定前缀。

### 5c-2. 失败入队 + 恢复补写（补偿队列）：把「可告警」变成「不丢」

> **2026-09-24 owner 拍板**：落库失败一律**直接反馈到客户端**；服务器侧**暂不要求**保护/补偿，
> 也**不要求落盘 spool**。本节实现的补偿队列因此**降级为既有低成本兜底、不作为上线门槛**；
> 门槛改为「失败必须让玩家看到」（见 `db::persist_failure_notice` + `notify_persist_failure`）。

`ServerRust/src/actors/world/persist_queue.rs`（新模块）：
落库失败的数据不再丢——进队列，等存储恢复后在世界 tick 上有界补写。

- **为什么是队列 + 每 tick 有界 flush**（而不是原地重试）：原地重试会把连接占用从 5s 拉到 ~10s，
  把登录读挤到超时（§5c-1 的实测）；队列把重试挪到世界自己的 tick 上，每 tick 只试
  [`FLUSH_PER_TICK`]=2 条，代价摊平且可观测（`PERSIST_REPLAY` / `PERSIST_DROPPED` 日志）。
- **去重**：同 `(玩家, 类型)` 只留**最新**一份快照（后写胜过先写是正确语义），所以同一个人反复
  失败不会把队列撑爆。
- **轮转**：本轮失败的条目挪到队尾——否则队首两条一直失败会把后面所有玩家的补写饿死。
- **上限与放弃**：队列 ≤128 条、单条 ≤20 次尝试；超限/超次数都 `error! PERSIST_DROPPED`
  （**响亮**丢弃，不静默）。角色快照一条几十 KB，128 条约数 MB 量级，只在存储故障时才会用满。
- **覆盖范围**：角色全量（`player_character`，含背包/仓库/任务/邮件）、英雄列表、驯服宠物、
  最后下线时间。账号行（`account_save`/`account_offline`）**刻意不入队**——它是簿记，
  每次登录都会重写，入队收益低。

**门禁（4 条，`persist_queue::tests`）**：失败必须留队 + 轮转不吃饿死 / 超次数响亮丢弃 /
同玩家同类型去重 / 容量上限丢最旧。阳性对照实做：把失败分支的 `deferred.push(item)` 删掉
（失败即丢）→ 前两条门禁立即红，撤回后绿。

**实机证据（drill，22s 写锁）**：

```
ERROR db: PERSIST_LOST player_pets player=OpsLoad1 err=... database is locked     ← 写失败（够响亮）
INFO  world::persist_queue: PERSIST_REPLAY ok player_pets player=OpsLoad1 attempts=1  ← 锁释放后补写成功
INFO  world: PERSIST_REPLAY tick ok=1 dropped=0 queued=0 (was 1)                  ← 队列清空
```

也就是说：**此前会丢的那条宠物持久化，现在补回来了**。

**下一轮要做的持久化可靠性工作**（本轮没做，别当已完成）：

1. **缩短每次尝试的 busy_timeout** 或**读写分离连接池**，让「重试」不再牺牲登录读路径；
   之后重试才谈得上收益（目标：长锁下既零丢失、登录也不超时）。
2. ~~离线补偿队列~~ **已做（§5c-2）**：失败入队 + 每 tick 有界补写；实机验证宠物持久化补回。
3. 登录读路径在长写锁下的延迟本身要单独定位（master 也出现过 9.5s 的登录回复；
   候选：连接获取排队、`list_character_summaries` 读、WAL checkpoint）。

## 5c-2. 连登连退的平台门禁：`leak_plateau.ps1`

把「登出后不归还」（CAPACITY.md §4.1 → §4.1b 已更正为**预热台阶 + 平台**）做成可复跑门禁：
预热若干轮后，再测量若干轮，断言**平台**而不是"没涨"。

```powershell
# 前置：deploy 库里要有 -AccountPrefix 前缀的账号+角色
python tools/ops/seed_load_accounts.py <deploy>/data/crystal.db 20 --prefix opsload --char-prefix OpsLoad
pwsh tools/ops/leak_plateau.ps1 -DeployDir <deploy> -ExePath <mir2_server.exe> `
     -Sessions 20 -WarmCycles 3 -MeasureCycles 5 -OutFile tools/ops/out/leak_plateau.json
```

判据（缺一不可）：**J1** 每个 idle 采样 `online_players == 0`（玩家记录真回收）；
**J2** `tasks.running` 回到基线（后台任务不泄漏）；**J3** 测量轮 idle 的**线程数±2 / 句柄数±8** 稳定；
**J4** 测量轮 idle RSS 斜率 ≤ `-MaxRssSlopePerCycleMb`（默认 0.5 MB/轮，20 会话/轮）。
基线会话失败（`failed>0`）直接 exit 3，不产出空转报告。

**2026-09-24 release 实测**：`ok=true`，测量段 RSS 43.0/43.2/43.3/43.9/43.4 MB（斜率 **0.1 MB/轮**），
线程 span 0、句柄 span 0、`online_players` 每轮回 0、`tasks.running` 每轮回基线 3。
注意 admin 端口 = gate 端口 **+1**（脚本按 `-Port + 1` 取；硬编码 7001 会拿到 -1）。

**2026-09-25 复跑 + 时长前置守卫**（20 会话、默认时长 Hold 8s / Idle 12s）：`ok=true`，
测量段 RSS 116.51/117.76/119.05/118.29/118.44 MB，斜率 **0.482 MB/轮**（阈值 0.5），
`J1/J2/J3` 全绿（线程 span 0、句柄 span 0、每轮回 0 在线、任务回基线）。

> **为什么加了 `-HoldSec ≥ 8 / -IdleSec ≥ 12` 的前置守卫**：我先把时长压到 Hold 6 / Idle 6 想快一点，
> 结果斜率 0.857、`J4` 假红——那测到的是**高水位预热尾巴**，不是泄漏（增量 1.14→1.19→0.90→0.20 递减）。
> 用文档默认时长同一份数据就回到 0.482 通过。所以脚本现在直接拒绝出结论（`exit 2`）并说明：
> 要缩短总时长请减 `-MeasureCycles`，别压 `-HoldSec/-IdleSec`。
>
> **余量提示（如实记）**：默认时长下 0.482 vs 阈值 0.5 只剩约 4% 余量，属**会抖的门禁**——
> 发布判定建议用更长窗口（更多 `-MeasureCycles` 或更长 Hold/Idle）复跑；单机窗口再长也替代不了
> `CAPACITY.md` §7 的 ≥24h 老化（那是外部项）。

### 5c-2a. 本轮把这条门禁查到底：release 上 J4 红、且不是 session 键容器泄漏（2026-09-25）

三条证据把它从"偶发假红"推进到"有实测结论的缺口"：

1. **同设置会抖**（debug 构建，20 会话、默认时长，连跑 3 次）：端点斜率
   **0.482（ok）/ 0.81（红）/ 0.375（ok）** ⇒ 1/3 假红。逐轮增量呈 `+2.05, +0.44, -1.48, +0.49`
   这类**混合符号**，说明 5 点端点斜率被单点离群值主导。报告现在同时给出
   `rss_increments_mb`（逐轮增量）与 `rss_slope_regression_mb_per_cycle`（最小二乘斜率）与
   `build_kind`，便于一眼区分"持续增长"与"某点抖动"（判定语义未改）。
2. **release 构建（当前 master）上判据红，且不收敛**：
   - 默认 5 轮测量：`ok=false`，端点 **0.742**、回归 **0.728**，RSS 49.68→52.65（增量 1.51/0.52/0.82/0.12）；
   - 12 轮测量：`ok=false`，端点 **0.611**、回归 **0.66**，RSS 50.07→56.79（**+6.7MB**，
     增量 `+1.07,-0.07,+0.21,+1.07,+0.88,+0.89,-0.20,+1.56,+1.27,-0.22,+0.26`）。
   而本夹具 2026-09-24 在 release 上的基线是 **0.1 MB/轮**（43.0→43.4 已平台）。
3. **排除"按 session 键的容器没清"**：加了只读探针（`MIR2_LEAK_PROBE=1` 时在每次断线清理后打印
   33 个 session 键容器的尺寸，见 `session.rs` 的 `LEAK_PROBE`），release 连登连退多轮实测
   **每一个容器都是 0**（`players/buyback/chat_items/npc_timers/last_move/delayed_actions/
   flaming/double_hit/mp_eater/hemorrhage/mental/counter_attack/targets/pet_modes/last_mail/
   death_queue/fishing/fishing_counters/session_npc/session_npc_page/market_*/poison/stacking/
   logout_block/observe_links/rental/invisible/hidden/gm_observer/sneaking/slaying` 全 0）。

**因此当前状态（如实记）**：release 上 20 会话连登连退的空闲 RSS **每轮约 +0.6MB 且 12 轮内不收敛**，
但**不是** per-session 容器泄漏（那批全清）；`J3`（线程 span 0 / 句柄 span 0）也证明没有 OS 资源泄漏。
嫌疑剩下：分配器高水位/arena 增长、按**非 session 键**（地图/全局/社交/DB 层）的缓存、或某条登录/登出
路径持有的长生命周期对象。**下一步**：① 用更长的窗口（≥30 轮）看是否最终平台（高水位 vs 缓慢泄漏）；
② 给分配器/关键非 session 容器加同样的只读探针；③ 跨版本对比需要**与旧二进制 schema 匹配的库**
（本轮用当前库跑 2026-09-23 的 release，第 8 轮整批登录失败 → `FAIL(J0)` exit 3，夹具正确地拒绝出结论）。

**两条前置守卫（本轮加）**：`-HoldSec < 8 / -IdleSec < 12` 直接 `exit 2`（否则测到的是高水位预热尾巴，
实测 0.857 假红）；**debug 构建**未显式加 `-AllowDebugThreshold` 也 `exit 2`（阈值与基线都是 release 标定的）。

### 5c-2b. 计数分配器：把"RSS 在涨"拆成"活跃字节在涨"（2026-09-25）

RSS 涨可能是**真泄漏**（活跃分配持续增长）也可能是**分配器高水位/arena**（活跃字节平稳、RSS 留高位）——
两者修法完全不同（前者改代码，后者不影响可用性）。为此加了**只在特性开关下编译**的计数分配器：

```powershell
cargo build --release --features mem-probe --bin mir2_server     # 默认构建不含该特性 ⇒ 零开销
$env:MIR2_LEAK_PROBE=1
pwsh tools/ops/leak_plateau.ps1 -DeployDir <deploy> -ExePath ServerRust/target/release/mir2_server.exe -Port 7400 -Sessions 20 -MeasureCycles 20
```

打开后，每次断线清理会多打一行 `MEM_PROBE sid=… live_bytes=… allocs=… deallocs=… live_le256b=…
live_le16k=… live_gt16k=…`（按尺寸分档的活跃字节）。

**实测（release + mem-probe，20 会话）：**

| 量 | 起点 | 终点 | 说明 |
|---|---|---|---|
| RSS | 49.04 MB | 60.57 MB | 20 轮仍爬升（端点斜率 0.607、回归 0.531，J4 红） |
| **live_bytes** | 19.63 MB | 25.77 MB | **活跃字节同向增长** ⇒ 不是单纯的 arena 高水位 |
| live ≤256B | 2.168 MB | 2.302 MB | 小对象档只涨 0.13 MB |
| live ≤16KB | 9.884 MB | 10.747 MB | **中档（257B–16KB）贡献 +0.73 MB，最大** |
| live >16KB | 4.637 MB | 5.022 MB | 大档 +0.38 MB |

即：20 会话每轮约 **10–15 KB/次登出**的**活对象**被保留下来，集中在 257B–16KB 档。
候选方向（下一步）：社交 actor 的在线表（登出通知走 `try_send`，邮箱满即丢 ⇒ 在线条目可能残留）、
按非 session 键的缓存、以及登录/登出路径上被 `Arc` 延长生命周期的对象。

### 5c-2c. 沿候选逐个排除：**找到一个真泄漏（`player_heroes`）**，并纠正一处无效采样（2026-09-25）

**先纠正我自己的测量**：上一节的 `live_bytes` 取自 `PlayerDisconnected`（罕见路径，一轮 460 次登出里只出现 10 次），
取样时机随事件漂移（可能取在别的会话仍在线时），所以那条"增长"不可靠。现在把计数分配器读数挪到
**每轮一次的空闲点**（`cleanup_map_spawns` 之后，与 `leak_plateau` 的 RSS 采样同相位）——
相位一致后仍然是**单调增长**：`live_bytes 16.79→18.83 MB`（5 轮，+0.41 MB/轮），RSS `48.08→51.46 MB`。

**逐个排除（都带读数）：**

| 假设 | 探针读数 | 结论 |
|---|---|---|
| 33 个 session 键容器的清理 | 断线清理后**全为 0** | 排除 |
| 社交 actor 在线表（`try_send` 丢通知） | `SOCIAL_PROBE joined=95 / left=95`、丢弃 **0** 次、`players` 逐条降到 **0** | 排除 |
| 按 object_id 的辅助表（`monster_targets`/`cursed`/`revealed_hp`/`pet_*`…） | 每次整图清理后**全为 0** | 排除 |
| PlayerActor 没被释放 | 新增存活计数器 `live_player_actors`：每轮空闲点**恒为 1**（不增长） | 排除 |
| **`player_heroes`（session 键）** | 每轮空闲点 **19→38→57→76→95→114**（+19/轮，单调） | **真泄漏，已修** |

`player_heroes: HashMap<session_id, Vec<HeroInfo>>` 在登录时插入（`StartGame` 载入英雄列表），
而**登出与断线两条清理路径都没有删它**——session id 每次唯一 ⇒ 条目永久累积
（同时是"过期会话键"的正确性问题）。修法＝两条路径各补一行 `self.player_heroes.remove(&session_id)`；
修后每轮空闲点 **`player_heroes=0`**。

**仍未归因**：修掉 `player_heroes` 之后 `live_bytes` 依然每轮约 +0.5MB（16.32→18.84 MB / 5 轮），
而上述所有容器与 actor 计数都平稳、DB/WAL 体积稳定（2.59MB + 4MB WAL 不随轮次增长）。
下一步候选：依赖层（sqlite/page cache、tokio 缓冲）、`MapCache`、以及尚未逐个列出的结构；
必要时用按调用点归因的分配分析（当前计数分配器只给总量与尺寸档）。

### 5c-2d. 精确尺寸直方图 + 累计增长榜 + 一个会让上面所有判据假绿的夹具缺口（2026-09-25）

**仪器升级**（`--features mem-probe` + `MIR2_LEAK_PROBE=1`，每轮空闲点打印，见 `ServerRust/src/mem_probe.rs`）：

| 行 | 含义 |
|---|---|
| `MEM_PROBE_IDLE …` | 总量与三档分档（不变） |
| `MEM_PROBE_NET dpos=… dneg=…` | 本轮**全部尺寸**的正/负增量之和——一起看就知道增长是集中还是弥散 |
| `MEM_PROBE_DELTA size=… dbytes=…` ×8 | 本轮变化最大的 8 个**精确尺寸** |
| `MEM_PROBE_CUM size=… cum_dbytes=…` ×10 | 相对**第一次采样**的累计增长榜（泄漏指纹） |
| `MEM_PROBE_CUMNET cum_pos=… cum_neg=…` | 累计正/负之和（判断 top-10 盖住了多少） |

读侧：`py -3.12 tools/ops/mem_probe_report.py [日志] --skip 3`（只读、不起服务端；自动算「泄漏指纹」＝每个测量轮累计榜里都出现的尺寸）。

**为什么必须看累计榜**：单轮 top-8 里全是 ±几 KB 的分配抖动（实测第 3–7 轮前几名都是 300–500B 的「每轮 +20 个」这种小信号），
而累计 7 轮后同一个尺寸会涨到 +50KB，才从噪声里浮出来。

**实测（release + mem-probe，3 预热 + 5 测量，端口 7450）**：

| 运行 | live_bytes | 活跃对象数 | 每轮增量 | 平均每对象 |
|---|---|---|---|---|
| 20 会话 | 16.72 → 19.49 MB（+2.76 MB / 7 轮） | 47,595 → 54,195 | **+394,696 B、+943 个对象** | ~419 B |
| **1 会话** | 16.01 → 17.87 MB（+1.86 MB / 4 轮） | 44,089 → 48,882 | **+464,204 B、+1,198 个对象** | ~387 B |

**两条结论（各自限定范围）**：

1. **增长与并发会话数无关**：1 个会话每轮涨得与 20 个会话一样多（甚至更多）⇒ **不是登录/登出/会话级对象泄漏**，
   而是**每轮发生一次**的东西。本演练每轮都会「全员登出 → `cleanup_map_spawns` 整图清理 → 下一轮重新物化整图」
   （§4.6 实测每轮 1 次 × 1912 只怪）。**每轮约 1000 个对象（平均 ~400B）被永久保留**。
2. **增长弥散在很多尺寸上**：累计正增量 2.23MB，top-10 尺寸只占 ~0.47MB（21%）——这 ~1000 个对象分布在
   **很多不同尺寸**上（每尺寸每轮 +1 个量级），不像某个固定结构在攒，更像「每个被物化的怪物留下一个可变长度对象」。

**下一步（已写进线程 residual，未做）**：给计数分配器加**标签**（分配时记标签、释放时按标签减），
把候选子系统（整图物化 / 整图清理 / 登录 / 登出 / 落库 / 广播）各占多少活字节读出来；
或先做更便宜的判别：让一个会话跨轮保持在线（地图不被清理），看增长是否消失。

**⚠️ 顺带修掉一个会让上面所有判据假绿的夹具缺口（J0b）**：bot 的 `ok` 只代表「登录 + 保持连接」，**不代表进图**。
实测 `-Sessions 1` 时（`bot.py` 的 `--account-prefix` 从 **0** 起编号，而 `seed_load_accounts.py` 播的是 `<prefix>1..N`）
用的是不存在的 `opsload0`：服务端**自动建号**（登录成功、没有角色）、`StartGame rejected` —— 一个会话都没进图，
于是没有地图物化、也没有整图清理，**J1–J4 全绿、RSS 斜率 0.02 MB/轮、exit 0**。现在：

- 账号改用显式 **1-based** 列表（与 `seed_load_accounts.py` 对齐）；
- 新增 **J0b**：用**服务端日志**卡「每轮 `StartGame: session=` ≥ N 且 `StartGame rejected` == 0」，与 bot 的自我报告无关。
  阳性对照：`-AccountPrefix nosuchacct` → `FAIL(J0b): 第 1 轮只有 20/20 个会话真的进图（StartGame rejected=20）`、
  **exit 3**（修复前同样的运行是绿的）；
- 清场从「按进程名杀所有 `mir2_server`」改成**端口 + 同 `-ExePath` 残留**的前置失败（同机多 agent 并行时不再误杀别人的实例；
  静态门禁 `tools/ops/check_process_scope.ps1`）。

### 5c-2e. 按调用点归因（标签）+ 两条排除结论（2026-09-25）

**仪器**：给计数分配器加**标签**——分配时把当前标签写进块首 16 字节头部、释放时读回来按标签减；
调用点用 RAII `TagGuard::enter(TAG_X)` 圈定可疑区域（`mem_probe.rs` 的 `TAG_*`）。空闲点多打两组行：
`MEM_PROBE_TAG tag=… live_bytes=… dbytes=…` 与 `MEM_PROBE_TAGSUM sum=… live_bytes=… gap=…`
（`gap` = 未打标签的字节；目前只有 `align>16` 的分配不在标签内，实测**恒定 262,144 B**，说明标签铺得还算全）。

**踩坑（已写成 LESSON）**：第一版把「是否带头部」的决定挂在运行时开关 `enabled()` 上 ⇒
「开关打开**之前**分配、打开**之后**释放」的块会被当成带头部的块（读错标签头 + 按错布局 free）——
**堆损坏**。表现极具迷惑性：服务端正常起、`Gate listening` 正常，但 20 个客户端**全部**登录失败
（`ok=0 failed=20`）、日志里**没有 panic**。纠偏：布局决策**只看 `layout.align()`**、与开关无关，
开关只决定记账（代价：mem-probe 构建里 16 字节对齐以内的分配恒多 16 字节头部；默认构建不含该模块）。

**两条排除结论（实测）**：

1. **不是按时间/tick 泄漏**：把每轮空闲窗口从 12s 拉到 **90s**（轮时长约 3 倍），
   每轮净增仍只有 **+460,658 / +488,385 B**（IdleSec=12 时是 +394,696 B/轮），**没有随轮时长按比例涨**
   ⇒ 增长来自**每轮发生一次**的事件，不是 tick 循环。（顺带给出按时间分量的上界：≲1 KB/s。）
2. **不是空载荷空转**：J0b（§5c-2d）已保证每轮 20 个会话真的进图。

**阶段分解（20 会话，`MEM_PROBE_PHASE` 打点，取最后一轮）**：

| 段 | Δbytes | Δ活跃对象 |
|---|---|---|
| idle → materialize_begin（首个进图会话的准备） | −44,252 | −884 |
| materialize_begin → materialize_end（物化/复用发送 + 入库 + 精英广播） | **+1,021,289** | +3,111 |
| materialize_end → cleanup_begin（游玩 + 登出） | **−667,390** | +1,097 |
| cleanup_begin → idle（整图清理） | −13,836 | −2,435 |
| **一轮净增** | **+295,811** | ~+760 |

即：物化段一次分配约 1MB，其中约 2/3 在「游玩+登出」段被释放，**约 300KB/轮留下来**。
标签读数另给出一处**与每轮无关**的保留：`materialize` 标签上**恒定**挂着 **1,908,464 B**——
它跨轮不涨，但整图清理也不释放它，属「每图一次性 ~1.9MB 保留」，与每轮 +0.3MB 是两件事，都已进 residual。

**已知边界（写清，别当成精确归属）**：标签是**线程局部**的，而带 `await` 的区域在等待期间会让出线程，
同线程上交错执行的其它任务会把分配记到当前标签上 ⇒ 带 `await` 的区域（如清理里的玩家遍历）有**误归因**。
所以本轮标签结论只用于粗粒度排除；下一步＝把标签只圈在**同步段**，并在物化路径内部再切几刀
（`spawn_npcs_and_monsters` 本体 / 入库循环 / 精英广播 / 物化后的发包）。

## 5d. 写锁下的登录时延：`login_latency_probe.ps1`（带阈值夹具）

判据（缺一不可）：L1 写锁**真注入**（等注入器输出 `LOCK_HELD`，拿不到直接退出码 3、不产出结论）；
L2 持锁 20 样本 p95 < 1.0s；L3 对照 p95 同样达标；L4 持锁 p50 < 0.3s。

```powershell
pwsh tools/ops/login_latency_probe.ps1 -DeployDir <deploy> -ExePath <mir2_server.exe> `
     -OutFile tools/ops/out/login_latency.json
```

**递进定位（同一夹具、30s 写锁、每档 20 样本）——每一步都靠分段日志而不是猜**：

| 版本 | 持锁 p50 | 持锁 p95 | 结论 |
|---|---|---|---|
| 修前（master f6fa868d） | 0.023s | **5.575s** | `LOGIN_TIMING account … save_ms=5554 list_ms=0` → 根因① `finish_login` 的 `save_account` 在 AccountActor 里 await 等满 busy_timeout |
| 把账号登录写挪后台 | 0.023s | 5.529s | account 侧 0ms 了，但 `gate_ask` 仍 5.5s → 根因② 排在前一条消息后面：登出路径的 `set_account_offline` 同样在 actor 里 await |
| 再挪账号离线写 | 0.027s | 5.122s | 露出根因③：`list_ms≈5.1s` —— 被锁住的写**每条占住一条池连接 5s**，登录节奏下耗尽连接池，登录的读取不到连接（跨进程纯读只要 0.001s，读本身无罪） |
| 预热门（min=max=8） | 0.022s | 5.234s | 预热不够：连接仍会被"占用" |
| **连接 busy_timeout 收到 500ms** | **0.029s** | **0.382s** | 占用上界 = 0.5s → 登录延迟上界随之钉住；`>=500ms` 的 LOGIN_TIMING 归零 |

**最终状态（夹具实跑，exit 0）**：`control p50=0.021 / p95=0.025`，
`locked p50=0.029 / p95=0.382 / max=0.382`，`login_timing_warn_ge_500ms=0`。

**残留（未做，另评）**：p95 的 0.38s 就是"一次写尝试的等待"；要再下去需要**读写分离池**
（读连接永不被写占用）或把簿记写做成真正的异步队列（现在只是"不挡 actor + 快速失败"）。
另：`LOGIN_TIMING` 分段日志保留在代码里（正常 debug、慢于 500ms 才 warn），下次出现登录抖动可直接看分段。

**2026-09-25 复跑（master `e22d11c84` 构建，12 样本/阶段，写锁 20s）**：exit 0、L2/L3/L4 全绿 ——
`control p50=0.244 / p95=0.252`、`locked p50=0.236 / p95=0.257 / max=0.257`、
`login_timing_warn_ge_500ms=0`（阈值 1.0s；仍在"一次写尝试等待"的量级内）。

> 注意 `-Samples` **必须 ≥ 10**：判定里是 `n >= Max(10, Samples-2)`（p95 至少要 10 个样本才成立），
> 样本更少时报出来的是 `L2/L3=false`，读起来像"时延超标"——2026-09-25 已加前置守卫：`-Samples < 10`
> 直接 `exit 2` 并说明原因（本轮用 8 样本实测踩到过这个误导）。

## 5e. 高负载 tick 滞后长窗口：`tick_lag_probe.ps1`

CAPACITY.md §5 那条「20–50 会话下 tick 滞后待测」的收口工具：`capacity_ramp.ps1` 的 `-HoldSec` 默认
20s，比心跳间隔（300 tick × 100ms = 30s）还短，短窗口根本采不到样本；本工具把窗口拉到 ≥2 分钟。

判据（缺一不可）：**J-1** 8s 平均 CPU ≤ `MaxIdleCpuPct`（默认 20%，防"测的是别人的编译抢 CPU"；
忙机要测必须 `-AllowBusy` 并标注 CPU）/ **J0** 起服（`Gate listening`）/ **J1 载荷成立**
（`bot.ok == N` 且心跳 `online ≥ N`）/ **J2** 心跳条数 ≥ `MinHeartbeats`（默认 4）× **J3** `|lag_pct| ≤ MaxLagPct`（默认 5.0）。

```powershell
pwsh tools/ops/tick_lag_probe.ps1 -DeployDir <deploy> -StepsCsv '10,20,50' -HoldSec 150 `
     -OutFile tools/ops/out/tick_lag.json     # 退出码 0 过 / 1 判据红 / 2 前置失败
```

**绝不按进程名杀 `mir2_server`**（同机可能有别的 agent 的开发服 7000 与其它 deploy 实例，本工具只停
自己启动的 PID；`capacity_ramp.ps1` 自 2026-09-25 起同口径，门禁 `check_process_scope.ps1`）；
端口从 `<deploy>\config\server.toml` 的 `listen_addr` 读（读错端口会让机器人静默连不上，从而得到
"零滞后"的假绿）。

实测（2026-09-25，release 构建，同图 1912 只怪）：10/20/50 会话 × 150s → `lag_pct` ≤ 0.1%、
`interval_ms` 29974–30032、零丢包零踢线；对照（主机 95.6% CPU）`lag_pct` 0.2%——**判据对主机 CPU 抢占
不敏感，擅于发现"tick 自己变慢"**（见 CAPACITY.md §5.1 的边界说明）。

**活动负载（同图扇出）**：加 `-Activity walk|run -StepMs <ms>` 会让 `bot.py` 进图后按步频走位
（默认 600ms = C# `HumanObject` MoveDelay；服务端下限 50ms，低于它判 `Speed hack detected`）。
真玩家每成功一步都会触发 `UserLocation` + 向同图其它会话广播 `ObjectWalk`，所以这是**纯保持压不出来的
扇出路径**；探针按 opcode 聚合每会话**收到**的推送帧（28/29 = 别人移动、23 = 自己位置、24 = 新进视野），
并记 `Speed hack detected` 计数。用法与实测（20/50 人同图走位、零丢包零出箱满）见 CAPACITY.md §3.7。
（`run` 也实现了但**本轮未测**：它每步 2 格且带体力消耗/掉血，属另一条路径，别把 walk 的结论外推给它。）

```powershell
pwsh tools/ops/tick_lag_probe.ps1 -DeployDir <deploy> -StepsCsv '50' -HoldSec 150 `
     -Activity walk -StepMs 600 -OutFile tools/ops/out/fanout_walk50.json
```

## 6. 故障注入：`fault_injection.ps1`（+ `latency_proxy.py`）

> **2026-09-25 修（会误报的那条判据）**：抖动场景的判据是「会话可用 **且服务端零真错误**」，
> 而"真错误"由 `health_report.ps1` 的 `$benignPattern` 过滤良性断连。原模式只匹配 `read error`，
> **漏了 `write error`**——客户端被抖动代理掐断时服务端打的正是
> `Session 1 write error: … (os error 10054)`，于是被算成真错误、抖动项假红（exit 5）。
> 同一条还会让 `alert_probe.ps1` 把"玩家正常退出"报成告警（值班噪音）。
> 现在模式为 `(read|write) error.*(forced|强迫关闭|os error 10054|os error 10053)|Connection reset|Broken pipe`。
> 证据：① 分类器对照——合成日志里放 2 条 10054 断连 + 1 条 Broken pipe + 3 条真错误（`PERSIST_LOST`、
> `os error 10057`、`Failed to save player pets`）→ `errors=3`（良性被排除、真错误仍计数）；
> ② 告警噪音对照——只有断连的日志跑 `alert_probe` → `alert=false` exit 0（修复前会 exit 3）；
> ③ 重跑故障注入 → **exit 0，三个场景全 ok**（kill 感知 2.9s / 恢复 2s、抖动 `server_real_errors=0`、
> 只读库 `panicked=0`）。

| 场景 | 做法 | 判据 | 实测 |
|---|---|---|---|
| 杀进程 | 压测会话中途 kill -9 服务端 | 会话能感知断开 + 重启后可再登录 | 感知 2.3s、恢复 1s ✅ |
| 网络抖动 | 经代理注入 200ms 延迟 + 5% 丢包 | 会话仍可用 + 服务端零真错误 | 1/1 成功、0 真错误 ✅ |
| 存储只读 | `attrib +R` 锁 DB 后起服 | **不 panic**（可报错/降级） | 起服正常、0 panic ✅ |

诚实边界：存储只读这一项目前只证明「起服不炸」；**写入失败的降级行为**（登录写 is_online、
存档、邮件落库失败时是否吞错）尚未单独演练——下一轮补「运行中把 DB 置只读」的用例。

## 7. 告警接入最小集：`alert_probe.ps1`

把健康信号折成布尔告警（tick 抖动 / 真错误数 / 慢读者被踢 / 邮箱背压丢包 / RSS），
输出 `alerts.json` + **退出码**（0 无告警、3 有告警）供任务计划程序/CI 使用；可选 `-Webhook` 外送。

自测（本轮实做）：干净日志 → `alert=false, exit 0`；含 1 条 ERROR + 1 次邮箱丢包 →
`alert=true, reasons=['真错误 1 条 > 0','邮箱背压丢包 1 次'], exit 3` ✅

值班侧仍需外部资源：告警通道（IM/邮件/电话）、值班表、演练流程——仓库里没有，也不该编造。

## 7b. 演练脚本的两条工程护栏（2026-09-25 新增）

### 带超时的 bot 调用：`_run_bot.ps1`

`Invoke-BotJson -OpsDir <ops> -BotArgs @(…) -TimeoutSec 120 -Tag <名字>`：起 `bot.py`、**有界等待**、
超时即杀并返回 `timedOut=$true`（调用方按该次采样失败处理）。自检：`pwsh tools/ops/_run_bot.ps1 -SelfTest`
——用一个必然睡眠 30s 的子进程配 2s 超时，必须 `timedOut=True` 且实际用时 <10s（去掉超时机制这条自检会红）。

> 为什么要有它：`& python bot.py … | Select-Object -Last 1` 这种同步写法没有超时，只要被测服务端没绑到
> `-Port`（部署配置端口不一致）就会一直等 ⇒ **整个演练静默挂死**（`storage_degrade_drill` 实测挂了
> 6 分钟以上、无任何输出）。已迁移：`storage_degrade_drill` / `deploy_smoke` / `load_baseline` / `rollback_drill`。

### 静态门禁：`check_bot_timeouts.ps1`

扫 `tools/ops/*.ps1`，凡出现同步直调 `& python … bot.py` 就报出来；历史遗留的 8 个脚本列在
`-Allowlist` 里。**2026-09-25 更新：那 8 个已全部迁移完成，allowlist 已清空**（现在是
`fault_injection` / `leak_plateau` / `fresh_mirdb_deploy` / `migration_drill` / `capacity_ramp` /
`memory_cycle` / `memory_ramp` / `login_latency_probe` 全部走 `_run_bot.ps1`；其中三个 job 型脚本
在 **job 内部**也 dot-source 了 helper，父线程的 `Wait-Job -Timeout` 作第二层）。
新增脚本再这么写即红（阳性对照：往扫描目录放一个同步直调的假脚本 → `[违规]` + exit 1）；
`-Strict` 下也应为 0。

> 门禁只扫**代码行**、跳过注释行——注释里常写"原先 `& python bot.py …`"这种说明，
> 把它们算成违规会让门禁自己变噪音（第一版就踩到，实测 4 个脚本被自己的注释误伤）。

### 静态门禁：`check_process_scope.ps1`

同一类问题的另一半：**按进程名清场**。本机常态是多 agent 并行 —— 7000 上常驻一个开发服、
其它 agent 可能正在用实机客户端跑验收；演练若用 `Get-Process -Name mir2_server | Stop-Process -Force`
清场，会把**别人的**服务端一起带走，对方拿到的是假红（登录 `result=4` / 连不上），重试再多也修不了
（`LESSON_多agent并行时按进程名清进程会污染他人GUI实验`）。

```powershell
pwsh tools/ops/check_process_scope.ps1              # 0 无新增 / 1 有新增未迁移 / 2 前置失败或门禁自检失败
pwsh tools/ops/check_process_scope.ps1 -Strict      # allowlist 里的一起报红（全部迁移完后用）
```

扫描面 `tools/ops` + `tools/acceptance` + `scripts` 的 `*.ps1`，判据只认**共享名**（`mir2_server` /
`client_bevy`）与「`Get-Process … | Stop-Process`」管道，所以 `Stop-Process -Name <自己的唯一名>`
（LESSON 推荐的做法）与只做存在性探测的 `if (-not (Get-Process -Name mir2_server …)) { exit 9 }`
都不会被误判。**自带沙箱正/负对照**（4 个临时脚本：2 个乱杀必须被抓、2 个合规写法不许被误判），
判据空了直接 exit 2 —— 门禁自己不许假绿。

待迁移清单（本轮只记名、不静默放过）：`fault_injection.ps1`（杀服务端就是故障注入的目的）、
`l5y_reconnect.ps1`（断线重连要真杀服务端）、`run_real_e2e.ps1`（仓库级 harness 开跑前清场）。
2026-09-25 已改齐：`capacity_ramp.ps1`（清场 + RSS 取数都改成只认自己的 PID）、`login_latency_probe.ps1`
（原先按 `$ExePath` 过滤后仍按进程名杀，现改为**前置失败**让操作者处置残留，不再静默清理）。

### 端口契约（同 5c）

服务端读的是部署目录 `config/server.toml` 的 `[network].listen_addr`，而 `-Port` 只作用于 bot。
`deploy_smoke` 本来就改写副本配置；`storage_degrade_drill` / `rollback_drill` 现在也会在两者不一致时
生成临时配置并按 `mir2_server <config>` 启动（输出里写明用哪份）。

## 当前状态（2026-09-23）

七项（部署/回滚/压测/可观测性/容量/故障注入/告警）都有可运行脚本与实测证据
（见 walgit 线程 `crystal-ops-readiness`）。
**本机已做到**（2026-09-25 更新）：单机安全线**世界会话 ≥120 全绿零丢包**（`CAPACITY.md` §3.6）、
登录 200 并发真延迟 p95 0.86s（§2.1）、内存 20 会话 3.22MB/会话（§4.5）、高负载 tick 长窗口
10/20/50 会话 `lag_pct` ≤0.1%（§5.1）、运行中存储写故障降级演练 J1–J5 全过且落库失败对玩家可见（§5c）。
**仍未做（本机做不了/需外部资源）**：独立压测机与生产拓扑、真实渲染客户端容量、≥24h 长稳（本机最长
窗口 150s）、告警通道接入与值班流程、脱敏生产数据迁移演练——清单、判据与缺什么写在
`CAPACITY.md` §7 与 `EXTERNAL_OPS_HANDOFF.md`。
因此「可交付上线运营」仍**未**达：卡点是上列外部项，本机侧已推进到"最小集可跑、可复现、有判据"。
