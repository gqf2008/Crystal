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

Windows 客户端 zip 内已自动带上 MSYS2 UCRT64 运行库 DLL（`glib`/`libdb`/`libiconv`/`libintl`/`libpcre2`/
`libstdc++`/`libgcc`/`libwinpthread` 共 8 个，由 objdump 按导入闭包解析）、`assets/` 与 `libpinyin/{data,conf}`，
解压即跑；从源码运行时才需要自行把 `D:\toolchains\msys64\ucrt64\bin` 加进 `PATH`。
> **更正与实测（2026-09-25）**：上文原先把 `libpinyin` 与 glib/libdb 并列写成"DLL"，属**措辞不准**——
> 本端把 libpinyin 以 `libpinyin.a` **静态链接**进 exe（导入表里没有 pinyin*），随包带的是它的
> **数据与配置**（缺了才会退化为禁用 IME）。其余依赖里 `api-ms-win-*`/`combase`/`pdh`/`powrprof`/
> `uiautomationcore` 等由 Windows 自带（CI 配方本就不分发）。
>
> **本机可复跑的打包预演**（发版日关键路径，防止"等到打 tag 才发现配方漂移"）：
> ```powershell
> # ① 客户端：先构建 GNU release 产物（CI 同款目标）
> $env:CARGO_BUILD_TARGET='x86_64-pc-windows-gnu'; cargo build --release --bin client_bevy
> pwsh tools/ops/package_windows_rehearsal.ps1 -SmokeServer 127.0.0.1:7000
>
> # ② 服务端：release 产物 + 全新目录部署 + 协议级登录冒烟（判据 J1 产物哈希 / J2 起服就绪 / J3 登录）
> cargo build --release --bin mir2_server        # 在 ServerRust/ 下跑
> pwsh tools/ops/deploy_smoke.ps1 -ReleaseDir ServerRust/target/release -SourceRoot ServerRust -Port 7100
> #   -SourceRoot 必须是"含 config/ + Data/(DB) + Daneo1989/"的目录：worktree 检出里 Data/crystal.db
> #   是 gitignore 的（不存在），这种情况指向手边**有可用数据**的那份检出；-Port 别用 7000（会撞开发服）。
>
> # ③ 内存泄漏门禁（发版前必跑；判据 = 测量窗口内活跃字节"一次都没回落"即判泄漏）
> pwsh tools/ops/mem_leak_gate.ps1 -DeployDir %TEMP%\ramp_deploy -MeasureCycles 6
> #   没跑出 J5（被测二进制没带 --features mem-probe）会 exit 3「没判成」，不许当通过；
> #   部署目录不存在会 exit 2；其 config/server.toml 里的端口必须与 -Port 一致
> #   （不一致 = 登录全失败、J0 标定无效 + exit 3，不静默给绿）；
> #   只想验判据本身、不起服：pwsh tools/ops/mem_leak_gate.ps1 -SelfTest
> ```
> 判据 J1 产物 / J2 staging（8 DLL + assets + libpinyin 数据）/ **J2b 依赖闭包**（stage 内每个 PE 的导入
> 要么在 stage、要么 System32 或 `api-ms-win-*` 虚拟 api-set）/ J3 zip 结构断言 / **J4 解压到干净目录真启动**。
> **2026-09-25 按上面三条从零复跑（同一台机器，master `8ec2b58d5`）**：
> 客户端 GNU release 构建 **21m12s** exit 0；打包预演 exit 0 —— zip **66.1 MiB / 38 条目** / 依赖闭包 0 缺失 /
> 解压后 `alive=true, control_rpc=true, **entered_game=true**`（发布产物不仅能起来，还能连上服务器登录并进图）；
> 服务端 release 构建 **1m38s**、`deploy_smoke` exit 0 —— 部署目录里 exe 的 sha256 与刚构建的产物一致
> （`F0597E9D…`，证明跑的是本次产物而不是旧物）、日志 `Gate listening on 0.0.0.0:7480`、协议级登录
> **ok=1 / failed=0**；内存门禁 exit 0 —— 20 会话 × 6 测量轮、活跃字节每轮净增 **+30,050 B**、`dips=1`（无泄漏）。
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

  > **✅ 2026-09-25 实测：`.MirADB` 迁移对当前发布数据（v112）已可用**（此前从没验证过，实跑发现整条链路是坏的）。
  > 用真实 `Server.MirADB`（20455 字节、v112、3 账号 8 角色）跑通，判据是**读到的位置恰好等于文件大小**：
  >
  > ```
  > Accounts migrated: 3 / Characters migrated: 8 / Errors: 0
  > File consumed: 20455 / 20455 bytes（完全对齐）
  > Verification: 3 accounts, 8 characters, 13 backpack items in DB
  > ```
  >
  > 修掉的缺陷（都对着 C# 权威源码核过）：① Windows 下 DB URL 拼错（`sqlite://C:\…` → `code 14`，且 sqlx 不建目录/文件）；
  > ②三处「格子是否有物品」标志判反（C# `AccountInfo.Save:242-248`、`CharacterInfo.Save:434-455` 都是
  > `writer.Write(X != null); if (X == null) continue;` ⇒ **true 才跟一个 `UserItem`**）；
  > ③ `UserItem.Slots` 的极性**与②相反**（C# `ItemData.cs:397-402/479`：`true` 表示该孔为空）；
  > ④ `UserItem` 的 `Awake`（`Type u8 + count i32 + count×u8`）、`ExpireInfo`（单个 i64）、
  > `RentalInformation`（string+i16+i64+bool）、`SealedInfo`（i64，v>92 再加 i64）四处结构与工具不符；
  > ⑤ `UserMagic`（`Spell u8+Level u8+Key u8+Experience u16+IsTemp+CastTime i64`）；
  > ⑥ 任务进度表按 v≥90 的**非 orphan** 布局（每个 kill/item 只写一个 i32、flag 写一个 bool）；
  > ⑦ `RentedItems`（`u64+string+string+i64`）；⑧ 角色 `Flags` 的真实长度是 `Globals.FlagIndexCount = 1999`（工具写死 256，每条角色少读 1743 字节）；
  > ⑨ 账号/角色之后的收尾段（`NextAuctionID u64` → Auctions → `NextMailID u64` → GameshopLog → SavedSpawns）此前完全没读。
  >
  > **密码说明（重要）**：C# `Crypto.HashPassword` 是 `Encoding.UTF8.GetString(pbkdf2_GetBytes(24))`
  > ——24 字节哈希被当成 UTF-8 字符串存盘，非法序列会被替换成 U+FFFD（实测三个账号该字段是 52/40/42 字节，
  > 原始哈希已不可还原）。服务端 `verify_password` 现已**复刻这一步损失转换**，所以迁移过来的账号
  > 仍可用**原密码**登录（首次登录后自动改写为 Argon2）。这一条由单测覆盖并做了阳性对照。
  >
  > 排查同类错位用 `MIR2_MIGRATE_TRACE=1`（逐条打印 `i32/str` 的**位置与长度**，并标出段边界），
  > 比只看最后一句 `failed to fill whole buffer` 有效得多。
  >
  > **一条命令跑完整迁移演练**（对应第 7 张外部项「脱敏生产数据迁移演练」的本机版；判据 J1–J5 =
  > 备份含哈希 / 两条迁移 / 起服+首启导入 / 抽样比对+真登录进图 / 回滚）：
  >
  > ```powershell
  > pwsh tools/ops/migration_drill.ps1 -MirDb <Server.MirDB> -MirAdb <Server.MirADB> `
  >      -ServerDir ServerRust/target/release -DataRoot <含 Daneo1989 与 config/ 的目录> -Port 7600
  > ```
  >
  > 实测（原版 v112 数据）：**PASS J1–J5** —— 3 账号/8 角色/13 背包物品迁入 → 起服就绪 + 首启导入
  > → 用迁移账号**真登录并进图**（20 帧）且首登后哈希自动升级为 `$argon2id$` → 回滚后库哈希与备份一致、再登录成功。
  > 演练还顺带暴露并修掉一个 schema 缺陷：迁移工具原先自建一套 `characters` 表（缺 `class/gender/hair` 等列），
  > `IF NOT EXISTS` 让服务端建表变空操作 ⇒ 「登录能过、角色列表查不出（`no such column: class`）、进不了图」。
  > 现在迁移工具先调用**服务端自己的** `db::init_db_pool` 建表（单一真源），再写入。

   > **版本支持与核验（2026-09-25）**：工具原先只实现 **≤84** 的旧布局，拿当前发布数据
   > （`Server.MirDB` **version 112**）实跑会失败——物品段读到第 607/1628 个就 EOF、之后各段全空
   > （exit 1、990 条 parse error，落库 items 384 / monsters 0 / npcs 0）。现已按
   > `Shared/Data/ItemData.cs` 与 `Server/MirDatabase/MonsterInfo.cs` 的版本分支补齐
   > **v>84 布局**（`StackSize` u16、自描述 `Stats` 块 `int count + count×(u8 stat, i32 value)`、`Slots`），
   > 并给每段条目数加了「不合理即失败」的上限检查（防止错位后 `for 0..18 亿` 空转）。
   >
   > **一条命令核验整条路径**（J1 迁移干净 / J2 定义表行数 / J3 起服+首启导入 / J4 新玩家全路径冒烟 / J5 库里真有账号角色）：
   >
   > ```powershell
   > pwsh tools/ops/fresh_mirdb_deploy.ps1 -MirDb <path-to-Server.MirDB> `
   >      -ServerDir ServerRust/target/release -DataRoot <含 Daneo1989 与 config/ 的目录> -Port 7500
   > ```
   >
   > 实测（原版 `Server.MirDB` v112）：maps **463** / items **1628** / monsters **555** / NPCs **375** /
   > quests **154** / magics **109**；首启自动导入 **21688** 条掉落、**796** 页 NPC 脚本、**1124** 条商品、recipes，
   > 零 `ERROR`；新玩家全路径（注册→登录→建角→进图）收到 **5222** 帧世界数据。
   > 阳性对照：把 v>84 分支关掉 → **J1 立即红**（exit 1、990 parse error、items 638）。

