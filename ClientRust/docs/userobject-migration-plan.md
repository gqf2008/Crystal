# UserObject 移植计划

**日期**: 2025-10-04  
**目标**: 完成 UserObject 的核心功能移植  
**优先级**: P0 (游戏运行必须)

---

## 📊 当前状态

### ✅ 已完成
- 基础数据结构 (字段定义)
- 构造函数 `new()`
- Load 方法框架
- 基础工具方法 (get_inventory_item, calculate_weight 等)
- 单元测试框架

### ⏳ 待完成（4个 TODO）
1. `load()` - 加载 magics, item sets 等
2. `refresh_stats()` - 添加装备属性到 stats
3. `refresh_stats()` - 添加 buff 属性
4. `gain_experience()` - 检查升级

---

## 📋 C# UserObject 核心方法清单

### 数据加载 (P0)
- [x] `Load(UserInformation)` - 框架完成，需完善
- [ ] `SetSlots(UserSlotsRefresh)` - 刷新装备和背包

### 属性刷新 (P0)
- [x] `RefreshStats()` - 框架完成，需完善细节
- [ ] `RefreshLevelStats()` - 等级属性
- [ ] `RefreshBagWeight()` - 背包重量
- [ ] `RefreshEquipmentStats()` - 装备属性
- [ ] `RefreshSocketStats()` - 宝石属性
- [ ] `RefreshItemSetStats()` - 套装属性
- [ ] `RefreshMirSetStats()` - Mir套装
- [ ] `RefreshSkills()` - 技能加成
- [ ] `RefreshBuffs()` - Buff加成
- [ ] `RefreshGuildBuffs()` - 公会Buff
- [ ] `RefreshStatCaps()` - 属性上限

### 物品管理 (P1)
- [ ] `BindAllItems()` - 绑定物品信息
- [ ] `GetMaxGain(UserItem)` - 计算物品增益

### 动作和渲染 (P0)
- [ ] `SetAction()` - 设置动作 (override)
- [ ] `ProcessFrames()` - 处理动画帧 (override)
- [ ] `SetLibraries()` - 设置贴图库 (override)
- [ ] `SetEffects()` - 设置特效 (override)

### 魔法和技能 (P1)
- [ ] `ClearMagic()` - 清除魔法状态
- [ ] 魔法冷却管理
- [ ] 技能状态管理

---

## 🎯 分阶段实施计划

### Phase 1: 核心数据加载 (今天完成)

**目标**: 让 UserObject 能正确加载并显示

#### 1.1 完成 `load()` 方法
```rust
pub fn load(&mut self, info: &UserInformation) {
    // ✅ 已完成: 基础数据
    // ✅ 已完成: 背包和装备
    // TODO: 加载 magics
    // TODO: 加载 intelligent creatures
    // TODO: 调用 bind_all_items()
    // TODO: 调用 refresh_stats()
    // TODO: 调用 set_action()
}
```

#### 1.2 实现 `bind_all_items()`
```rust
fn bind_all_items(&mut self) {
    // 遍历所有物品，关联 ItemInfo
    // C# 中从 GameScene.ItemInfoList 获取
    // Rust 中可能需要从全局状态或参数传入
}
```

#### 1.3 实现 `set_action()` (override)
```rust
pub fn set_action(&mut self) {
    // 设置默认站立动作
    // 调用 map_object.set_action()
}
```

### Phase 2: 属性刷新系统 (明天完成)

**目标**: 正确计算角色属性

#### 2.1 实现 `refresh_stats()` 完整逻辑
```rust
pub fn refresh_stats(&mut self) {
    self.stats.clear();
    
    self.refresh_level_stats();
    self.refresh_bag_weight();
    self.refresh_equipment_stats();
    self.refresh_item_set_stats();
    self.refresh_mir_set_stats();
    self.refresh_skills();
    self.refresh_buffs();
    self.refresh_guild_buffs();
    
    self.set_libraries();
    self.set_effects();
    
    // 百分比加成
    self.apply_rate_bonuses();
    
    // 上限限制
    self.refresh_stat_caps();
    
    // 攻击速度计算
    self.calculate_attack_speed();
}
```

#### 2.2 实现各个子方法
- `refresh_level_stats()` - 根据等级和职业计算基础属性
- `refresh_bag_weight()` - ✅ 基本完成
- `refresh_equipment_stats()` - 遍历装备，累加属性
- `refresh_socket_stats()` - 宝石属性
- `refresh_item_set_stats()` - 套装加成
- `refresh_mir_set_stats()` - Mir套装
- `refresh_skills()` - 技能加成
- `refresh_buffs()` - Buff加成
- `refresh_guild_buffs()` - 公会Buff
- `refresh_stat_caps()` - 属性上限

