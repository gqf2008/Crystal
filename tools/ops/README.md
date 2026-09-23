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

## 当前状态（2026-09-23）

四项都有可运行脚本与一次实测证据（见 walgit 线程 `crystal-ops-readiness`）。
**尚未做**：独立压测机、真实生产拓扑、故障注入（杀进程/网络抖动）、告警接入。
因此「可交付上线运营」仍**未**达；本目录把它从"完全空白"推进到"最小集可跑、可复现、有判据"。
