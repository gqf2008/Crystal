# Crystal 传奇2 Rust 客户端 - 优化计划

> 基于全面 Code Review 产出，按优先级分为 5 个阶段。
> 预计总工作量较大，建议按阶段逐步推进。

---

## 阶段一：正确性 Bug 修复（P0 - 必须修复）

> 这些问题直接影响游戏正确性，应最先处理。

### 1.1 负索引转型 Bug
**文件**: `src/components/character_select.rs:39, 48`

`index == -1` 时 `as usize` 变成 `usize::MAX`，导致取消选中永远被拒绝。

```rust
// Before
pub fn set_selected(&mut self, index: i32) {
    if index >= -1 && (index as usize) < self.characters.len() {

// After
pub fn set_selected(&mut self, index: i32) {
    if index == -1 || (index >= 0 && (index as usize) < self.characters.len()) {
```

同样修复 `get_selected` (line 39)。

**改动量**: 2 行

---

### 1.2 DoT 伤害帧率相关
**文件**: `src/systems/logic/combat/regen_system.rs:72-87`

当前每帧 `damage = strength * delay_time`，60fps 下 delay_time≈0.016 导致 damage=0。

**方案**: 引入累加器或在 BuffList 中记录上次跳伤害的时间。

```rust
// Buff DoT tick 改为基于计时器
if now - buff.last_damage_tick >= buff.tick_interval {
    health.current -= buff.damage_per_tick;
    buff.last_damage_tick = now;
}
```

**改动量**: ~20 行

---

### 1.3 MapUpdateSystem 清空世界后吞错误
**文件**: `src/systems/logic/physics/map_update_system.rs`

当前流程: `world.clear()` → 加载地图 → 失败时返回 `Ok(())` → 世界已空。

**修复**:
1. 先尝试加载地图，成功后再 `world.clear()`
2. 错误时返回 `Err(...)` 而非 `Ok(())`

```rust
// Before: 先清世界再加载
ctx.world.clear();
match load_map(...) {
    Ok(map) => { /* ... */ }
    Err(e) => {
        tracing::error!("...");
        return Ok(()); // 世界已经空了！
    }
}

// After: 先加载成功再清理
let map = load_map(...).map_err(|e| /* ... */)?;
ctx.world.clear();
// 写入新地图
```

**改动量**: ~10 行

---

### 1.4 碰撞系统每帧 clone 整个地图网格
**文件**: `src/systems/logic/physics/collision_system.rs:120`

`map_data.cells.clone()` 是游戏中最大的数据结构，每帧 clone 严重影响性能。

**方案**: 改为引用或 Arc 共享。地图变更时更新引用，帧内只读。

```rust
// 方案 A: 使用 Arc
cells: Option<Arc<Vec<Vec<CellData>>>>

// 方案 B: 改为 &MapData 引用查询
// 从 ECS 中获取 MapData 组件后直接引用
```

**改动量**: ~15 行

---

### 1.5 战斗伤害 target 不匹配
**文件**: `src/systems/logic/combat/combat_system.rs:534-553`

`take_damage()` 遍历所有 `LocalPlayer` 扣血，不使用 `target_id`。应通过 entity ID 或 NetworkSync 匹配目标。

**改动量**: ~10 行

---

### 1.6 MenuDialog 直接退出进程
**文件**: `src/scenes/dialogs/game/menu_dialog.rs:282`

`std::process::exit(0)` 应改为触发 `SceneTransition::Exit`。

**改动量**: 2 行

---

### 1.7 粒子时间戳使用 SystemTime
**文件**: `src/components/particle.rs:30`

`SystemTime::now()` 会被 NTP 调整影响。改为 `macroquad::get_time()` 或 `Instant::now()`。

**改动量**: ~5 行

---

## 阶段二：功能完整性（P1）

### 2.1 补全网络 Handler 存根

> 当前大量 opcode 被路由到 handler 但直接返回 `UnhandledPacket`。

**优先级排序**（按功能重要性）:

