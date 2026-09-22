# 玩家视角实测报告（2026-09-22）

**被测版本**：master `563b73e8`（含 #3006/#3007/#3009 三个恢复与门禁 PR）
**环境**：ServerRust release（后台常驻）+ Client-Bevy debug（主检出自建，`--real-net --auto-enter --e2e-user test`），
测试角色 `bevychar`（31 级，HP 11359/50005，金币 1,000,000），地图 0 BichonProvince（288,616）。
**判定工具**：`jev-decisions`（`typesafe/jev-1.13-20260917`）以玩家身份对覆盖度与下一步做类型化判定；门禁类结论一律走确定性检查。

> 本文件与 `player_walk.ps1` / `player_walk2.ps1` / `player_shop.ps1`、`player_shots/`、`player_walk*_results.json`
> 同处 `tools/acceptance/`；该目录被 `.gitignore` 白名单排除（只有 3 个文件入库），故这些证据目前**只在本机**——
> 与 §6.1 记录的老问题同源（报告引用的脚本不在干净 checkout 上，见 `LESSON_实机验收脚本要当门禁…`）。

---

## 1. 走过的路径与实测结果

| ID | 玩家路径 | 实测结果 | 证据 |
|---|---|---|---|
| P-01 | 启动器进游戏（登录→选角→进图） | ✅ 进图 tile=(288,616)，地图 700x700，36 chunk / 2629 前层精灵 / 52 灯光 | `player_shots/p01_world.png`、客户端日志 |
| P-02 | 世界行走（单方向） | ✅ 精确 +1 格（294→295），回走复原 | `p21_walk_step.png`、`player_walk2_results.json` |
| P-03 | 跑步移动 | ✅ 位移生效（288→289） | `p03_run.png` |
| P-04 | 40 窗交互巡回（开窗→点 X 关闭钮→断言关栈） | ✅ **41/41，退出码 0**（含 NPC 窗与 hero_manage；6 个设计无 X 的窗走 RPC 往返） | 门禁脚本 stdout、`ui_interact_results.json` |
| P-05 | 背包（含 ITEMS I/II/QUEST/BUY 页签） | ✅ 开/关正常，9/10 格有物，金币 1,000,000 显示正确 | `p27_inventory_after_make.png` |
| P-06 | GM 造物（`@MAKE Saddle` / `药水` / `@GOLD`） | ✅ 聊天区回「已给物品：Saddle / 药水 / Gold」，背包出现相应条目 | `p27_inventory_after_make.png` |
| P-07 | 骑乘（`@RIDE` + 骑乘状态移动） | ✅ 客户端日志「🐴 玩家 6868 骑乘坐骑 type=0」，骑乘下可移动 | `p17_mount.png`、客户端日志 |
| P-08 | 中文输入法（候选/上屏/取消） | ✅ **10/10**：`ime_composing='nihao'`、上屏 `chat_input_text='你好'`、无裸 ASCII 泄漏 | `ime_rpc_verify_results.json`、`shots/ime_rpc_3_candidates.png` |
| P-09 | 宠物面板（5 只宠物） | ✅ 立绘偏移已修、5 只宠物列在册、STAMINA/饥饿/模式显示正常；⚠ 说明文字含占位串 | `p15_creature.png` |
| P-10 | 行会面板（公告/成员/仓库/名次） | ✅ 页签与公告正文渲染、滚动条可见；⚠ NOTICE 页右侧残留一块空黑区 | `p14_guild.png` |
| P-11 | 其余界面：角色/任务/邮件/仓库/大地图/设置/合成/精炼/镶嵌/钓鱼/排行/好友/组队/师徒/关系/帮助/公告等 | ✅ 逐个开→截图→关，均由 P-04 的巡回判定通过 | `player_shots/p1*.png`、`p20_*.png` |
| P-12 | 商城浏览 | ✅ 窗开、分类树+8 格+页脚（金币/积分单选、1/14 页）齐全；⚠ 格子标题是 `#1268` 这类内部 ID、无物品图标、右上预览区空白 | `shop12_after_buy.png` |
| P-13 | 商城购买尝试（点格→点 BUY） | ⚠ **未完成成交**：点击命中 `root=GameShop` 的节点（格 24x21、BUY 143x30），但金币未变、无购买回执 | `shop11_cell_clicked.png` / `shop12_after_buy.png` |
| P-14 | 走近 NPC 对话 | ❌ 直行 30 步仍被地形挡在 136px 外（本驱动无绕障）；按玩家真实动作**点击 NPC**也因距离过远未开窗 | `p24_npc_dialog.png` |
| P-15 | 近战击杀 | ❌ 贴身（32px）连打 30 次，鹿/稻草人未死；期间怪物对玩家有伤害（HP 50005→11359） | `p22_combat_kill.png`、调试日志 |
| P-16 | 掉落拾取 | ⚠ 因 P-15 未产生掉落，未验证 | `p23_no_drop.png` |
| P-17 | 传送换图（`@MOVE 0 300 300`） | ❌ 只回一条系统消息，坐标未变 | `p26_after_move_cmd.png` |

