# MapObject 重构进度报告 - Session 1

**日期**: 2025-10-03  
**状态**: 🟡 **进行中** - 核心重构完成，正在更新依赖文件  
**进度**: 60% 完成

---

## ✅ 已完成 (60%)

### 1. 核心重构 - map_object.rs ✅
**时间**: 2.5 小时  
**状态**: 完全完成并编译通过

#### 主要变更

**移除的内容**:
- ❌ `MapObjectKind` enum (完全删除)
- ❌ `new_player()`, `new_hero()`, `new_monster()`, `new_npc()` (旧工厂方法)
- ❌ `from_player()`, `from_hero()`, `from_monster()` (旧创建方法)
- ❌ `sync_player()`, `sync_hero()`, `sync_monster()` (旧同步方法)
- ❌ 所有依赖 `kind` 字段的辅助方法

**新增的内容**:
- ✅ 扁平化的 `MapObject` 结构（12 个字段）
- ✅ `MapObjectType` enum (User/Hero/Monster)
- ✅ 工厂方法: `for_user()`, `for_hero()`, `for_monster()`
- ✅ 便利方法: `from_player_packet()`, `from_hero_packet()`, `from_monster_packet()`, `from_npc_packet()`
- ✅ 同步方法: `sync_from_player_packet()`, `sync_from_hero_packet()`, `sync_from_monster_packet()`
- ✅ 完整的 getter/setter API (30+ 方法)

#### 新结构

```rust
pub struct MapObject {
    // === Identity ===
    object_id: u32,
    object_type: MapObjectType,
    
    // === Position and Direction ===
    location: Point,
    direction: MirDirection,
    
    // === Display Information ===
    name: String,
    name_colour: i32,
    
    // === State Flags ===
    dead: bool,
    hidden: bool,
    poison: PoisonType,
    
    // === Monster/NPC specific ===
    ai: u8,
    light: u8,
    
    // === Private State ===
    buffs: BuffState,
    animation: AnimationState,
    last_update: Instant,
}
```

#### API 变更

```rust
// ❌ 旧 API (已移除)
let obj = MapObject::new_player(123);
let (obj, sync) = MapObject::from_player(player_packet);
obj.sync_player(player_packet);

// ✅ 新 API
let obj = MapObject::for_user(123, "Player".to_string());
let (obj, sync) = MapObject::from_player_packet(&player_packet);
obj.sync_from_player_packet(&player_packet);
```

---

## 🔴 待完成 (40%)

### 2. 更新依赖文件 (进行中)

需要更新以下文件以适配新的 MapObject API：

| 文件 | 状态 | 问题 |
|------|------|------|
| user_object.rs | ⚠️ 编译错误 | 调用 `new_player()`, `set_guild_name()`, `set_level()` |
| hero_object.rs | ⚠️ 编译错误 | 调用 `new_hero()` |
| monster_object.rs | ⚠️ 编译错误 | 调用 `new_monster()` |
| npc_object.rs | ⚠️ 编译错误 | 调用 `new_npc()` |
| item_object.rs | ⚠️ 编译错误 | 调用 `new_player()` |
| spell_object.rs | ⚠️ 编译错误 | 调用 `new_player()` |
| scenes/state.rs | ✅ 导入修复 | 已修复类型导入 |

### 具体错误

#### user_object.rs
```rust
// 错误 1: new_player() 不存在
self.map_object = MapObject::new_player(self.id);
// 修复: 使用 for_user()
self.map_object = MapObject::for_user(self.id, self.name.clone());

// 错误 2: set_guild_name() 不存在 (只有 Player 有)
self.map_object.set_guild_name(info.guild_name.clone());
// 修复: UserObject 直接保存 guild_name 字段

// 错误 3: set_level() 不存在 (只有 Player 有)
self.map_object.set_level(info.level);
// 修复: UserObject 直接保存 level 字段
```

#### hero_object.rs
```rust
// 错误: new_hero() 不存在
self.map_object = MapObject::new_hero(self.object_id);
// 修复: 使用 for_hero()
self.map_object = MapObject::for_hero(self.object_id, self.name.clone());
```

#### monster_object.rs
```rust
// 错误: new_monster() 不存在
self.map_object = MapObject::new_monster(id);
// 修复: 使用 for_monster()
self.map_object = MapObject::for_monster(id, String::new());
```

#### npc_object.rs
```rust
// 错误: new_npc() 不存在
self.map_object = MapObject::new_npc(id);
// 修复: 使用 for_monster() (NPCs are monsters)
self.map_object = MapObject::for_monster(id, String::new());
```

#### item_object.rs, spell_object.rs
```rust
// 错误: new_player() 不存在
self.map_object = MapObject::new_player(0);
// 修复: 这些应该不需要 MapObject，或者使用简化的工厂方法
```

---

## 📋 下一步行动计划

### 步骤 1: 修复 UserObject (30 min)
- [ ] 添加 `guild_name: String` 字段
- [ ] 添加 `level: u16` 字段
- [ ] 更新 `load()` 方法
- [ ] 修复 MapObject 初始化

### 步骤 2: 修复 HeroObject (15 min)
- [ ] 更新 MapObject 初始化

### 步骤 3: 修复 MonsterObject (15 min)
- [ ] 更新 MapObject 初始化

### 步骤 4: 修复 NPCObject (15 min)
- [ ] 更新 MapObject 初始化