| Handler | 当前处理 | 待实现 | 影响功能 |
|---------|---------|--------|---------|
| Item | 4/37 | MoveItem, EquipItem, UseItem, DropItem, MergeItem, SplitItem | 物品系统 |
| NPC | 2/32 | NPCSell, NPCRepair, NPCStorage, CraftItem | NPC交互 |
| Movement | 基础/15+ | Teleport, BackStep, Dash, SitDown | 战斗表现 |
| Quest | 0/全部 | 所有 | 任务系统 |
| Trade | 2/6 | TradeGold, TradeItem, TradeConfirm, TradeCancel | 交易系统 |

**方案**: 每个 handler 逐个补充解析逻辑，至少将 opcode 映射为 NetworkEvent 发送出去，即使暂时不做游戏逻辑处理。

**改动量**: 每个 handler ~50-100 行

---

### 2.2 删除/编译修复 inventory_persistence.rs
**文件**: `src/scenes/dialogs/game/inventory_persistence.rs`

引用了不存在的类型，当前编译会失败。

**方案**: 如果暂不需要持久化功能，直接删除文件并从 `mod.rs` 移除模块声明。

**改动量**: 删除 ~120 行

---

### 2.3 网络线程优雅关闭
**文件**: `src/network/client.rs`

添加 `shutdown()` 方法:
1. 暴露 read/write 线程的 JoinHandle
2. 添加 shutdown channel 或 AtomicBool
3. 写线程出错时发送 `Disconnected` 事件
4. read 线程 MAX_PAYLOAD 超限也发送 `Disconnected`

**改动量**: ~30 行

---

### 2.4 对话 z-ordering / Focus 管理

当前所有 Dialog 同时接收输入，重叠区域无优先级。

**方案**: 添加简单的 DialogFocusManager:
```rust
pub struct DialogFocusManager {
    active_dialogs: Vec<DialogId>, // 最后点击的在最后（最顶层）
}

impl DialogFocusManager {
    fn is_focused(&self, id: DialogId) -> bool {
        self.active_dialogs.last() == Some(&id)
    }
    fn bring_to_front(&mut self, id: DialogId) { ... }
}
```

每个 Dialog 在处理鼠标输入前先检查 `focus_manager.is_focused(self.id())`。

**改动量**: ~80 行

---

### 2.5 密码日志安全
**文件**: `src/network/client.rs:602-608` 及 `network/mod.rs:53`

`NetworkEvent` 的 `Debug` 实现会暴露密码。

**方案**: 手动实现 `Debug`，密码字段显示 `[REDACTED]`。

```rust
impl Debug for NetworkEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            NetworkEvent::LoginRequest { username, password, .. } => {
                f.debug_struct("LoginRequest")
                    .field("username", username)
                    .field("password", &"[REDACTED]")
                    .finish()
            }
            // ...
        }
    }
}
```

**改动量**: ~30 行

---

## 阶段三：性能优化（P2）

### 3.1 消除重复方向计算
**涉及文件**:
- `systems/logic/combat/combat_system.rs:645-668`
- `systems/logic/combat/skill_system.rs:409-432`
- `systems/logic/decision/npc_ai_system.rs:86-93`
- `systems/logic/physics/movement/movement_system.rs:52-81`

**方案**: 在 `coord.rs` 中统一实现:
```rust
pub fn calculate_direction(from: Coord, to: Coord) -> MirDirection { ... }
```

**改动量**: 提取 ~40 行，删除 ~100 行重复代码

---

### 3.2 UI 渲染性能优化

#### 3.2.1 提取透明皮肤创建为共享函数
**重复位置**: `belt_dialog.rs:177-202`, `inventory_dialog.rs:221-243`, `character_dialog.rs:301-316`, `game_shop_dialog/dialog.rs:386-411`

```rust
// native_ui_utils.rs 新增
pub fn create_transparent_skin() -> Skin {
    Skin {
        window_texture: load_texture_from_image(&Image {
            bytes: vec![0, 0, 0, 0], width: 1, height: 1,
        }),
        // ...
    }
}
```

