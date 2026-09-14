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
cd SharedRust  && cargo test       # 187 lib + 11（另有 2 个 ignored）
cd ServerRust  && cargo test       # 737 lib + 6 integration（另有 3 个 ignored）
cd Client-Bevy && cargo test       # 542 lib + 2 + 1 + 24
```

三侧 `cargo fmt -- --check` 均须 0 差异。

（数字为 2026-09-14 master 实测基线，随批次增长；以 CI 与本地复跑为准。）

### 5.2 真机 E2E（真实 ServerRust）

```powershell
pwsh scripts/run_real_e2e.ps1      # 默认用仓库内 debug 产物与测试库
```

脚本会：准备测试库（`scripts/e2e_setup_db.py`）→ 起服务端 → 跑用例 → 按客户端日志里的
`✅/❌` 标记判定 → 打印汇总 → 停服务端。用例与判定标记：

**当前基线（2026-09-14）：12/13 通过**，唯一未过的是下面标注「专项前置」的精炼用例。

| 用例 | flag | 判定标记 |
|---|---|---|
| 钓鱼 | `--fishing-test` | `[FISHTEST] ✅ 收获消息` |
| 坐骑 | `--mount-test` | `[MOUNT] ✅ 下马成功` |
| 商城 | `--gameshop-test` | `[SHOPTEST] ✅ 完成（购买 #…）` |
| 排行榜 | `--ranking-test` | `[RANKTEST] ✅ 排行榜` |
| 精炼 | `--refine-test` | `[REFINETEST] ✅ 精炼已开始` |
| 举报 | `--report-test` | `[REPORTTEST] ✅ 举报已提交确认` |
| 升级特效 | `--level-fx-test` | `[LEVELFX] ✅ PASS 升级生效` |
| 组队 / 私聊 / 邮件 / 交易 / 好友 / 婚姻 | 双客户端配对 | `[GROUPTEST]`/`[WHCHECK]`/`[MAILREAD]`/`[TRADETEST]`/`[FRIENDTEST]`/`[MARRY]` |

测试库准备脚本会（幂等）把测试角色摆到安全钓鱼点、复位「允许组队/交易/观察/结婚」开关、
补鱼竿的鱼钩与鱼饵、恢复精炼/交易所需物品——这些开关和消耗品会被用例改动并被服务端**存档**，
不复位会出现「上一轮跑通、下一轮偶发失败」。

**精炼用例的前置更重**：需要把角色放到铁匠（`Blacksmith_Carlos`）旁、背包带指定武器、
服务端精炼基础成功率调成 100%，属于专项复验（历史做法：临时改库 + `config/server.toml` 的
`[refine] base_chance`，跑完还原）。默认库状态下该用例不保证通过。

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
| 端口被占用 | `config/server.toml` 的 `network.listen_addr`；客户端同步改 `config.ini` 的 `ServerAddr` |
| 角色存档异常/数据回退 | 看服务端日志是否有 `Failed to save player …`（存档是单事务，任何一步失败整档回滚）；实例见 #2879（好友主键冲突，已修） |