## 2. 发现的不一致与缺陷

### 2.1 P0 — 缺地图文件时客户端直接 panic（本次实测真崩过一次）

**现象**：客户端进图时 `panicked at ... error/handler.rs:130`，
`Encountered an error in system client_bevy::map_renderer::chunks::chunk_stream_system: Parameter Res<'MapLightTexture> failed validation: Resource does not exist`，
随后 `main_schedule::Main::run_main` 一并 panic，进程退出、服务端记录 `Session read error … os error 10054` 并离线存档。

**根因**：`MapLightTexture` 只在 `setup_world` 的**成功路径**（`chunks_build.rs:123`）插入，
而地图加载失败会走 M4 失败分支（日志 `❌ 地图加载失败 Map/0.map: Map file not found`）——
该分支不插入资源；`chunk_stream_system`（`mod.rs:302`）只按 `in_state(Game)` 运行、参数是必选的 `Res<MapLightTexture>`，
于是下一帧参数校验失败直接 panic。代码注释本意是「加载失败→错误可见+退回登录」（`chunks_build.rs:537`），实际不是。

**复现**：让客户端找不到地图（本次是资源根指向另一 worktree、其 `Client-Bevy/Data/Map` 为空）即 100% 复现。
**影响**：任何地图资源缺失/损坏（打包漏文件、玩家客户端半更新）都会变成一进图崩溃，而不是可读的错误提示。
**建议**：`chunk_stream_system` 参数改 `Option<Res<MapLightTexture>>`（或加 `resource_exists` 门控），失败分支插默认资源 + 显示错误。

### 2.2 P1 — 玩家攻击打不死怪

**现象**：贴身（1 格）连续 30 次攻击，鹿/稻草人未死亡；同场怪物对玩家有伤害（HP 50005→11359）。
客户端确实发出攻击包：调试日志 `send_packet opcode=47`（ClientPacketIds::Attack）共 **80 个**。
聊天区常驻提示 **「攻击模式：和平」**。
**未定位根因**：三种可能未排除——① 和平模式被服务端用于拒绝攻击；② 客户端 `Attack{direction, spell}` 不带目标 id，
服务端按朝向取目标，朝向/序列不匹配被丢弃；③ 伤害为 0（角色徒手，日志有 `CWeapons[793] 贴图缺失`）。
**测试能力缺口**：control RPC 没有「切换攻击模式」「注入按键」的通道，只能发攻击请求，无法完成完整战斗闭环。

### 2.3 P2 — `@MOVE` 传送命令无效

`@MOVE 0 300 300` 后聊天区出现系统消息，但 `state` 的坐标不变（296,616 → 296,616），世界也未重载。
同批 `@MAKE`（Saddle/药水/Gold）均生效，说明 GM 通道本身可用，问题在 MOVE 的参数/校验路径。

### 2.4 P2 — 心跳按帧率往返（网络噪音）

调试统计：2 分钟内 `send_packet opcode=2`（KeepAlive）**2912 个**，峰值 **63 个/秒**。
成因是服务端发心跳、客户端收到即回（`handle_social.rs:872-874`），呈乒乓；客户端自身另有「5 秒无包才发」的节流，
所以量级由服务端发送频率决定。**建议**：核对服务端 KeepAlive 节拍（C# 口径为秒级），避免 60/s 无意义往返。

