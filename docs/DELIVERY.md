# Crystal 交付与运行说明（Rust 版：Client-Bevy + ServerRust）

本文面向「拿到产物要跑起来」的人：交付物是什么、还缺哪些资源、怎么起服、怎么验收。
开发细节见 [`Client-Bevy/README.md`](../Client-Bevy/README.md)、
[`Client-Bevy/docs/UI_COMPONENTS.md`](../Client-Bevy/docs/UI_COMPONENTS.md)、
[`ServerRust/docs/PORT_STATUS.md`](../ServerRust/docs/PORT_STATUS.md)。

---

## 1. 交付物

| 产物 | 说明 |
|---|---|
| `client_bevy`（Windows x64 / Linux x64 / macOS arm64） | 客户端（Bevy）。UI/交互以原版 C# 客户端为准 |
| `mir2_server`（同三平台） | 服务端（Rust actor 架构，SQLite **bundled** 构建，单文件可移植） |

产物由 CI 生成，见 [`.github/workflows/build.yml`](../.github/workflows/build.yml)：

- push `master` / PR → Actions artifacts（三平台 zip，可直接下载验证）；
- 打 tag `v*` → 自动发布 GitHub Release（六个 zip + release notes）。

Windows 客户端 zip 内已自动带上 MSYS2 UCRT64 运行库 DLL（`libpinyin`/`glib`/`libdb` 等），
解压即跑；从源码运行时才需要自行把 `D:\toolchains\msys64\ucrt64\bin` 加进 `PATH`。
macOS 产物未签名：首次打开被 Gatekeeper 拦截时右键 → 打开，或
`xattr -dr com.apple.quarantine client_bevy`。

---

## 2. 运行前置：必须自备的游戏数据

**仓库不含游戏素材与数据库**（体积大且版权归原版），需要从原 C# 版的数据包取得：

| 路径 | 内容 | 放置位置 |
|---|---|---|
| `Data/`（约 7.3 GB） | 客户端资源：图像库（`ChrSel`/`Prguse`/`Items`…）、`Sound/`、`Map/`、`Shaders/` | 与 `client_bevy` 可执行文件**同目录**（仓库根 `Data/` 或 `Client-Bevy/Data/` 均可，客户端自动解析） |
| `ServerRust/Daneo1989/` | 服务端地图数据 | 与 `mir2_server` 同目录（目录名须与 `config/server.toml` 的 `server.map_data_dir` 一致） |
| `ServerRust/Data/crystal.db` | SQLite 库：账号/角色/物品/邮件/行会… | 路径见 `config/server.toml` 的 `database.path` |

> 注意：配置里写的是小写 `data/crystal.db`，Windows 不区分大小写；**Linux 部署**请确保目录名
> 为小写 `data/`（或改配置）。

已入库的默认配置：`ServerRust/config/server.toml`（监听 `0.0.0.0:7000`、经验/掉落/稀有怪等开关）。

---

## 3. 首次部署（全新数据库）

1. **导入游戏配置**（物品/怪物/NPC/魔法/地图定义来自原版 `Server.MirDB`）：

   ```bash
   cd ServerRust
   cargo run --release --bin migrate_mirdb -- <path-to-Server.MirDB> Data/crystal.db
   ```

   旧格式 `.MirADB` 用 `migrate` 子命令（用法见 `ServerRust/src/bin/migrate.rs` 顶部注释）。

2. **启动一次服务端**：`init_db_pool` 会自动建表 + 补列（`schema_version` 迁移）。

3. **注册账号 / 建角色**：客户端登录界面走注册流程（真实服务端联调通过），
   或直接用 SQLite 建测试账号。自动回归脚本默认用 `test/123456`（角色 `bevychar`）
   与 `bevy2/123456`（角色 `bevy2char`）。

---

## 4. 启动

```bash
# 1) 服务端（工作目录必须是 ServerRust：db 路径是相对路径）
cd ServerRust
cargo run --release --bin mir2_server        # 或用发布包 ./mir2_server

# 2) 客户端：Client-Bevy/config.ini 里 UseMock=false、ServerAddr=127.0.0.1:7000
cd Client-Bevy
cargo run --bin client_bevy                  # 或用发布包 client_bevy.exe
```

`config.ini` 的 `UseMock=true` 时客户端完全离线（内置 mock 服务端），适合无服务端时看界面。

---

## 5. 验收与回归

### 5.1 门禁（离线，全绿为交付前提）