2. **启动一次服务端**：`init_db_pool` 会自动建表 + 补列（`schema_version` 迁移）。

   首启还会**自动导入**运行期数据（表为空才导）：掉落（`Envir/Drops`）、NPC 脚本页（`Envir/NPCs`）、
   商品（`Envir/*.txt`）、配方（`Envir/Recipe`）。所以第 2 步之后 `npc_goods`/`monster_drops`/`npc_scripts`
   才非空——这是预期，不是漏导。

3. **注册账号 / 建角色**：客户端登录界面走注册流程（真实服务端联调通过），
   或直接用 SQLite 建测试账号。自动回归脚本默认用 `test/123456`（角色 `bevychar`）
   与 `bevy2/123456`（角色 `bevy2char`）。

---

## 4. 启动

```bash
# 1) 服务端（工作目录必须是 ServerRust：db 路径默认是相对路径）
cd ServerRust
cargo run --release --bin mir2_server        # 或用发布包 ./mir2_server

# 2) 客户端：Client-Bevy/config.ini 里 UseMock=false、ServerAddr=127.0.0.1:7000
cd Client-Bevy
cargo run --bin client_bevy                  # 或用发布包 client_bevy.exe
```

> 任务脚本与 NPC 脚本目录**跟随 `[server].map_data_dir`**（`<map_data_dir>/Envir/Quests`、`<map_data_dir>/Envir/NPCs`）。
> 工作目录只影响 `[database].path` 这类相对路径；数据根不对时启动日志会给出 `quest_dir 不存在` 与
> `任务奖励全部为 0` 两条告警（见 §7）。