### 2.5 P3 — 玩家可见的文案/界面瑕疵

| 位置 | 现象 | 证据 |
|---|---|---|
| 宠物面板说明 | `可以拾取物品（0x0 semi-auto0x0 mouse）。` —— 含占位串 | `p15_creature.png` |
| 行会 NOTICE 页 | 右侧残留一块空的黑面板（疑似隐藏页串页） | `p14_guild.png` |
| 商城格子 | 标题直接显示 `#1268`…（内部物品 ID），无图标；右上预览区空白 | `shop12_after_buy.png` |
| 资源 | `CWeapons[793]` 贴图加载失败（系统找不到文件） | 调试日志 |

其中商城「物品图标/预览/名称」与此前验收报告 §10.5、§11.4 记录为**已知未补齐项**（不是本轮新发现）。

## 3. 可信度自评

**结论：本轮未达 9.5。给出 7.0 / 10（以「把游戏全部功能玩一遍」为目标的覆盖可信度）。**

依据：

1. **Jev 独立判定（玩家身份）**：覆盖度落在 5 档量表的**第 2 档**——「界面全走通 + 部分玩法（移动/骑乘/造物）走通，
   战斗与交易等核心闭环没走通」（p=0.990）；同时判定「存在玩家可感知缺陷」p=0.98。
2. **实测比例**：48 个窗口全部开/关（41/41 交互门禁）＋ 2 个真值脚本（IME 10/10）＝ 界面面 100%；
   但**玩法闭环**只走通移动/跑步/骑乘/GM 造物/聊天，战斗、买卖、任务、邮件、仓库、交易、死亡复活均未闭环。
3. **反面证据**：本报告不是「只跑绿」——真崩了一次（P0）、真发现近战无效（P1）与传送无效（P2），
   且每条都附了可复查的日志/截图/堆栈。
4. **为什么不给高分**：可信度要看「是否真把全部功能走一遍」。Jev 的第 2 档即 40% 量级的玩法覆盖；
   把未覆盖的核心闭环按「用户能感知的功能面」估算，整体覆盖约五成，故 7.0。

**要把可信度抬到 9.5 以上，至少要补齐（按 Jev 指出的优先级）**：

1. **买卖闭环**（Jev `next_path=trade_shop`，0.83）：走到商人旁 → 开商店 → 买/卖 → 核对金币与背包增减；
   前置：给 control RPC 补「绕障寻路到坐标」或按键注入，否则挡在建筑外的 NPC 无法触达。
2. **战斗闭环**：先定位 2.2 的根因（和平模式/朝向/伤害），再做「击杀→掉落→拾取→经验/等级变化」全链。
3. **任务闭环**：接取 → 目标达成 → 提交 → 奖励。
4. **邮件与仓库闭环**：发送/附件/收取；存入/取出并核对格子。
5. **跨图传送与死亡复活**：传送生效后验证世界重载；死亡→复活→掉落。
6. 顺带把 §2.1 的 panic 修掉（否则「地图缺文件」这条最常见的玩家事故仍然是崩溃）。

## 4. 复现配方

```powershell
# 1) 服务端（后台常驻）
cd ServerRust; ./target/release/mir2_server.exe

# 2) 客户端（主检出重建，资源根=../Data + ../ServerRust/Daneo1989/Maps）
$env:PATH='D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;'+$env:PATH
$env:LIBPINYIN_DIR='D:/toolchains/libpinyin-install'
cd Client-Bevy; ./target/debug/client_bevy.exe --real-net --auto-enter --e2e-user test --e2e-pass 123456

# 3) 遍历与门禁
pwsh tools/acceptance/ui_interact_sweep.ps1     # 40 窗交互门禁（本轮 41/41）
pwsh tools/acceptance/player_walk.ps1            # 玩法第一批（20 步）
pwsh tools/acceptance/player_walk2.ps1           # 玩法第二批（走到目标旁再交互）
pwsh tools/acceptance/ime_rpc_verify.ps1         # 中文输入法真值（需客户端在跑）
```

**Jev 判定原始响应**：`%TEMP%\jev-review\out_play.json`（含 `player_verdict` / `next_path` / `coverage_self` 三问）。