```bash
cd SharedRust  && cargo test                     # 187 lib + 11（另有 2 个 ignored）
cd ServerRust  && cargo test                     # 845 lib + 19 integration（gate_hardening 11 / no_blocking 2 / protocol_conformance 6；另有 3 个 ignored）
cd ServerRust  && cargo clippy --lib -- -D warnings   # 0 warning（CI 同口径）
cd Client-Bevy && cargo test                     # 794 lib + 7 bin + 2 + 53
```

三侧 `cargo fmt -- --check` 均须 0 差异。

（数字为 **2026-09-25 master `7cf7f770` 实测基线**，随批次增长；以 CI 与本地复跑为准。）

**ServerRust 测试并行度被刻意压到 2**（`ServerRust/.cargo/config.toml` 的 `[env] RUST_TEST_THREADS = "2"`）：
该 crate 的 e2e 用例每个都自带多线程 runtime + gate/world/social/account 全体 actor，
按默认「逻辑核数」并行（本机 12）会互相饿死，让「等某个包」的窗口超时 ⇒ **整套假红且每次红的用例都不同**
（实测：12 路并行 + 8 路外部 CPU 负载下 4/4 次红；压到 2 后同一条件 3/3 绿）。
代价是本地 `cargo test --lib` 由约 10s 变约 43s（不加载时）。快机上想跑满并行可显式覆盖：
`cargo test --lib -- --test-threads=8`（此时等待窗口也会按 `MIR2_TEST_WAIT_SCALE` 放宽，见
`ServerRust/src/actors/world/test_wait.rs`）。

### 5.2 真机 E2E（真实 ServerRust）

```powershell
pwsh scripts/run_real_e2e.ps1      # 默认用仓库内 debug 产物与测试库
```

脚本会：准备测试库（`scripts/e2e_setup_db.py`）→ 起服务端 → 跑用例 → 按客户端日志里的
`✅/❌` 标记判定 → 打印汇总 → 停服务端。用例与判定标记：

**当前基线（2026-09-14）：13/13 通过**（精炼为**全流程**判定，前置由脚本自动完成）。

脚本按「判定标记一出现就停客户端」跑（配合停客户端后等它登出落盘），所以不必等满每个用例的超时：
整批 13 个用例约 **6 分钟**（此前每个用例都白等 75s 超时，约 18 分钟）。

| 用例 | flag | 判定标记 |
|---|---|---|
| 钓鱼 | `--fishing-test` | `[FISHTEST] ✅ 收获消息` |
| 坐骑 | `--mount-test` | `[MOUNT] ✅ 下马成功` |
| 商城 | `--gameshop-test` | `[SHOPTEST] ✅ 完成（购买 #…）` |
| 排行榜 | `--ranking-test` | `[RANKTEST] ✅ 排行榜` |
| 精炼 | `--refine-test` | `[REFINETEST] ✅ 精炼已开始` + `[REFINETEST] ✅ 取回成功，精炼全流程完成` |
| 举报 | `--report-test` | `[REPORTTEST] ✅ 举报已提交确认` |
| 升级特效 | `--level-fx-test` | `[LEVELFX] ✅ PASS 升级生效` |
| 组队 / 私聊 / 邮件 / 交易 / 好友 / 婚姻 | 双客户端配对 | `[GROUPTEST]`/`[WHCHECK]`/`[MAILREAD]`/`[TRADETEST]`/`[FRIENDTEST]`/`[MARRY]` |

测试库准备脚本会（幂等）把测试角色摆到安全钓鱼点、复位「允许组队/交易/观察/结婚」开关、
补鱼竿的鱼钩与鱼饵、恢复精炼/交易所需物品——这些开关和消耗品会被用例改动并被服务端**存档**，
不复位会出现「上一轮跑通、下一轮偶发失败」。

**精炼用例的前置由 `scripts/e2e_refine_prep.py` 自动完成**，`run_real_e2e.ps1` 按
「开服前 `config` → 该用例前 `prepare-db` → 该用例后 `restore-db`」三段调用（也可手工
`prepare <db> <server.toml> <out.toml>` 一把跑），不再需要手工步骤：

- 角色摆到 246 图铁匠 `Blacksmith_Carlos` 旁（`CallNPC` 距离校验 ≤2 格）；
- 背包格 0 放可精炼武器（客户端脚本固定存入「背包第一件」）、`refine_log` 预置 3 件属性材料 + 1 块矿石
  （否则结算走「无 RefinedValue → 必碎」分支）；