`config.ini` 的 `UseMock=true` 时客户端完全离线（内置 mock 服务端），适合无服务端时看界面。

---

## 5. 验收与回归

### 5.1 门禁（离线，全绿为交付前提）

```bash
cd SharedRust  && cargo test                     # 187 lib + 11（另有 2 个 ignored）
cd ServerRust  && cargo test                     # 849 lib + 19 integration（gate_hardening 11 / no_blocking 2 / protocol_conformance 6；另有 3 个 ignored）
cd ServerRust  && cargo clippy --lib -- -D warnings   # 0 warning（CI 同口径）
cd Client-Bevy && cargo test                     # 794 lib + 7 bin + 2 + 53
```

三侧 `cargo fmt -- --check` 均须 0 差异。

另有四条**离线的对账/卫生门禁**（秒级，不起客户端）：

```bash
pwsh tools/acceptance/e2e_lock_selftest.ps1      # 实机资源锁自证（§5.4）
pwsh tools/acceptance/flag_coverage_check.ps1    # 客户端 auto 开关覆盖清单对账
pwsh tools/ops/check_process_scope.ps1           # ops/验收脚本不得按进程名清共享资源
pwsh tools/ops/check_bot_timeouts.ps1            # ops 演练不得有同步无超时的 bot 调用
```

