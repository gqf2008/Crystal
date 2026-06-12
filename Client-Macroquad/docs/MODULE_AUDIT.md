# Client-Macroquad 模块使用审查(Module Audit)

日期: 2026-06-12 | 重写基于实际代码探索结果

> 与历史版本的差异:早期 `MODULE_AUDIT.md` 写于 2025-12,描述的状态已被以下工作覆盖
> (按时间顺序):
> 1. `feat: migrate all NetworkEvent handlers to real packet data through ECS pipeline` (9f893db4)
> 2. `refactor: split monolithic world.rs (12511 lines) into 12 domain-specific submodules` (d1de5344)
> 3. `refactor(client): split large system files, add DialogWidget trait, UI efficiency fixes` (189d38a8)
> 4. **本 commit 周期** (a8470f9c, bc05d925, e93c2362, d21c01d5, …):移除 `src/camera/`、合并 IME、补全 5 个 partial 对话框、同步 master 协议

## 目标

- 盘点 `src/` 下各模块/子目录:
  - 是否进入编译树(被 `lib.rs` 或某个 `mod.rs` 引入)
  - 是否被运行链路实际使用(从 `src/bin/*.rs` 的入口往下)
- 哪些是当前真实依赖、哪些是未接入/历史遗留、哪些已收敛

## 方法

1. 从入口程序开始:`src/bin/*.rs`(`mir2`、`test_game_scene`、`scene_demo` 等 13 个)
2. 追踪其 `use client_macroquad::...` 引用到的库模块
3. 对 `src/` 下顶层目录做两层判断:
   - **编译树层**:是否在 `src/lib.rs` 中 `pub mod ...`
   - **运行层**:是否存在明确的调用链/实例化/执行点

## 模块总览(按 src 顶层目录)

| 模块/目录 | 编译树 | 运行链路 | 备注 |
|---|:---:|:---:|---|
| `camera/` | — | — | **2026-06 删除**(GameCamera2D 146 行,从未被引用) |
| `compat/` | ✓ | 部分 | 向后兼容 re-export 层,只在老 API 路径调用 |
| `components/` | ✓ | 是 | 21 个组件文件,ECS 核心,scene + systems 都用 |
| `coord/` | ✓ | 是 | 坐标转换,所有渲染路径都依赖 |
| `core/` | ✓ | 是 | 基础错误/常量/设置 |
| `event_bus/` | ✓ | 弱 | `FrameEndSystem::clear_frame()` 消费;`send_*` API 仍无主链路调用 |
| `game/` | ✓ | 是 | `GameState` + `GameContext`,所有场景共用 |
| `map_renderer/` | ✓ | 是 | mesh-based 地图渲染,scene/map_viewer 都用 |
| `network/` | ✓ | 是 | TCP + mock 双模式,`NetworkSystem` 已接入调度 |
| `objects/` | ✓ | 是 | 帧动画数据,精灵系统依赖 |
| `resources/` | ✓ | 是 | MLibrary 解析,所有纹理/数据来源 |
| `scenes/` | ✓ | 是 | login / select / game / loading,主运行入口 |
| `systems/` | ✓ | 是 | 43/46 系统注册到 `SystemScheduler` (3 个未注册,见下) |
| `ui/` | ✓ | 是 | 原生 UI 渲染 + 51 个对话框 |
| `utils/` | ✓ | 部分 | `ime.rs`(thin wrapper)被 chat/text_input 依赖 |

## ECS Systems 实际状态(从"死目录"变成"已运行")

`src/systems/` 下 6 个子目录、46 个系统文件。原 `MODULE_AUDIT.md` 声称"无调度器入口",**此为过时描述**。

`game_scene.rs::new()` 实际注册了 **43 个系统** 到 `SystemScheduler`:

| 层级 | 数量 | 例子 |
|---|---:|---|
| `infra` (0-99) | 5 | `NetworkSystem`, `NetworkApplySystem`, `MapBootstrapSystem`, `MapLoadSystem`, `TimeTickSystem`, `FrameEndSystem` |
| `input` (100-199) | 3 | `PlayerControlSystem`, `LocalPlayerAiSystem`, `AutoPotionSystem` (注: `InputStateSystem` **未注册**) |
| `logic` (200-599) | 12 | `CombatSystem`, `SkillSystem`, `HealthRegenSystem`, `BufSystem`, `PathfindingSystem`, `MovementSystem`, `CollisionSystem`, `MonsterAISystem`, `NpcAISystem`, `NpcDialogueSystem`, `LifetimeCleanupSystem`, `PositionInterpolationSystem`, `RemoteMoveAnimSystem`, `MountStateSyncSystem` |
| `presentation` (600-899) | 13 | `AnimationSystem`, `ParticleSystem`, `SoundSystem`, `WeatherSystem`, `FloatingTextSystem`, `HealthBarAnimSystem`, `CameraSystem`, `CameraFollowSystem`, `CameraBoundsSystem`, `CameraSpaceGateSystem`, `UISystem`, `HUDSystem`, `MinimapSystem`, `DialogSystem` |
| `rendering` (900+) | 4 | `MapRenderSystem`, `SpriteRenderSystem`, `EffectRenderSystem`, `UIRenderSystem` |
| `dbug` (9000+) | 1 | `DebugSystem` |

**未注册到调度器的 3 个系统**(都是 isolated single-file, 文档标记为"实验性"):

| 文件 | 状态 |
|---|---|
| `systems/input/input_state_system.rs` | 定义了但未注册,可能用于将来的"指令缓冲" |
| `systems/logic/physics/map_update_system.rs` | 同上,可能并入 `MapLoadSystem` |
| `systems/rendering/sprite_system/{character,weapon}.rs` | 子模块,被 `sprite_system/mod.rs` re-export;主 sprite_system 已注册 |