### Phase 3: 动作和渲染 (后天完成)

#### 3.1 实现 `set_libraries()` (override)
```rust
pub fn set_libraries(&mut self) {
    // 调用父类 (MapObject)
    // 设置武器、盔甲贴图
}
```

#### 3.2 实现 `set_effects()` (override)
```rust
pub fn set_effects(&mut self) {
    // 调用父类
    // 设置翅膀、武器特效
}
```

#### 3.3 实现 `process_frames()` (override)
```rust
pub fn process_frames(&mut self) {
    // 处理动画帧
    // 调用 map_object 的动画系统
}
```

### Phase 4: 物品和魔法 (可选，P1)

#### 4.1 完善物品管理
- `set_slots()` - 刷新物品槽
- `get_max_gain()` - 计算物品增益

#### 4.2 完善魔法系统
- `clear_magic()` - 清除魔法状态
- 冷却管理
- 技能状态

---

## 🔧 实施策略

### 原则
1. ✅ **先让编译通过** - 空实现 > 没有实现
2. ✅ **最小可用版本** - 只实现游戏运行必须的功能
3. ✅ **渐进式完善** - 先基础，后细节
4. ✅ **保持与 C# 一致** - 方法名、逻辑流程对应

### 优先级
- **P0** (今天): load(), bind_all_items(), set_action(), 基础 refresh_stats()
- **P1** (明天): 完整 refresh_stats() 和所有子方法
- **P2** (后天): set_libraries(), set_effects(), process_frames()
- **P3** (未来): 物品增益计算、高级魔法功能

---

## 📝 关键挑战

### 1. Stats 系统
C# 中 `Stats` 类有很多方法:
```csharp
Stats.Add(realItem.Stats);
Stats.Add(temp.AddedStats);
Stats[Stat.HP] += value;
```

Rust 中需要实现类似的接口。

**解决方案**: 在 `SharedRust` 的 `Stats` 中添加方法：
- `add(&self, other: &Stats)` - 累加属性
- `clear(&mut self)` - 清空属性
- Index trait 实现 `stats[StatType::HP]`

### 2. ItemInfo 查找
C# 中从 `GameScene.ItemInfoList` 获取物品信息。

**解决方案**:
- 暂时跳过，使用 `UserItem.info` 字段
- 或在 GameScene 中维护全局 ItemInfo 列表

### 3. 套装系统
C# 中有复杂的套装判定逻辑。

**解决方案**:
- Phase 1: 简化实现或跳过
- Phase 2: 完整移植套装逻辑

### 4. Buff 系统
Buff 的属性加成需要根据 BuffType 判断。

**解决方案**:
- 先实现基础框架
- 逐步添加各种 Buff 的效果

---

## 📊 预计工作量

| 阶段 | 任务 | 预计时间 | 优先级 |
|------|------|----------|--------|
| Phase 1 | 核心数据加载 | 2-3 小时 | P0 |
| Phase 2 | 属性刷新系统 | 4-6 小时 | P0 |
| Phase 3 | 动作和渲染 | 2-3 小时 | P0 |
| Phase 4 | 物品和魔法 | 3-4 小时 | P1 |
| **总计** | | **11-16 小时** | **约 2 天** |

---

## ✅ 成功标准

### 最小可用版本 (MVP)
- [ ] UserObject 能从 UserInformation 加载
- [ ] 能显示在地图上
- [ ] 能正确计算基础属性 (HP, MP, 攻击力等)
- [ ] 装备能正确显示
- [ ] 能响应移动命令

### 完整版本
- [ ] 所有属性计算正确
- [ ] 套装加成生效
- [ ] Buff 加成生效
- [ ] 技能状态正确
- [ ] 物品增益正确

---

## 🚀 立即开始

现在开始 **Phase 1: 核心数据加载**

### 第一步：完善 `load()` 方法

需要添加：
1. 加载 magics (with cooldown adjustment)
2. 加载 intelligent creatures
3. 调用 bind_all_items() (先空实现)
4. 调用 refresh_stats() (使用现有框架)
5. 调用 set_action() (先空实现)

### 第二步：实现空方法
1. `bind_all_items()` - 空实现，添加 TODO
2. `set_action()` - 调用 map_object 的方法

### 第三步：测试编译
确保没有编译错误。

---

**准备好了吗？** 让我们开始编码！ 🚀