后两条是**运营工具自身的卫生门禁**（本机常态多 agent 并行：7000 上常驻开发服）：前者禁止
`Get-Process -Name mir2_server | Stop-Process -Force` 这类按进程名清场（会把别人正在跑的验收
打成假红，见 `LESSON_多agent并行时按进程名清进程会污染他人GUI实验`），后者禁止"同步无超时的
bot 调用"（端口不一致时会静默挂死）。两条都自带阳性对照，且各有 allowlist 作为待迁移清单。

`flag_coverage_check.ps1` 把「哪些 `--xxx-test` 开关进发版门禁、哪些只是历史探针」钉在
`tools/acceptance/CLIENT_AUTO_FLAGS.md` 上：源码新增/删除开关、或清单把某个开关标成 `gate`
而门禁脚本里其实没跑它，门禁都会红（两条阳性对照均已实做：注入假开关 → C3 红；桶标记改成
`gate` 但脚本没跑 → C5 红）。

（数字为 **2026-09-25 晚 master `45e378aa` 实测基线**，随批次增长；以 CI 与本地复跑为准。
该次复跑逐条实跑：SharedRust `187 lib + 11`（2 ignored）、ServerRust `849 lib + 19 integration`、
ServerRust `clippy --lib -- -D warnings` 0 warning、Client-Bevy `794 lib + 7 bin + 2 + 53`，
三侧 `cargo fmt -- --check` 均 0 差异。ServerRust lib 从 `7cf7f770` 时的 845 涨到 849，
增量来自任务数据根修复（#3165）新增的 3 条单测与同期其它批次。）

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

**当前基线（2026-09-25 晚，master `8f000adac` + `--fishing-test` 空杆重抛修复）：14/14 通过**
（精炼为**全流程**判定、钓鱼为**咬钩后必须有收竿消息**判定，前置由脚本自动完成）；
同一次运行里的「40 窗交互巡回」也是 43/43。上一次刷新的基线是 2026-09-14——中间隔了 11 天、大量改动落地，
那一次复跑暴露并修掉了三处问题（两处是**夹具前置**，一处是**真缺陷**）：

> **钓鱼用例（`--fishing-test`）此前是概率性假红**：服务端整轮没有鱼咬钩时**静默收竿**
> （C# `PlayerObject.UpdateFish` 同样不 Enqueue 结果消息，`Server/MirObjects/PlayerObject.cs:11199-11231`），
> 而夹具到点没看到聊天消息就判失败——实测单跑 4 次红 1 次（≈25%）。现在按 C# 语义区分两种「没消息」：
> **咬过钩却没消息 = 收竿链断了 → 仍判失败**；整轮没咬钩 → 重抛再等（最多 5 次，≈3e-4 才误报）。
> 两个阳性对照都实做过：强制不咬钩 → 连抛 5 次后硬判失败；屏蔽收竿消息 → 判「咬钩后未收到收竿消息」。

1. **服务端 `S.ReceiveMail` 列表条目包漏写附件**（真缺陷，已修）：客户端在 #3120 ① 已按
   `item_count` 逐条读 `{uid,idx,image,name,count,dura,max}`，而服务端的条目包**只写计数不写条目**
   ⇒ 任何带附件的邮件（商城购买、拍卖成交、邮件附件）在客户端一律 `⚠️ ReceiveMail 解析失败`、
   条目被静默丢弃（实测：商城购买金币已扣、邮件已进玩家邮箱，但客户端列表看不到）。
   修法：条目包补写附件，并把 `world/mod.rs` 里那份**重复实现**改为转发到
   `social_packets` 的单一真源（同一函数原来有两份，正是「客户端补了读侧、服务端那份没跟」的成因）。
2. **fishing-test 前置判据写错**（夹具）：`e2e_setup_db.py` 原来用「武器/坐骑槽**行数** < 2」决定是否补装备，
   而开发库里武器槽已被换成剑（行数已是 2）⇒ 整段恢复被静默跳过 ⇒ 鱼竿没装上、抛竿被服务端正确拒绝
   （「你需要装备鱼竿才能钓鱼」）⇒ 假红。已改为按**槽内物品身份**判断（不是鱼竿就换回鱼竿）。
3. **gameshop-test 选了买不了的商品**（夹具）：原自动化「按金币价挑、却发 `p_type=0`（信用点）」，
   且原版 MirDB 里 `CanBuyGold/CanBuyCredit` **默认全 false**（C# 服务端 `PlayerObject.cs:13794/13803`
   与客户端 `MirGameShopCell.cs:195/199` 都按它判定 ⇒ 那批数据在原版里也买不了）。
   已在 `e2e_setup_db.py` 里给「最便宜、库存不限」的一件打上 `can_buy_gold=1` 作为前置，
   自动化改为按 `can_buy_gold` 挑并发 `p_type=1`。

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
  **当前基线（2026-09-25 扩扫描面后复跑）：37 passed / 0 failed**（含 T9.1b 盲区回归锁、
  T9.3/T9.3b/T9.3c 三条阳性对照、T9.3d/T9.3e 两条阴性对照、T9.4/T9.4b/T9.5/T9.6 防漂移、T10 语法解析）。
