# Client-Bevy — 传奇2 (Legend of Mir 2) 客户端 Bevy 移植版

把 `Client-Macroquad`（macroquad + hecs 实现）迁移到 **Bevy 0.19**。
UI 交互以原版 C# 客户端（`Client/MirScenes/Dialogs/*.cs`）为准，游戏绘制/网络参考 Rust 版
（`Client-Macroquad` + `SharedRust`/`ServerRust`）。

## 当前状态（M58，2026-08-03）

✅ **数据层**（引擎无关）：
- `resources/mlibrary.rs` — `.Lib` 图像库解析（原始 RGBA）
- `resources/map_reader.rs` — `.map` 解析（7 种格式）
- `resources/libraries.rs` — 库注册表（MapLibs[0-399] + 24 个单体库）

✅ **渲染层**：
- `map_renderer.rs` — 32x32 格块纹理（1536x1024），Back/Middle/Front 三层 + 逐瓦片前景遮挡
- Y 轴深度排序（z = depth_y），精灵图缓存，日夜循环，雨/雪粒子，伤害飘字，SoundList 450 条

✅ **登录/选角/新建/删除角色界面**（bevy_ui）：
- ChrSel 动画背景 + 原版坐标/纹理；中文字体内嵌（阿里普惠体）；拼音输入法（内置 IME）
- 新建角色/删除角色/改密/注册全流程（真实 ServerRust 联调通过）

✅ **网络层**（真实 TCP + mock）：
- codec → PacketHeader → 类型化包（mir2_shared）；KeepAlive 心跳
- 真实 ServerRust 全链路：握手 → 登录 → 选角 → StartGame → 进游戏 → 对象生成
- **M58 断线自动重连**：指数退避重连 + 自动重新登录并进入之前的角色
- 264+ 服务端包 handler、145+ 客户端包发送（随对话框/玩法里程碑推进）

✅ **游戏场景**：
- 主对话框 HUD（血/蓝球、经验条、金币/等级/名字、原版五按钮 + 菜单/商城）、聊天面板（Enter/IME）
- 玩家控制：右键寻路（A*）、左键 NPC/怪物交互、中键 AutoRun、移动 Walk/Run + 远端插值
- 战斗：攻击发包、受击/死亡动画、伤害飘字、自动喝药、技能/状态（Buff 对话框）

✅ **对话框系统（M9 全部完成，56 个）**：
- 核心：inventory（40 格数据驱动）/ character（4 页 + 14 装备槽）/ menu / minimap / belt / compass
- 交互：npc / npc_goods（商店闭环）/ trade / amount_box / group / quest_log / friend / inspect
- 社交：guild / guild_territory / mail / market（寄售=TrustMerchant 等价物）/ item_rental / ranking / report / mentor / relationship / hero / creature / mount
- 系统批次（M51-M57 补齐）：option 设置 / keyboard_layout 键位 / big_map 大地图 / npc_awake 觉醒 / dura_status 耐久 / socket 镶嵌 / roll 掷骰 / sell_panel（出售+修理，即 C# NPCDropDialog）/ notice / chat_notice / timer / help / fishing / buff / storage / game_shop / refine / craft
- 对话框交互全部以原版 C# 布局/纹理/索引为准，网络链路与 ServerRust 联调通过（每个里程碑均有真实 E2E 证据）

✅ **质量**：
- 客户端 34 / 服务端 139 单元测试全过（`protocol_conformance::test_startgame_full_flow` 栈溢出为 HEAD 既有问题，与本次无关）
- 每个里程碑均有真实 ServerRust E2E 验证日志（详见 GitHub Issue #12 评论）
- 已知服务端遗留：NPC 登出未清理（多次登录后大地图 NPC 重复）、mount 无服务端 RideMount 包（坐骑对话框暂无价值）

## 运行

```bash
# 需要数据目录（共享 Client-Macroquad/Data，自动解析）与 ServerRust
# mock 模式（离线演示）：
cargo run --bin client_bevy -- --auto-enter          # 自动进游戏
cargo run --bin client_bevy                          # 手动登录界面

# 真实服务器：config.ini UseMock=false（在 Client-Bevy 目录修改），先启动 ServerRust
cargo run --bin client_bevy -- --auto-enter --e2e-user test --e2e-pass 123456

# E2E 自动化驱动（真实服务器，均带 --auto-enter）：
#   --option-test / --keyboard-test / --bigmap-test / --awake-test / --dura-test
#   --socket-test / --roll-test / --reconnect-test / --quest-test / --buff-test
#   --trade-test / --group-test / --mail-test / --guild-test / --market-test / ...

# 测试
cargo test
```

## 与 macroquad / C# 版的关系

- 共享 `SharedRust`（协议）与游戏数据目录（`Client-Macroquad/Data`、`Client-Macroquad/Map`）
- 数据解析逻辑保持与 `Client-Macroquad/src/resources/*` 一致，仅去掉渲染引擎耦合
- 对话框布局/交互以 `Client/MirScenes/Dialogs/*.cs` 为准（纹理索引、坐标、三态帧）

## 版本

当前锁定 `bevy = "0.19"`。从 0.16 升级时需适配：
`Projection`/`OrthographicProjection` 移到 `bevy::camera`、`RenderAssetUsages` 移到 `bevy::asset`、
`WindowResolution` 接受 `(u32, u32)`、系统函数最多 16 参数（超限拆 SystemParam/独立系统）、
Query 冲突需 `Without<T>` 或 ParamSet。
