# MapObject 重构执行计划

**日期**: 2025-10-03  
**任务**: P0-2 MapObject 架构重构  
**预计时间**: 6-8 小时

---

## 🎯 重构目标

将 MapObject 从"存储网络包"的模式改为"扁平化存储游戏数据"的模式。

---

## 📋 当前结构分析

### 当前错误设计
```rust
enum MapObjectKind {
    Player(PlayerObject),      // ❌ 存储完整网络包
    Hero(HeroObject),          // ❌ 存储完整网络包
    Monster(ObjectMonster),    // ❌ 存储完整网络包
}

pub struct MapObject {
    kind: MapObjectKind,       // ❌ 包含网络包数据
    buffs: BuffState,
    animation: AnimationState,
    location: Point,
    direction: MirDirection,
    last_update: Instant,
}
```

### 问题
1. **数据重复**: location 和 direction 既在 MapObject 中，又在 kind 的网络包中
2. **职责混乱**: MapObject 应该是游戏对象，不应存储网络包
3. **依赖错误**: 游戏对象层依赖网络层

---

## ✅ 目标结构设计

### 新的扁平化设计
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapObjectType {
    User,      // 改名: Player → User
    Hero,
    Monster,
    NPC,       // 新增
}

pub struct MapObject {
    // === 标识 ===
    object_id: u32,
    object_type: MapObjectType,
    
    // === 位置和方向 ===
    location: Point,
    direction: MirDirection,
    
    // === 显示信息 ===
    name: String,
    name_colour: i32,
    
    // === 状态标志 (所有对象共有) ===
    dead: bool,
    hidden: bool,
    poison: PoisonType,
    
    // === 怪物/NPC 特有 (其他类型为默认值) ===
    ai: u8,
    light: u8,
    
    // === 私有状态 ===
    buffs: BuffState,
    animation: AnimationState,
    last_update: Instant,
}
```

---

## 🔧 实施步骤

### 步骤 1: 定义新结构 (30min)
- [x] 规划新 MapObject 字段
- [ ] 定义新的 MapObjectType enum
- [ ] 定义新的 MapObject struct
- [ ] 实现 Default trait

### 步骤 2: 实现工厂方法 (1h)
- [ ] `for_user(object_id, name) -> Self`
- [ ] `for_hero(object_id, name) -> Self`
- [ ] `for_monster(object_id, name) -> Self`
- [ ] `for_npc(object_id, name) -> Self`

### 步骤 3: 实现同步方法 (1-2h)
- [ ] `sync_from_player_packet(&mut self, packet: &PlayerObject)`
- [ ] `sync_from_hero_packet(&mut self, packet: &HeroObject)`
- [ ] `sync_from_monster_packet(&mut self, packet: &ObjectMonster)`
- [ ] `sync_from_npc_packet(&mut self, packet: &ObjectNpc)`

### 步骤 4: 实现 Getters (30min)
- [ ] `object_id() -> u32`
- [ ] `object_type() -> MapObjectType`
- [ ] `location() -> Point`
- [ ] `direction() -> MirDirection`
- [ ] `name() -> &str`
- [ ] `name_colour() -> i32`
- [ ] `is_dead() -> bool`
- [ ] `is_hidden() -> bool`
- [ ] `poison() -> PoisonType`
- [ ] `ai() -> u8`
- [ ] `light() -> u8`

### 步骤 5: 实现 Setters (30min)
- [ ] `set_location(&mut self, location: Point)`
- [ ] `set_direction(&mut self, direction: MirDirection)`
- [ ] `set_name(&mut self, name: String)`
- [ ] `set_name_colour(&mut self, colour: i32)`
- [ ] `set_dead(&mut self, dead: bool)`
- [ ] `set_hidden(&mut self, hidden: bool)`
- [ ] `set_poison(&mut self, poison: PoisonType)`

### 步骤 6: 实现动画方法 (30min)
- [ ] `current_action() -> MirAction`
- [ ] `set_action(&mut self, action: MirAction)`
- [ ] `advance_animation(&mut self, delta_ms: u32) -> AnimationStep`

### 步骤 7: 实现 Buff 方法 (30min)
- [ ] `has_buff(&self, buff_type: BuffType) -> bool`
- [ ] `buffs() -> &BuffState`
- [ ] `update_buffs(&mut self, buffs: &[BuffInfo]) -> BuffDelta`

### 步骤 8: 实现其他方法 (1h)
- [ ] `apply_attack(...) -> AttackOutcome`
- [ ] `apply_struck(...) -> StruckOutcome`
- [ ] `apply_action(...) -> ActionResult`
- [ ] 保留所有需要的辅助方法

### 步骤 9: 移除旧代码 (30min)
- [ ] 删除 MapObjectKind enum
- [ ] 删除 MapObjectKind 的所有方法
- [ ] 删除 from_player/from_hero/from_monster 方法
- [ ] 删除 sync_player/sync_hero/sync_monster 方法

### 步骤 10: 更新依赖文件 (2-3h)
- [ ] user_object.rs
- [ ] hero_object.rs
- [ ] monster_object.rs
- [ ] npc_object.rs
- [ ] item_object.rs
- [ ] spell_object.rs

### 步骤 11: 测试验证 (1h)
- [ ] cargo check 通过
- [ ] cargo test 通过
- [ ] 手动功能测试

---

## 📝 新 API 设计

### 初始化模式
```rust
// ✅ 新方式: 两步初始化
let mut map_object = MapObject::for_user(packet.object_id, packet.name.clone());
map_object.sync_from_player_packet(&packet);