- **覆盖**：**整仓** `*.ps1` / `*.bat` / `*.cmd`（排除 `.git`/`target`/`node_modules`）里全部会起客户端的脚本都已接入
  （#3129 铺齐 acceptance/scripts，2026-09-25 起扫描面改为整仓——`tools/ops/package_windows_rehearsal.ps1`
  这类新目录里的实机入口此前扫不到，漏锁也不会红，属假绿盲区）。
  **扩展名也在覆盖范围内**（2026-09-25 补）：只认 `*.ps1` 时，一个
  `launch_client.bat`（`start "" client_bevy.exe --e2e-user ...`）能**整体绕开锁与门禁**——
  实测旧判据对它返回 0 条，而资源互斥假红照旧发生；`*.bat`/`*.cmd` 没有 dot-source 的写法，
  所以这一档的接入判据是「正文点名 `e2e_lock`，或点名调用某个**已接入的**入口（把拿锁委托出去）」，
  接入器对这种文件只报「需人工接入」、不会去改它的正文。**包装入口**（正文无客户端字样、但
  调用某个会起客户端的脚本，例如 `call pwsh -File ...\l5h_buy_item.ps1`）同样纳入扫描面，
  且只认调用形态（`call`/`&`/`pwsh`/`Start-Process`/`-File`）——只「提到」入口名的静态门禁
  （`flag_coverage_check.ps1` 把 `run_real_e2e.ps1` 当默认参数值）不算入口，免得凭空变红。
  覆盖面**由门禁钉住**：自证的 `T9.1` 会扫出「会起客户端」的脚本（判据＝正文出现
  `--e2e-user` / `client_bevy.exe` / `--real-net` / `--auto-enter` 任一），`T9.2`/`T9.2b` 要求
  **dot-source + `Enter` + `Exit` 三件齐全**（缺 Exit 的早退路径会把锁留到下一次进入才发现要回收），
  `T9.3` 是阳性对照（临时造一个没接入的脚本必须被判不合规），`T9.3b` 是**盲区阳性对照**
  （把没接入的脚本放进旧清单之外的目录也必须被认出），`T9.3c` 是**扩展名盲区阳性对照**
  （`.bat` 起客户端必须被认出且判不合规；拿旧版锁脚本 A/B 跑它立刻变红，就是「去掉守卫应立红」），
  `T9.3d`/`T9.3e` 是两条阴性对照（委托拿锁的 `.bat` 不许假红、只提到入口名不算入口），
  `T9.1b` 是目录盲区的回归锁，`T9.4`/`T9.4b` 钉「自检自带判据与锁脚本里的
  `Get-E2eClientScripts` 识别同一批脚本（含扩展面与委托判定）」防两处漂移，
  `T9.5` 钉「批量接入器与门禁同口径」（dry-run 需为 0），
  `T9.6` 钉「接入器不会去改非 PowerShell 启动器（只报需人工接入）」。
  新增夹具忘了接入时：`pwsh tools/acceptance/enroll_e2e_lock.ps1 -Apply`（幂等，只插不删），
  或照抄已接入夹具的写法。**注意** `.gitignore` 对 `tools/acceptance/` 是白名单式管理——新脚本必须补
  `!tools/acceptance/<名字>` 一行，否则 `git add` 会被静默忽略、门禁也看不到它。

### 5.5 服务端重启 / 断线重连（`l5y_reconnect.ps1`）

#### 5.5.1 **共享 e2e 开发服**（本机 7000）的安全重启

本机多个 agent / 夹具都连同一个 7000 开发服，它的重启**必须用受版本管理的工具**，
不许再跑 `%TEMP%` 里那种临时脚本——那些脚本有两个会让证据失真的坑（2026-09-25 实名踩到）：

```powershell
# 默认从"本脚本所在检出"取产物；也可以显式指源（例：指向主检出）
pwsh tools/ops/restart_e2e_server.ps1 -SourceRoot <含 origin/master 的检出>

pwsh tools/ops/restart_e2e_server.ps1 -SelfTest     # 判据自检（不起服，秒级）
```

