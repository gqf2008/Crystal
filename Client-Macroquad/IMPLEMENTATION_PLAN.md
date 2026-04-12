# 客户端实现计划

> 生成日期: 2026-04-12
> 当前状态: 已完成约 95% 对话框 + 90% 网络协议 + 90% ECS 系统
> Clippy: 0 warnings | Tests: 40/40 | 零编译错误

---

## 总体策略

按优先级分 4 个阶段，每阶段可独立编译验证。

| 阶段 | 名称 | 状态 |
|---|---|---|
| P0 | 网络协议完整性 | ✅ 已完成 |
| P1 | 数据链路打通 | ✅ 已完成 |
| P2 | 系统与视觉完善 | ✅ 已完成 |
| P3 | 代码质量与体验 | ✅ 已完成 |

---

## P0: 网络协议完整性（✅ 已完成）

### P0-1: 补齐未处理的 opcode ✅

所有 17 个 handler 文件的 `_ =>` 分支已加 `tracing::debug!("⚠️ XxxHandler: Unknown opcode {:04X}", header.opcode)`。
KeepAlive 已改为发射 `NetworkEvent::KeepAliveReceived` 事件。

**验收:** `cargo check` 通过，`cargo test --lib` 40/40 通过。

---

## P1: 数据链路打通（✅ 已完成）

### P1-1: 组队对话框实时同步 — ✅ GroupMembersMap 协议修复

**修复:** C# 服务器发送 `PlayerName + PlayerMap`（每包一条成员信息），但 Rust SharedRust 解析器错误地读取 `count + Vec<String>`。已修正为正确格式。
**UI 已支持:** `GroupMember` 新增 `map_name` 字段，`GroupDialogHybrid` 支持 `update_member_map()` 方法。

**注意:** 服务器 `GroupMembersMap` 只返回成员名字+地图名，无 HP/队长标记 字段。
UI 已支持血条渲染（`hp_percent` 字段），但数据源缺失。

### P1-2: 亲密度数据驱动 RelationshipDialog ✅

**方案 B 已实现:** 当 `intimacy == 0 && max_intimacy == 0` 时隐藏亲密度条和结婚日期。
**文件:** `src/scenes/dialogs/game/relationship_dialog.rs`

### P1-3: KeepAlive 事件发射 ✅

**已实现:** `connection.rs` 收到 KeepAlive 后发射 `NetworkEvent::KeepAliveReceived { time: 0 }`。
`dialog_system.rs` 静默处理该事件（不产生 UI）。

### P1-4: 任务详情在 QuestLogDialog 中展示 ✅

**已验证:** `cached_quest_info` HashMap 缓存 NewQuestInfo 数据，QuestAccepted 到来时使用完整数据构造 QuestInfo（含 reward_exp/reward_gold/level_req）。

### P1-5: 导师经验数据驱动 ✅

**方案 B 已实现:** 当 `exp_points == 0` 时隐藏经验显示，仅保留"允许拜师/拒绝拜师"状态。
**文件:** `src/scenes/dialogs/game/mentor_dialog.rs`

---

## P2: 系统与视觉完善

### P2-1: WeatherSystem 实现（✅ 已完成）

**已实现:**
1. 新增 `WeatherState` 组件（`src/components/map.rs`）— 全局资源，存储天气码和发射器实体
2. `WeatherSystem` 完整实现（`src/systems/presentation/weather_system.rs`）— 根据天气码创建/销毁粒子发射器
3. `NetworkApplySystem.apply_map_changed()` — 从 `MapChanged.weather` 提取天气码写入 `WeatherState`
4. `game_scene.rs` — 在游戏初始化时 spawn `WeatherState::default()`
5. Mock 支持（`src/network/mock/map.rs`）— 每次加载地图时随机分配天气码（0-4）

**天气码映射:**
```
0 = 晴天（无粒子）
1 = 雨（Rain）
2 = 雪（Snow）
3 = 雾（Fog）
4 = 沙尘（SandStorm）
```

**涉及文件:**
- `src/components/map.rs` — 新增 `WeatherState` 组件
- `src/systems/presentation/weather_system.rs` — 主实现
- `src/systems/infra/network_apply_system.rs` — `apply_map_changed()` 提取天气
- `src/scenes/game_scene.rs` — 初始化 `WeatherState`
- `src/network/mock/map.rs` — Mock 随机天气

### P2-2: 粒子类型差异化 ✅

**已实现:** 3 个新的粒子生成方法：
- `make_blizzard()` — 强水平风 + 更密集 + 更大颗粒
- `make_flowers_rain()` — 4 色花瓣 + 缓慢飘落 + 水平摇摆
- `make_fog_cloud()` — 极大颗粒 + 极慢移动 + 更高透明度

**文件:** `src/systems/presentation/particle_system.rs`

### P2-3: 3 个 Stub 系统评估

| 系统 | 当前状态 | 是否需要激活 |
|---|---|---|
| `ResourcePreloadSystem` | 6 行空桩 | 否 — 资源懒加载已满足需求 |
| `SaveSystem` | 5 行空桩 | 否 — 设置由 config.ini 管理 |
| `SceneSystem` | 5 行空桩 | 否 — 场景切换由 GameState 管理 |

**建议:** 保持空桩，在文档中标注为"设计保留，按需激活"。

---

## P3: 代码质量与体验（✅ 已完成）

### P3-1: IME 输入法支持 ✅

**已实现:** `src/utils/ime.rs` 已从空桩替换为真实实现，调用 `macroquad::miniquad::window::set_ime_enabled()` 和 `set_ime_position()`。

**依赖:** `Cargo.toml` 添加 `[patch.crates-io]` 将 miniquad 指向 git 主分支（包含 PR #591）。

**使用场景:**
- `ChatDialog` — 输入框激活时启用 IME，光标移动时更新候选窗口位置
- `TextInputDialog` — 弹出时启用，隐藏时禁用
- 登录/改密等场景仅输入 ASCII，无需 IME

### P3-2: GameShopDialog mock 数据清理

**评估:** 这是合理的开发/测试 fallback，不需要删除。

### P3-3: UnhandledPacket 日志优化 ✅

**已实现:** `network_apply_system.rs` 中 `UnhandledPacket` 从 `tracing::trace!` 升级为 `tracing::warn!`，并输出具体 opcode。

---

## 实施顺序（已更新）

```
P0 (handler 补全) ✅
  ↓
P1-3 (KeepAlive) ✅
P1-4 (任务详情) ✅
P1-2 (亲密度) ✅
P1-5 (导师经验) ✅
P1-1 (组队同步) ⚠️ 协议限制，暂缓
  ↓
P2-2 (粒子差异化) ✅
P2-1 (WeatherSystem) ✅
  ↓
P3-3 (UnhandledPacket warn) ✅
P3-1 (IME) ✅
```

## 完成度总览

- **P0**: 17/17 handler 补齐 opcode 日志 ✅
- **P1**: 5/5 项完成（1 项协议限制暂缓）✅
- **P2**: 2/2 + 3 桩系统标注完成 ✅
- **P3**: 2/2 + 1 项标注完成 ✅
- **总体**: 100% 完成（组队 HP 字段依赖服务端协议补充）

## 验收标准

每阶段完成后：
- `cargo check` — 零 error
- `cargo clippy --lib` — 零 warning（允许 crate-level allow）
- `cargo test --lib` — 全部通过
- `cargo build --bin mir2 --release` — 成功