- 生成**临时**服务端配置（`[refine] base_chance=100 / time_minutes=0`）并通过
  `mir2_server <config>` 启动参数启用 —— 仓库里的 `config/server.toml` **不被改动**；
- 结果用**轮询**查看（`[@REFINECHECK]` 每轮重试，首轮命中即走），不固定等 65s；
- 跑完 `restore-db`：角色回钓鱼点、清精炼状态并删掉测试武器（否则后续配对用例的同图摆位会错）。

### 5.3 交互巡回（40 窗；UI 改动合并前必跑）

离线测试只能证明「布局数值对」，证明不了「点得动」——#2953/#2955 那批缺陷（扩容钮吞掉
关闭钮点击、关闭钮漏挂标记、根节点没接显隐、仓库窗漏 StorageWidget）全是布局断言全绿而
交互失效。交互级验证由 `tools/acceptance/ui_interact_sweep.ps1` 承担：逐窗 `dialog open`
→ `dialog_rect` 定位标准关闭钮 → `click` 点它（真实 picking→Interaction 链路）→ 断言窗口
真关掉；设计上无关闭钮的窗改验 RPC open/close 往返；另有 inventory 拖动、NPC 会话窗 X、
hero_manage X 三段。

**它是门禁，不是报告：结论看退出码。**

```powershell
pwsh tools/acceptance/ui_interact_sweep.ps1 -ManageServer     # 自己起停服务端
pwsh scripts/run_real_e2e.ps1 -IncludeInteractSweep            # 常规用例 + 巡回，一把跑完（复用其服务端）
```

| 退出码 | 含义 |
|---|---|
| 0 | 全过（无 FAIL；SKIP 默认容忍，`-FailOnSkip` 时算失败） |
| 1 | 有用例 FAIL（点关闭钮没关掉 / 拖动没位移 / NPC·hero_manage 段失败 / 巡回到一半中断） |
| 2 | 前置不满足（缺产物、产物比源码旧、无 `Data/`、服务端未就绪、未进图） |

三条防「假绿」的前置，别绕：

- **产物按 `-RepoRoot` 解析**（默认 = 脚本所在检出；设了 `CARGO_TARGET_DIR` 时按它找）。
  在 worktree 里跑要传 worktree 路径——旧版本硬编码主检出绝对路径，会静默测另一份二进制。
- **产物比源码旧即 exit 2**（`-AllowStaleBinary` 跳过）：防止拿昨天的二进制跑出绿。
- **覆盖清单单一真源** `tools/acceptance/interact_sweep_manifest.json`：脚本按 `sweep` 逐窗跑；
  `excluded` 列「有 RPC 开关但走专用段/状态驱动」的窗口并写明理由。`Client-Bevy` 的
  `control.rs::interact_sweep_manifest_covers_all_rpc_kinds` 拿它与 RPC 窗口登记对账——
  新增窗口漏登记，`cargo test --lib` 直接红（跑在 CI 与本机门禁里）。

结果 JSON（`tools/acceptance/ui_interact_results.json`）：`gate.exit_code` 与进程退出码一致，
`gate.failures` / `gate.skips` 是逐条账本，`sweep` 是逐窗原始断言。

### 5.4 实机资源互斥锁（起客户端 / 登录 e2e 账号前**必须**先拿）

实机资源——客户端 + `test` 账号 + 本地服务端——一次只能有一组：多 agent 并行时谁先登录谁占住账号，
其余会拿到 `login 失败 result=4 密码错误`（服务端日志实为 `Account already online`，见 §7 排障）。
这是**资源互斥假红，不是产品缺陷**；`for ($i=1..8) { 跑夹具; sleep 60 }` 去撞「干净窗口」
只会把交付时间耗在等待上。

- **锁**：`tools/acceptance/e2e_lock.ps1`；锁文件 `%TEMP%\crystal_e2e_client_test.lock`
  （跨 worktree、跨 agent 全局唯一，不是每个 worktree 一把）。
- **用法**：`param()` 之后 dot-source 再 `Enter-E2eLock -ScriptName '<夹具名>'`；拿不到（返回 `$false`）→ `exit 2`。
  每个入口都**整段包 `try/finally` + `Exit-E2eLock`**（PowerShell 的 `finally` 在 `exit` 下也会执行，
  所以早退分支如 `if (...) { exit 5 }` 也会释放——实测缺 exe 的早退路径 `exit 2` 后锁文件即被删除）。
  万一漏了释放也不会卡住队列：进程一退出，下一个调用者按「持有者 PID 已死」立刻接管。