#### 3.2.2 修复 draw_tooltip 字节长度计算
**文件**: `src/scenes/dialogs/game/inventory_dialog.rs:637`

```rust
// Before
let w = text.len() as f32 * 7.0 + 8.0;
// After
let w = text.chars().count() as f32 * 7.0 + 8.0;
```

#### 3.2.3 缓存聊天可见消息索引
**文件**: `src/scenes/dialogs/game/chat_dialog.rs:143-151`

`visible_message_indices()` 每帧分配 Vec。改为 Dialog 字段缓存，仅在消息增减时重新计算。

#### 3.2.4 对话纹理按需加载而非每帧查找
**文件**: 多处 `LibraryName::Prguse.get_texture()` 在 `update_and_draw()` 中每帧调用

在 Dialog 的 `load_textures()` 中一次性加载并缓存，draw 中直接使用缓存句柄。

#### 3.2.5 ItemTextureCache 添加淘汰机制
**文件**: `native_ui_utils.rs:339-393`

添加 LRU 或最大容量限制（如 500 个条目），超过时淘汰最久未使用的条目。

**改动量**: ~50 行

---

### 3.3 补全系统缺失 derive

| 组件 | 文件 | 缺少的 derive |
|------|------|-------------|
| MapData | `components/map.rs:56` | `Debug` |
| MovementVelocity | `components/movement.rs:28` | `Copy` |
| NetworkSync | `components/network.rs:8` | `Copy` |

---

### 3.4 Health/Mana 改为 u32
**文件**: `src/components/combat.rs:194-197, 224-227`

`current: i32` → `current: u32`，消除 `.max(0)` 守卫和负值可能性。

**改动量**: ~10 行 + 调用方适配

---

### 3.5 Inventory/Storage/QuestInventory 去重
**文件**: `src/components/item.rs`

三个容器结构体有完全相同的 `add_item`/`remove_item`/`get_item` 实现。

**方案**: 提取泛型容器:
```rust
pub struct ItemContainer<const N: usize> {
    slots: [Option<Item>; N],
}
```

或保留三个 newtype 但通过宏生成重复方法。

**改动量**: ~60 行

---

### 3.6 EquipmentSlot 枚举替换魔术数字
**文件**: `src/components/item.rs:158-221`

```rust
#[repr(u8)]
pub enum EquipmentSlot {
    Weapon = 0,
    Armor = 1,
    Helmet = 2,
    Necklace = 3,
    // ...
}
```

**改动量**: ~40 行

---

### 3.7 Experience::percent 除零防御
**文件**: `src/components/combat.rs:319`

```rust
// Before
(self.current as f32 / self.required as f32).clamp(0.0, 1.0)
// After
if self.required == 0 { 0.0 } else { (self.current as f32 / self.required as f32).clamp(0.0, 1.0) }
```

**改动量**: 3 行

---

### 3.8 怪物 AI 系统无意义 &mut
**文件**: `src/systems/logic/decision/monster_ai_system.rs:57`

`&mut Position` 从未修改，改为 `&Position`。

**改动量**: 1 行

---

## 阶段四：代码清理（P3）

### 4.1 删除死代码和占位系统

| 文件/位置 | 描述 | 操作 |
|-----------|------|------|
| `components/debug.rs` | `DebugCounters` 110 行无任何系统引用 | 删除文件 |
| `components/spell.rs:376-454` | `LearnableMagicList` 无引用 | 删除 |
| `systems/infra/resource_system.rs` | `ResourcePreloadSystem` 空结构体 | 删除或补充实现 |
| `systems/infra/scene_system.rs` | `SceneSystem` 空结构体 | 删除或补充实现 |
| `systems/infra/save_system.rs` | `SaveSystem` 空结构体 | 删除或补充实现 |
| `systems/rendering/entity_render_system.rs:139-147` | `EntityRenderSystem::draw()` 直接返回 Ok | 删除或补充 |
| `systems/logic/decision/npc_ai_system.rs` | `update_npc_ai()` 基本 no-op | 删除或标记 TODO |
| `systems/logic/mod.rs:1-68` | 68 行注释掉的系统声明 | 删除（移到文档） |
| `systems/mod.rs` | 大量注释掉的 Draw 变体代码 | 清理 |
| `systems/dbug/` | 目录名应为 `debug` | 重命名 |
| `systems/presentation/buf_system.rs` | 文件名应为 `buff_system.rs` | 重命名 |