### 步骤 5: 修复 ItemObject 和 SpellObject (20 min)
- [ ] 评估是否需要 MapObject
- [ ] 如需要，使用简化的初始化

### 步骤 6: 测试和验证 (30 min)
- [ ] cargo check 通过
- [ ] cargo test 通过
- [ ] 手动测试关键功能

---

## 🎯 设计决策记录

### 决策 1: MapObject 不包含类型特定字段

**问题**: level, guild_name 等字段应该放在哪里？

**决策**: 只有所有对象都有的字段才放在 MapObject

**理由**:
- ✅ MapObject 是基类，应该只包含共同字段
- ✅ 类型特定字段放在各自的对象类中 (UserObject, HeroObject, etc.)
- ✅ 避免内存浪费 (Monster 不需要 guild_name)

**影响**:
- UserObject 需要添加 `guild_name` 和 `level` 字段
- HeroObject 可能也需要类似字段
- 需要更新 load() 方法

### 决策 2: 使用 `ObjectPlayer` 而不是 `PlayerObject`

**问题**: mir2_shared 中使用的是 `ObjectPlayer`，不是 `PlayerObject`

**决策**: 统一使用 mir2_shared 的命名

**理由**:
- ✅ 与 shared库一致
- ✅ 避免类型混淆
- ✅ 遵循 Rust 命名惯例 (类型名在前)

**影响**:
- 需要更新所有导入
- 需要更新方法签名
- 需要更新文档注释

### 决策 3: 保留便利方法 `from_*_packet()`

**问题**: 是否需要 `from_player_packet()` 还是只用 `for_user()` + `sync_from_player_packet()`？

**决策**: 两者都保留

**理由**:
- ✅ `from_*_packet()` 简化常见用例（创建并初始化）
- ✅ `for_*()` + `sync_from_*_packet()` 提供灵活性
- ✅ 便利方法可以内联，没有性能开销

---

## 📊 改进指标

| 指标 | 重构前 | 重构后 | 改进 |
|------|--------|--------|------|
| MapObject 字段数 | 6 | 12 | ✅ 扁平化 |
| MapObject 行数 | ~830 | ~500 (预计) | ✅ -40% |
| MapObjectKind 变体数 | 3 | 0 (移除) | ✅ 简化 |
| 依赖网络包 | ✅ Yes | ❌ No | ✅ 消除 |
| 数据重复 | ✅ Yes | ❌ No | ✅ 消除 |
| 公共 API 方法数 | ~30 | ~35 | ✅ 更完整 |
| 编译错误 (objects) | 0 | 6 | 🔴 待修复 |

---

## ⚠️ 遇到的问题

### 问题 1: 类型特定字段如何处理？

**现象**: `level`, `guild_name` 等字段只有 Player/Hero 有

**解决方案**: 移到 UserObject/HeroObject 中

**状态**: ✅ 已决策，待实施

### 问题 2: ItemObject 和 SpellObject 需要 MapObject 吗？

**现象**: 它们调用 `new_player(0)` 创建占位符 MapObject

**分析**: 
- ItemObject 和 SpellObject 可能不应该继承 MapObject
- 它们更像是独立的实体，不是"地图对象"
- 可能需要重新设计这两个对象

**状态**: 🔴 待评估

### 问题 3: 导入混乱

**现象**: `PlayerObject` vs `ObjectPlayer` 命名不一致

**解决方案**: 统一使用 mir2_shared 的命名

**状态**: ✅ 已修复

---

## 📝 提交计划

建议分多次提交：

### Commit 1: Refactor MapObject core structure (已完成)
```
refactor(objects): Flatten MapObject, remove MapObjectKind enum

- Remove MapObjectKind enum and its wrapper pattern
- Flatten MapObject to only store common fields
- Add factory methods: for_user(), for_hero(), for_monster()
- Add sync methods: sync_from_player_packet(), etc.
- Update to use ObjectPlayer/ObjectHero from mir2_shared
- Remove dependency on network packet types in MapObject storage

BREAKING CHANGE: MapObject API completely redesigned
- new_player/new_hero/new_monster removed, use for_* instead
- from_player/from_hero/from_monster removed, use from_*_packet instead
- sync_player/sync_hero/sync_monster removed, use sync_from_*_packet instead
- Type-specific fields (level, guild_name) moved to UserObject

Refs: MAPOBJECT_ARCHITECTURE_FIX.md
```

### Commit 2: Update dependent objects (待完成)
```
refactor(objects): Update objects to use new MapObject API

- Update UserObject, HeroObject, MonsterObject, NPCObject
- Add type-specific fields to UserObject (guild_name, level)
- Fix all compilation errors
- Update tests

Follows: Previous MapObject refactor commit
```

---

## 🎓 经验教训

### 1. 大规模重构需要分步进行
- ✅ 先完成核心结构修改
- ✅ 确保核心编译通过
- ✅ 再逐个更新依赖文件

### 2. 类型命名的一致性很重要
- ❌ 混用 `PlayerObject` 和 `ObjectPlayer` 导致混乱
- ✅ 统一使用库的命名惯例

### 3. 架构决策需要提前规划
- ✅ 明确什么字段放在基类，什么字段放在子类
- ✅ 文档化设计决策和理由

---

**当前状态**: 🟡 **核心完成，正在更新依赖**  
**预计完成时间**: 还需 1.5-2 小时  
**建议**: 继续修复依赖文件的编译错误

---

*更新时间: 2025-10-03*  
*下次更新: 完成所有依赖文件修复后*