它把两条硬前置做进脚本：

1. **来源必须"含 origin/master"**：先 `git fetch`，再要求源检出 `HEAD` 不落后 `origin/master`
   （`merge-base --is-ancestor origin/master HEAD`）；不满足直接 `exit 2` 并打印补救命令。
   起因：`%TEMP%` 那个临时脚本从某个 detached 在旧提交的 worktree 拷 exe，
   **每重启一次就把已合并的修复整体回滚**（11:47 UTC 顶掉修复版、20:27 UTC 才恢复），
   而那之后所有连 7000 的实机结论都在测旧代码。真想放行落后版本要写 `-AllowBehindMaster`（会告警并留痕）。
2. **只接管"端口持有者，且它就是我们这份"**：按**端口**找进程（绝不按进程名清场），
   且要求它的 exe 路径就是本部署目录里那份；否则 `exit 2` 并打印占用者。
   起因：旧临时脚本 `Get-CimInstance … Name='mir2_server.exe' | Stop-Process` 会连带杀掉
   别人的演练/验收（对方拿到 `result=4 密码错误` 这类假红）。

重启成功会写 `<DeployDir>\server_build_record.json`（源检出 / 源 HEAD / origin/master HEAD /
profile / exe sha256 / pid / port / 时间）——"运行中的服务端是哪一份代码"从此有据可查。
**验收判据**：重启后该记录里的 `source_head` 必须**包含 `origin/master`**（`source_contains_master=true`）。

```powershell
pwsh tools/acceptance/l5y_reconnect.ps1 -ServerWorkDir %TEMP%\e2e_workdir
```

场景是上线运营真实会发生的事：玩家在游戏中 → 服务端重启（发布/回滚/崩溃恢复）或链路闪断 →
客户端应**自动重连并重新进入游戏**，不需要玩家手点。夹具把这条链路做成可复跑判据：
① 起服务端与客户端（客户端带 `--reconnect-test`）并等它进图；② **杀掉**服务端、
`-OutageSec`（默认 2）秒后把同一份服务端起回来；③ 断言客户端日志依次出现「检测到断线」与
「✅ 自动重连成功并重新进入游戏」。判据只取客户端 `[RECON]` 行，**不断言重连耗时**（客户端是指数退避
2s→4s→…→30s 无限重试，耗时与负载/关机速度有关）。

**当前基线（2026-09-25 晚 master `8ac314e88`）：PASS**——停机 2s，客户端在服务端回来后约 2.3s 内
完成「重连 → 重新登录 → 自动进入角色 → 回到游戏」（实测时间线：`04:35:46` 进图、
`04:35:48` 检测到断线、`04:35:49.8` 重连成功并重发登录、`04:35:50.3` 回到游戏），
`VERDICT enter_game=PASS saw_disconnect=PASS auto_reconnect=PASS`，退出码 0。

覆盖口径：这条场景此前只有客户端里的 `--reconnect-test` 探针（没有 runner、不算覆盖证据）；
现在它由本夹具承接，并在 `tools/acceptance/CLIENT_AUTO_FLAGS.md` 里从 `probe` 桶升到 `fixture` 桶
（覆盖清单门禁会校验「夹具文件存在且真的引用了该开关」）。

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
| 交任务没奖励 / 任务对话为空 / 任务列表空 | 任务与 NPC 脚本目录 = `<server.map_data_dir>/Envir/{Quests,NPCs}`；数据根不对时启动日志有 `quest_dir 不存在` 与 `任务奖励全部为 0` 告警（正常为 `Resolved N kill tasks…` + `Quest rewards resolved: x/y`） |
| 登录提示「密码错误」但密码没错 | 该账号**已在线**（服务端拒绝重复登录，C# 同语义）。等前一个连接断开（或重启服务端）再登 |
| `refine-test` 报「未收到 NPCRefine / 未收到精炼结果」 | 前置没做：角色不在铁匠旁（`CallNPC` 距离 ≤2 格）或没跑 `scripts/e2e_refine_prep.py prepare`（`run_real_e2e.ps1` 会自动做） |
| 端口被占用 | `config/server.toml` 的 `network.listen_addr`；客户端同步改 `config.ini` 的 `ServerAddr` |
| 角色存档异常/数据回退 | 看服务端日志是否有 `Failed to save player …`（存档是单事务，任何一步失败整档回滚）；实例见 #2879（好友主键冲突，已修） |