**改动量**: 删除 ~300 行

---

### 4.2 删除备份文件
- `src/scenes/login_scene_backup.rs`
- `src/scenes/dialogs/game/game_shop_dialog/dialog_backup.rs`

---

### 4.3 Dialog 构造函数移除硬编码测试数据
**涉及**:
- `belt_dialog.rs:112-120` — 6 个硬编码物品
- `inventory_dialog.rs:135-150` — 46+46+20 个硬编码物品
- `character_dialog.rs:199-218` — 硬编码装备/技能
- `minimap_dialog.rs:84-125` — 硬编码地图对象

**方案**: `new()` 返回空结构体，数据通过外部方法注入（从 ECS/GameContext 同步）。

---

### 4.4 统一 Dialog open/close/toggle 接口

当前不一致的模式:
| Dialog | 方法名 | toggle 实现 |
|--------|--------|-----------|
| Belt/Inventory/Character | `open()`/`close()` | 调用 open/close |
| QuestLog/MiniMap | `open()`/`close()` | 直接翻转 `visible`（跳过 side effect） |
| Npc/NpcGoods | `show()`/`hide()` | — |

**统一为**: 所有 Dialog 使用 `open()`/`close()`/`toggle()`，`toggle()` 始终调用 `open()`/`close()` 保证 side effect 执行。

---

### 4.5 统一注释语言
当前文件间混合使用中文和英文注释。建议新代码使用英文注释（与现有英文文件保持一致），或根据团队约定选择。

---

### 4.6 修复优先级常量文档不一致

| 系统 | 注释写的优先级 | mod.rs 实际值 |
|------|-------------|-------------|
| CombatSystem | 310 | COMBAT = 300 |
| SkillSystem | 300 | SKILL = 310 |

互换修正注释或常量值（根据语义，Combat 应在 Skill 之前执行，所以 COMBAT=300 是正确的，注释写错了）。

---

### 4.7 修复格式问题
- 多处 `pub  fn` (双空格) → `pub fn`
- `components/player.rs:163-212` — deprecated 方法如果确定不再使用，直接删除而非保留

---

## 阶段五：架构改进（P4 - 长期）

### 5.1 拆分 AnimationSystem (God System)
**文件**: `src/systems/presentation/animation_system.rs:686-707`

当前单个 `update()` 做了 5 件事:
1. 动画帧计算
2. 攻击音效触发
3. 怪物 LibrarySprite 动画
4. 怪物攻击音效
5. 攻击动画生命周期

**拆分为**:
- `AnimationFrameSystem` (Priority 600) — 帧计算
- `AttackSoundSystem` (Priority 610) — 音效
- `MonsterAnimSystem` (Priority 620) — 怪物动画
- `AttackLifecycleSystem` (Priority 630) — 生命周期

---

### 5.2 拆分 UIRenderSystem
**文件**: `src/systems/rendering/ui_system.rs`

当前 `update()` 同时处理:
- UI 命令处理
- Minimap/HP/MP 数据同步
- 键盘快捷键 (Enter/M/Tab/Escape)
- 输入阻塞管理

当前 `draw()` 处理:
- z-ordering
- 鼠标命中检测
- 对话框渲染
- 死亡倒计时覆盖层

**拆分为**:
- `UISyncSystem` — 数据同步（逻辑阶段）
- `UIInputSystem` — 键盘快捷键处理
- `UIDrawSystem` — 渲染（Draw 阶段）

---