// 或者一步初始化（保留便利方法）
let map_object = MapObject::from_player_packet(&packet);
```

### 更新模式
```rust
// ✅ 同步更新
map_object.sync_from_player_packet(&new_packet);
```

### 访问模式
```rust
// ✅ 通过 getters
let pos = map_object.location();
let is_dead = map_object.is_dead();
let name = map_object.name();
```

---

## 🔍 需要保留的关键方法

基于代码分析，以下方法被其他模块使用，必须保留：

```rust
// 标识和类型
pub fn object_id(&self) -> u32
pub fn object_type(&self) -> MapObjectType

// 位置和方向
pub fn location(&self) -> Point
pub fn direction(&self) -> MirDirection
pub fn set_location(&mut self, location: Point)
pub fn set_direction(&mut self, direction: MirDirection)

// 状态查询
pub fn is_dead(&self) -> bool
pub fn is_hidden(&self) -> bool
pub fn current_action(&self) -> MirAction

// 动画控制
pub fn advance(&mut self, delta_ms: u32) -> AnimationStep
pub fn apply_action(...) -> ActionResult
pub fn apply_attack(...) -> AttackOutcome
pub fn apply_struck(...) -> StruckOutcome

// 属性访问（部分可能需要移除，因为不是所有对象都有）
pub fn level(&self) -> u16                    // ❌ 移除（只有 Player 有）
pub fn set_level(&mut self, level: u16)      // ❌ 移除
pub fn guild_name(&self) -> &str              // ❌ 移除（只有 Player 有）
pub fn set_guild_name(&mut self, name: String) // ❌ 移除
pub fn name_colour_argb(&self) -> i32         // ✅ 保留（改名为 name_colour）
pub fn set_name_colour_argb(&mut self, i32)  // ✅ 保留
```

---

## ⚠️ 重要注意事项

### 1. 类型特定的字段不放在 MapObject
以下字段**不应该**放在 MapObject 中（只有特定类型才有）：

**Player/Hero 特有**:
- level, class, gender, hair
- weapon, armour, weapon_effect
- guild_name, guild_rank_name
- fishing, riding_mount, mount_type
- transform_type, wing_effect
- element_orb_*

**Monster 特有**:
- image, effect
- skeleton, shock_time
- binding_shot_center

这些字段应该保留在 UserObject, HeroObject, MonsterObject 中。

### 2. 共同字段才放在 MapObject
只有**所有**对象都有的字段才放在 MapObject：
- ✅ object_id, name, name_colour
- ✅ location, direction
- ✅ dead, hidden, poison
- ✅ buffs, animation

### 3. 数据流向
```
Network Packet (protocol::PlayerObject)
    ↓ sync_from_player_packet()
MapObject (基础数据)
    ↓ composition (has-a)
UserObject (extends with player-specific data)
```

---

## 📊 预期改进

| 指标 | 重构前 | 重构后 | 改进 |
|------|--------|--------|------|
| MapObject 字段数 | 6 | ~12 | 扁平化 |
| 依赖网络包 | ✅ Yes | ❌ No | ✅ 消除 |
| 数据重复 | ✅ Yes | ❌ No | ✅ 消除 |
| MapObjectKind 复杂度 | 7 变体 | 0 (移除) | ✅ 简化 |
| API 清晰度 | 混乱 | 清晰 | ✅ 改进 |

---

**开始时间**: 待定  
**状态**: 📋 计划完成，准备实施