## IME 收敛(从"3 套实现"到"1 套")

历史:曾存在 3 套 IME 入口:
- `src/utils/ime.rs` — thin wrapper(现存活)
- `src/platform/ime.rs` — Windows API / imm32.dll(**2025 年某次重构中已删除**)
- `macroquad::miniquad::window::set_ime_*` — 第三方库直接调用

**当前唯一入口**:`src/utils/ime.rs` 内 2 个函数,薄包装到 `miniquad::window::set_ime_enabled / set_ime_position`。

调用点:
- `src/scenes/dialogs/game/chat_dialog.rs` (5 处: enabled + position)
- `src/scenes/dialogs/game/text_input_dialog.rs` (1 处: enabled)

新代码**禁止**重新引入 `platform::ime` 或绕过 utils 直接调用 `miniquad::window`。

## 输入模块 `src/input_support/`

**2025-12 时**此目录存在但未在 `lib.rs` 注册(孤立目录)。**当前(2026-06)** 整个目录已被删除。MODULE_AUDIT 历史版本中关于"input_support"的所有描述都基于一个已被移除的目录。

## 协议对齐状态(2026-06)

合并 master 后,本分支已与上游同步以下 PR 的协议层:

| PR | 功能 | Rust 端状态 |
|---|---|---|
| #1169 | Warehouse password | ✅ `client::UnlockStorage`, `client::SetStoragePassword`, `client::RemoveStoragePassword`, `server::StorageUnlockResult`, `server::StoragePasswordResult` |
| #1126 | KR NPC/Quest Linking | ⚠️ `client::RequestMonsterInfo/NPCInfo/ItemInfo` 已加,但**未在 handler 中 wire**(dialog 暂未触发 tooltip 请求);`NewMonsterInfo`/`NewNPCInfo` server packet 未在 SharedRust 实现(ClientNPCInfo 字段重写已回退为兼容版) |
| #1148-1168 | 9 个纯 C# UI PR(KR 风格、weight bars、bag tab、socket tooltip) | ⏭️ 跳过(无 Rust 端代码可移植) |

**未与 master 同步**(已知漂移):
- `Spell`/`Monster`/`BuffType`/`MirAction` 等枚举的 **数值**(master 与本分支有偏差)
  - 影响: 5 个 `enums::tests::*_roundtrip` 测试失败(预存在问题,本 commit 周期未触及)
  - 解决路径: 需要在 SharedRust 端对每个 enum 重新对齐,或更新测试期望值
- `Shared/Enums.cs` 在合并后被 master 改动但**未**与 Rust 端 `SharedRust/src/enums.rs` 完全双向同步(只对 ClientPacketIds/ServerPacketIds 做了 ID shift,内层 enum 值未对齐)
- `Shared/Data/ClientData.cs::ClientNPCInfo` 字段顺序回退为 5 字段版本(master 是 12 字段,合并后手动回退以保持 Rust 端 read_from 兼容)
- `Shared/Data/MonsterData.cs` 由 master 引入(`ClientMonsterInfo` 数据类),Rust 端**未实现**

## 对话框完成度(2026-06)

51 个对话框文件(含子模块),覆盖 C# 端 36 个对话框 + 多个扩展。详细矩阵见 `GAMESCENE_UI_TODO.md`。

**本 commit 周期补全的 partial 对话框**:
- `compass_dialog.rs` — 罗盘纹理从 Title[468] 占位 → Prguse2[1470] 对齐 C#
- `game_shop_dialog/rendering.rs` — Buy 按钮 TODO 替换为 `GameShopBuyAction::Buy` 发包
- `socket_dialog.rs` — 新增 gem picker 子面板,收集 AwakeType 后发 `AwakeningRequest`(修复 [[memory:feedback_socket_gem.md]])
- `guild_territory_dialog.rs` — 新增 Buy 按钮,每个未占领领地右侧显示
- `network_events.rs` — GuildStorageItemChanged TODO 文档化为协议限制

**仍为 partial 的对话框**(本周期未触及):
- `chat_notice_dialog.rs` — 文本居中对齐有偏
- `npc_awake_dialog.rs` — 装备觉醒流程需与 SocketDialog 联动
- `craft_dialog.rs` — 合成材料槽位未持久化

## 死代码清理历史

| 日期 | 删除/收敛项 | 行数 |
|---|---|---:|
| 2025-12 | `src/input_support/` 目录 | ~50 |
| 2025-12 | `src/platform/ime.rs`(Windows API) | ~30 |
| 2026-06 | `src/camera/{camera2d,mod}.rs` | 146 |
| 2026-06 | `MODULE_AUDIT.md` 过期描述(input_support/platform) | — |

**当前剩余可疑项**(`utils::ime` thin wrapper 是允许的):
- 无

## 后续建议(非本次工作)

1. **Spell/Monster 枚举对齐**:与 master 双向同步所有 enum 值,修复 5 个失败的 enum roundtrip 测试
2. **NewMonsterInfo/NewNPCInfo 实现**:在 SharedRust 端加 `ClientMonsterInfo`,配合 handler 接收 tooltip 信息
3. **InputStateSystem/MapUpdateSystem** 注册:评估后并入调度器,或标为 deprecated 删除
4. **持续 merge master**:本 commit 周期已合一次,但上游仍在演进(2026-05 仍在合并新 PR)

---

**如果你是新接手者**:上面的"模块总览"表是 ground truth。任何与表不符的描述都是过时的。
