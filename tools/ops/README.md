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
pwsh tools/ops/rollback_drill.ps1 -NewBinary <new.exe> -PrevBinary <prev.exe> -ServerDir <运行目录>
```

**回滚要验的不是"能起来"，而是"起来之后玩家的档还在"**：二进制回退 + DB 结构兼容（本仓表创建走
`IF NOT EXISTS`，但字段语义变化不在守卫内）。抽到的快照字段是 `characters.gold/level/map_index`。

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

与 `capacity_ramp.ps1` 的区别：**只按 PID 管控自己启动的服务端**（不按进程名 `Stop-Process`），
所以可以在同一台机器上保留别的服务端实例；采样每 2s 一次、取保持期最大值（`Process` 属性必须
`Refresh()`，否则整轮都读到同一个陈旧值——踩过）。

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

判据（J1–J4，缺一不可）：写失败后**进程仍在** / 失败被**明确记录** / 故障期间**读路径不受影响**
（新会话仍能登录）/ 释放锁后**恢复**（新会话下线重新出现 `saved to database on logout`）。

**2026-09-23 实测结果：J1–J4 全绿**（`ok=true`，`save_failure_lines=2`）。同一份日志里的时间线：

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

## 6. 故障注入：`fault_injection.ps1`（+ `latency_proxy.py`）

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

## 当前状态（2026-09-23）

七项（部署/回滚/压测/可观测性/容量/故障注入/告警）都有可运行脚本与实测证据
（见 walgit 线程 `crystal-ops-readiness`）。
**尚未做**：独立压测机与生产拓扑、真实客户端容量、≥24h 长稳、运行中存储故障降级、
告警通道接入与值班流程、脱敏生产数据迁移演练——清单与理由写在 `CAPACITY.md` §7。
因此「可交付上线运营」仍**未**达；本目录把它从"完全空白"推进到"最小集可跑、可复现、有判据，
且已量化单机安全线（世界会话约 10）"。