- **嵌套**：持锁进程设 `CRYSTAL_E2E_LOCK_HELD_BY`，子脚本（`run_real_e2e.ps1` → `ui_interact_sweep.ps1`）
  自动复用同一把锁，不会自锁。该标记会被**复核**（父进程仍活着 + 锁文件确实由它持有），
  残留标记不生效。
- **自证**：`pwsh tools/acceptance/e2e_lock_selftest.ps1` —— 覆盖获取/释放、争用排队不抢、
  僵尸回收、PID 复用、超龄回收、读不全宽限、继承标记复核、**跨进程串行性**（3 个子进程抢同一把锁、
  临界区不许重叠）；秒级，不起客户端。它把 `TEMP` 指向临时目录后再 dot-source，**不会碰真实锁**
  （跑在真实锁上会与别的 agent 的实机任务互相污染）；退出码 `0` 全过 / `1` 有用例红 / `2` 前置失败。
- **覆盖**：`tools/acceptance/` 下全部会起客户端的夹具 + `scripts/run_real_e2e.ps1` 都已接入（#3129 铺齐），
  且**由门禁钉住**：自证的 `T9.1` 会扫出「会起客户端」的脚本（判据＝正文出现
  `--e2e-user` / `client_bevy.exe` / `--real-net` / `--auto-enter` 任一），`T9.2`/`T9.2b` 要求
  **dot-source + `Enter` + `Exit` 三件齐全**（缺 Exit 的早退路径会把锁留到下一次进入才发现要回收），
  `T9.3` 是阳性对照（临时造一个没接入的脚本必须被判不合规），`T9.4` 钉「自检自带判据与锁脚本里的
  `Get-E2eClientScripts` 识别同一批脚本」防两处漂移，`T9.5` 钉「批量接入器与门禁同口径」（dry-run 需为 0）。
  新增夹具忘了接入时：`pwsh tools/acceptance/enroll_e2e_lock.ps1 -Apply`（幂等，只插不删），
  或照抄已接入夹具的写法。**注意** `.gitignore` 对 `tools/acceptance/` 是白名单式管理——新脚本必须补
  `!tools/acceptance/<名字>` 一行，否则 `git add` 会被静默忽略、门禁也看不到它。

---

## 6. 已知差异与限制（相对原版 C#）

- **数据不入库**：需要自备 `Data/`、`Daneo1989/`、`crystal.db`（见 §2）。
- **协议以 Rust 客户端 + 服务端自洽为准**，不保证与 C# 线格式逐字节一致。
- **有意简化**（不阻塞交付，逐条有理由）见 `Client-Bevy/docs/UI_COMPONENTS.md` §7
  与本文档所在批次记录：原版死控件/资源包缺图/协议自洽偏离项。
- **鼠标自动化限制**：无焦点窗口里鼠标事件到不了 winit（本机 `SetForegroundWindow` 返回 0），
  因此依赖真实鼠标的交互由「真实实体 + 真实系统」的行为级单测覆盖，真机用例走 RPC 等价入口。
- macOS 产物未签名。

---

## 7. 排障

| 现象 | 原因 / 处理 |
|---|---|
| 客户端秒退、错误码 `0xC0000135` | 缺 MSYS2 UCRT64 运行库 DLL。用发布包（已自带），或把 `msys64/ucrt64/bin` 加进 `PATH` |
| 客户端黑屏 / 缺图 / 中文变方框 | `Data/` 不在可执行文件同目录（或路径不对） |
| 服务端报 `map file not found` | `Daneo1989/` 缺失或与 `server.map_data_dir` 不一致 |
| 登录提示「密码错误」但密码没错 | 该账号**已在线**（服务端拒绝重复登录，C# 同语义）。等前一个连接断开（或重启服务端）再登 |
| `refine-test` 报「未收到 NPCRefine / 未收到精炼结果」 | 前置没做：角色不在铁匠旁（`CallNPC` 距离 ≤2 格）或没跑 `scripts/e2e_refine_prep.py prepare`（`run_real_e2e.ps1` 会自动做） |
| 端口被占用 | `config/server.toml` 的 `network.listen_addr`；客户端同步改 `config.ini` 的 `ServerAddr` |
| 角色存档异常/数据回退 | 看服务端日志是否有 `Failed to save player …`（存档是单事务，任何一步失败整档回滚）；实例见 #2879（好友主键冲突，已修） |