### 5.3 统一时间类型
当前 4 种时间表示:
- `u64` — `AIState.last_action_time`
- `Instant` — `Movement.last_change_time`, `CollisionInfo.last_update`
- `f32` — `Particle.alive_until` (from SystemTime)
- `f64` — `NpcCallCooldown.until` (from get_time)

**方案**: 统一使用 `f64` (from `macroquad::get_time()`) 作为游戏内时间标准。

---

### 5.4 LocalPlayerAiSystem 行为树提取
**文件**: `src/systems/input/local_player_ai_system.rs:32-158`

内嵌了完整的 BT 实现（BtNode, BehaviorTree, Blackboard）。提取到 `systems/input/behavior_tree.rs` 可复用。

---

### 5.5 GameShopDialog 中文搜索支持
**文件**: `src/scenes/dialogs/game/game_shop_dialog/interaction.rs:76-83`

当前 `key_to_char()` 仅支持 ASCII。改为使用 `get_char_pressed()` 或 egui 的 TextInput 组件。

---

### 5.6 键盘输入 Dialog 作用域化
当前 `AmountBox` 和 `ChatDialog` 在 `update_and_draw()` 中直接读全局键盘输入。

**方案**: 通过 FocusManager 控制，只有顶层 Dialog 才能消费键盘事件。

---

## 执行顺序建议

```
阶段一（正确性）
  └─ 1.1 负索引 bug          (~10 min)
  └─ 1.6 process::exit       (~5 min)
  └─ 1.7 粒子时间戳          (~10 min)
  └─ 1.3 MapUpdate 错误处理  (~20 min)
  └─ 1.4 碰撞 clone          (~30 min)
  └─ 1.5 伤害 target 匹配    (~20 min)
  └─ 1.2 DoT 帧率            (~30 min)

阶段二（功能完整）
  └─ 2.2 删除 inventory_persistence  (~5 min)
  └─ 2.5 密码日志安全        (~20 min)
  └─ 2.3 网络线程关闭        (~45 min)
  └─ 2.4 Dialog FocusManager (~60 min)
  └─ 2.1 Handler 补全        (~4-8h，分多次提交)

阶段三（性能）
  └─ 3.1 方向计算统一        (~30 min)
  └─ 3.3 derive 补全         (~10 min)
  └─ 3.7 除零防御            (~5 min)
  └─ 3.8 &mut 修复           (~5 min)
  └─ 3.4 Health/Mana u32     (~30 min)
  └─ 3.2 UI 性能优化         (~2h)
  └─ 3.5 容器去重            (~1h)
  └─ 3.6 EquipmentSlot       (~1h)

阶段四（清理）
  └─ 4.1 删除死代码          (~30 min)
  └─ 4.2 删除备份文件        (~5 min)
  └─ 4.7 格式修复            (~15 min)
  └─ 4.6 优先级注释          (~10 min)
  └─ 4.3 移除硬编码数据      (~1h)
  └─ 4.4 Dialog 接口统一     (~1h)
  └─ 4.5 注释语言统一        (渐进)

阶段五（架构）
  └─ 5.1 AnimationSystem 拆分 (~2h)
  └─ 5.2 UIRenderSystem 拆分  (~2h)
  └─ 5.3 时间类型统一         (~2h)
  └─ 5.4 行为树提取           (~1h)
  └─ 5.5 中文搜索             (~1h)
  └─ 5.6 键盘作用域           (~1h)
```

---

## 风险与注意事项

1. **阶段一改动虽小但影响面大** — 尤其是战斗系统和地图系统，修改后需充分测试
2. **阶段二 Handler 补全** — 建议按功能逐个提交，不要一次性补完所有 opcode，避免引入大量新 bug
3. **阶段三 Health/Mana u32** — 会波及所有扣血/加血逻辑，需全面 grep 适配
4. **阶段四 Dialog 接口统一** — 改动面广但不涉及核心逻辑，风险低
5. **阶段五 架构拆分** — 可能影响现有系统注册和优先级配置，需谨慎测试
6. **所有阶段** — 修改前确保 `cargo run --bin test_login` 和 `cargo run --bin test_game_scene` 能正常运行
