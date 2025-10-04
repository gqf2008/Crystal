# MirObjects 模块移植审查报告

**日期**: 2025年10月4日  
**目的**: 全面审查 MirObjects 模块移植完整性，建立规范的移植流程

---

## 一、C# vs Rust 文件对照表

| C# 文件 | Rust 文件 | 状态 | 完整度 | 依赖问题 |
|---------|-----------|------|--------|----------|
| **MapObject.cs** | map_object.rs | ✅ 存在 | ~70% | - |
| **PlayerObject.cs** | ❌ **缺失** | ⚠️ 缺失 | 0% | **严重** |
| **UserObject.cs** | user_object.rs | ✅ 存在 | ~40% | 22+ TODO |
| **HeroObject.cs** | hero_object.rs | ✅ 存在 | ~30% | 继承 PlayerObject |
| **UserHeroObject.cs** | ❌ **缺失** | ⚠️ 缺失 | 0% | 继承 HeroObject |
| **MonsterObject.cs** | monster_object.rs | ✅ 存在 | ~40% | 10+ TODO |
| **NPCObject.cs** | npc_object.rs | ✅ 存在 | ~60% | 2 TODO |
| **ItemObject.cs** | item_object.rs | ✅ 存在 | ~20% | 2 TODO |
| **SpellObject.cs** | spell_object.rs | ✅ 存在 | ~10% | 5+ TODO |
| **MapCode.cs** | map_code.rs | ✅ 存在 | ~60% | 5 TODO |
| **Frames.cs** | frames.rs | ✅ 存在 | ~80% | - |
| **PathFinder.cs** | pathfinder.rs | ✅ 存在 | ~50% | - |
| **Effect.cs** | effect.rs | ✅ 存在 | ~10% | - |
| **Damage.cs** | damage.rs | ✅ 存在 | ~10% | - |
| **DecoObject.cs** | ❌ **缺失** | ⚠️ 缺失 | 0% | - |

### 严重问题

1. **PlayerObject.cs 缺失** ⚠️⚠️⚠️
   - C# 中是 UserObject 和 HeroObject 的**基类**
   - 包含大量共享逻辑（ActionFeed, Movement, Combat 等）
   - UserObject/HeroObject 直接继承它
   - **影响**: UserObject/HeroObject 移植不完整

2. **UserHeroObject.cs 缺失**
   - 用户的英雄对象
   - 继承自 HeroObject

3. **DecoObject.cs 缺失**
   - 装饰物对象

---

## 二、依赖关系审查

### C# 继承层次结构

```
MapObject (基类)
├── PlayerObject (抽象基类)
│   ├── UserObject (玩家)
│   └── HeroObject (英雄基类)
│       └── UserHeroObject (用户的英雄)
├── MonsterObject (怪物)
├── NPCObject (NPC)
├── ItemObject (掉落物)
├── SpellObject (技能效果)
└── DecoObject (装饰物)
```

### Rust 当前状态（错误）

```
MapObject (基类)
├── UserObject ❌ 缺少 PlayerObject 层
├── HeroObject ❌ 缺少 PlayerObject 层
├── MonsterObject ✅
├── NPCObject ✅
├── ItemObject ⚠️ 不完整
├── SpellObject ⚠️ 不完整
└── DecoObject ❌ 缺失
```

### 严重依赖问题

**问题 1: 缺少 PlayerObject 层**
- UserObject 和 HeroObject 直接继承 MapObject
- 丢失了大量 PlayerObject 的共享逻辑：
  - ActionFeed 系统
  - QueuedAction 处理
  - Movement 逻辑
  - Combat 基础逻辑
  - Animation 框架

**问题 2: 模块间依赖不清晰**
- UserObject 依赖哪些 Shared 类型？未系统审查
- HeroObject 依赖哪些 Client 类型？未系统审查
- ItemObject/SpellObject 依赖未完整梳理

---

## 三、各模块 TODO 统计

### user_object.rs (22 个 TODO)
```
- Implement item binding (ItemInfo)
- Add guild buffs
- Apply percentage bonuses
- Apply stat caps
- Calculate level-based stats from CoreStats
- Set weapon, armour, mount type
- Distinguish hand weight vs wear weight
- stats.add(&item.stats)
- stats.add(&item.added_stats)
- Handle durability check
- Handle awakening stats
- Handle sockets
- Track item sets
- Implement item set bonus system
- Implement skill stat bonuses
- Iterate through active buffs
- stats.get(StatType::AttackSpeed)
- Calculate new max_experience based on level
- Play level up effects
- Show level up message
- Update stats
- Set action (movement)
```

### monster_object.rs (10 个 TODO)
```
- 完整的转换逻辑
- Calculate shock time properly
- Handle GreatFoxSpirit stage changes
- Set frames based on base_image
- Implement frame animation logic
- Implement sound playing
- Get current time and compare with shock_time
```

### map_code.rs (5 个 TODO)
```
- 实现对象排序逻辑
- 实现 Wemade AntiHack 格式 (Type 4)
- 实现 Wemade Mir3 格式 (Type 5)
- 实现 Shanda Mir3 格式 (Type 6)
- 实现 3/4 Heroes 格式 (Type 7)
- 实现 C# 自定义格式 (Type 100)
```

### spell_object.rs (5 个 TODO)
```
- ObjectSpell not yet implemented
- FrameSet not yet implemented
- Create explosion effect at target location
- Play hit sound
```

### hero_object.rs (3 个 TODO)
```
- Implement actual loyalty decrease logic
- Stats structure needs proper DC fields
- Calculate new max experience
- Increase stats based on class
```

### item_object.rs (2 个 TODO)
```
- Add to game scene map control
- Get item name from item database
```

### npc_object.rs (2 个 TODO)
```
- Add to game scene map control
- Randomly turn NPC to face different directions
```

**总计: ~50 个 TODO**

---

## 四、核心问题分析

### 问题 1: 缺少 PlayerObject 层

**影响范围**:
- UserObject 失去 ~40% 功能
- HeroObject 失去 ~40% 功能
- QueuedAction 处理逻辑不完整
- ActionFeed 系统缺失

**C# PlayerObject 包含**:
```csharp
public abstract class PlayerObject : MapObject
{
    // Movement system
    public Queue<QueuedAction> ActionFeed;
    public QueuedAction NextAction;
    
    // Animation
    public virtual void SetAction() { }
    public virtual void ProcessFrames() { }
    
    // Combat
    public virtual void PerformAction() { }
    public virtual void CompleteAttack() { }
    
    // State management
    public bool Dead;
    public bool Observer;
    
    // 300+ 行共享逻辑
}
```

**Rust 当前**: 这些逻辑散落在 UserObject/HeroObject 中，重复且不一致

### 问题 2: 依赖关系混乱

**示例 1: ItemSetStatus**
- 今天才发现应该从 SharedRust 导入
- 之前重复定义且不正确

**示例 2: QueuedAction**
- 使用了自创的 QueuedActionType
- 实际应该使用 MirAction

**示例 3: ClientMagic**
- 最近才修正为从 SharedRust 导入

### 问题 3: 移植不系统

**表现**:
- 边移植边改结构（如创建独立 map 模块）
- 没有完整对照 C# 逐个移植
- 缺少依赖关系审查流程
- TODO 占位符缺乏追踪

---

## 五、规范的移植流程（建议）

### 阶段 1: 准备阶段

**步骤 1.1: 审查 C# 模块结构**
```bash
1. 列出所有 C# 文件
2. 确认继承关系
3. 绘制依赖图
4. 识别共享逻辑
```

**步骤 1.2: 审查依赖关系**
```bash
1. 找出所有 using 语句
2. 区分 Shared vs Client 依赖
3. 检查 SharedRust 是否已有对应类型
4. 列出缺失的 SharedRust 类型
```

**步骤 1.3: 创建移植清单**
```markdown
- [ ] MapObject.cs → map_object.rs
- [ ] PlayerObject.cs → player_object.rs ⚠️ 缺失
- [ ] UserObject.cs → user_object.rs
- ...
```

### 阶段 2: 基础类优先

**顺序**:
1. ✅ MapObject (已完成 ~70%)
2. ❌ PlayerObject (**必须先完成**)
3. UserObject / HeroObject (依赖 PlayerObject)
4. MonsterObject / NPCObject (独立)
5. ItemObject / SpellObject (较简单)
6. DecoObject (最简单)

### 阶段 3: 单个模块移植流程

**步骤 3.1: 移植前检查**
```bash
# 1. 查看 C# 源码
- 确认类名、字段、方法
- 确认继承关系
- 确认依赖的其他类型

# 2. 检查依赖类型
grep "using" Client/MirObjects/XXXObject.cs
# 对每个依赖类型:
#   - 如果在 Shared → 检查 SharedRust
#   - 如果在 Client → 检查是否已移植
#   - 如果缺失 → 添加 TODO 占位符

# 3. 创建模块文件
touch src/objects/xxx_object.rs
```

**步骤 3.2: 移植结构定义**
```rust
// XXXObject.rs - <功能说明>
// Mirrors Client/MirObjects/XXXObject.cs

// Step 1: 导入依赖
use mir2_shared::{
    // 从 C# using 语句对照
};

// Step 2: 定义结构（完全对照 C# 字段）
pub struct XXXObject {
    // 每个字段添加注释说明对应 C# 字段
}

// Step 3: 占位 impl（先框架，后细节）
impl XXXObject {
    pub fn new() -> Self {
        // TODO: Implement
        unimplemented!()
    }
}
```

**步骤 3.3: 逐方法移植**
```rust
// 优先级:
// P0: 构造函数、load 方法
// P1: 核心逻辑方法
// P2: 辅助方法
// P3: 特殊功能

// 每个方法:
// 1. 添加注释说明对应 C# 方法
// 2. 列出外部依赖（TODO 占位）
// 3. 实现核心逻辑
// 4. 添加测试
```

**步骤 3.4: 审查和测试**
```bash
# 1. 对照 C# 源码逐行审查
# 2. 检查所有字段是否对应
# 3. 检查所有方法是否对应
# 4. 统计 TODO 数量
# 5. 编译测试
# 6. 创建移植报告
```

### 阶段 4: 集成测试

**步骤 4.1: 模块间联调**
```rust
// 测试各模块之间的交互
// 验证依赖关系正确性
```

**步骤 4.2: TODO 追踪**
```bash
# 创建 TODO 追踪表
# 按优先级排序
# 逐个解决
```

---

## 六、立即行动计划

### 🚨 紧急任务（本周）

#### Task 1: 移植 PlayerObject.cs ⚠️ **最高优先级**
```
文件: Client/MirObjects/PlayerObject.cs (5277 行)

包含内容:
- ActionFeed 系统 (~500 lines)
- QueuedAction 处理 (~200 lines)
- Movement 逻辑 (~800 lines)
- Combat 基础 (~600 lines)
- Animation 框架 (~400 lines)
- 各种状态管理 (~500 lines)

依赖审查:
1. 检查所有 using 语句
2. 确认哪些在 SharedRust
3. 列出缺失的类型
4. 添加 TODO 占位符

预计工作量: 2-3 天
```

#### Task 2: 重构 UserObject / HeroObject
```
在 PlayerObject 完成后:
1. 创建 player_object.rs
2. 将共享逻辑从 user_object.rs 移到 player_object.rs
3. 将共享逻辑从 hero_object.rs 移到 player_object.rs
4. 更新继承关系
5. 测试

预计工作量: 1-2 天
```

### 📋 中期任务（下周）

#### Task 3: 补全缺失模块
```
- [ ] UserHeroObject.cs → user_hero_object.rs
- [ ] DecoObject.cs → deco_object.rs
```

#### Task 4: 完善现有模块
```
- [ ] MonsterObject (10 TODO)
- [ ] ItemObject (2 TODO)
- [ ] SpellObject (5 TODO)
- [ ] MapCode (5 TODO)
```

### 🔧 长期任务（未来）

#### Task 5: 解决所有 TODO
```
按模块逐个解决 TODO
优先级: 核心逻辑 > 辅助功能 > 特殊功能
```

---

## 七、依赖关系检查清单（模板）

### 移植 XXXObject.cs 前的检查清单

```markdown
## 模块: XXXObject

### 1. 文件信息
- [ ] C# 文件路径: Client/MirObjects/XXXObject.cs
- [ ] C# 文件行数: _____ lines
- [ ] 继承关系: 继承自 _____
- [ ] 实现接口: _____

### 2. 依赖类型审查

#### 2.1 Shared 项目依赖
- [ ] 列出所有 using Shared.* 语句
- [ ] 对每个类型检查 SharedRust 是否有对应
- [ ] 列出缺失的 SharedRust 类型

| C# 类型 | SharedRust 类型 | 状态 |
|---------|-----------------|------|
| XXX | ✅ YYY | 已有 |
| ZZZ | ❌ 缺失 | TODO |

#### 2.2 Client 项目依赖
- [ ] 列出所有引用的 Client 类型
- [ ] 检查这些类型是否已移植

| C# 类型 | ClientRust 文件 | 状态 |
|---------|-----------------|------|
| ItemInfo | ❌ 未移植 | TODO |

#### 2.3 System 依赖
- [ ] 确认使用的系统功能
- [ ] 确认 Rust 标准库对应

### 3. 字段清单
```rust
pub struct XXXObject {
    // 每个字段对应 C# 字段
    pub field1: Type1,  // C#: Type1 Field1
    pub field2: Type2,  // C#: Type2 Field2
}
```

### 4. 方法清单
- [ ] Method1() - 对应 C# XXX
- [ ] Method2() - 对应 C# YYY
- ...

### 5. TODO 追踪
| TODO 描述 | 优先级 | 依赖 | 预计完成 |
|-----------|--------|------|----------|
| Implement XXX | P0 | 需要 YYY | Week 1 |

### 6. 测试计划
- [ ] 单元测试: test_xxx
- [ ] 集成测试: test_xxx_integration
```

---

## 八、经验教训

### ❌ 错误做法

1. **过早重构**: 创建独立 map 模块（游戏只完成 15%）
2. **边移植边改**: 没有严格对照 C# 结构
3. **忽略继承关系**: 缺少 PlayerObject 层
4. **依赖关系不清**: 重复定义 SharedRust 类型
5. **缺乏系统性**: 随意选择移植顺序

### ✅ 正确做法

1. **先审查后移植**: 完整了解 C# 结构再开始
2. **严格对照**: 逐字段、逐方法对照移植
3. **遵循继承**: 先移植基类，再移植派生类
4. **依赖优先**: 先检查 SharedRust，避免重复定义
5. **系统推进**: 按优先级和依赖关系移植

### 核心原则

1. **完整性优先**: 先保证结构完整，再优化细节
2. **依赖关系清晰**: Client → Shared, ClientRust → SharedRust
3. **保持一致性**: 严格对照 C# 项目结构
4. **TODO 可追踪**: 每个 TODO 说明依赖和优先级
5. **文档驱动**: 每次移植创建审查文档

---

## 九、总结

### 当前状态评估

| 维度 | 评分 | 说明 |
|------|------|------|
| **结构完整性** | ⭐⭐☆☆☆ | 缺少 PlayerObject/UserHeroObject/DecoObject |
| **代码质量** | ⭐⭐⭐☆☆ | 存在重复定义，TODO 较多 |
| **依赖关系** | ⭐⭐☆☆☆ | 依赖关系审查不足 |
| **文档完善** | ⭐⭐⭐⭐☆ | 最近文档质量提升 |
| **可维护性** | ⭐⭐☆☆☆ | 缺少系统性，难以维护 |

**总体评分: 2.4/5 ⭐⭐☆☆☆**

### 核心问题

1. ⚠️ **缺少 PlayerObject 层** - 影响 30-40% 功能
2. ⚠️ **模块不完整** - 缺少 3 个模块
3. ⚠️ **依赖关系混乱** - 重复定义，依赖不清
4. ⚠️ **移植无序** - 缺乏系统性流程

### 改进方向

1. ✅ **建立规范流程**: 本文档提供的移植流程
2. ✅ **先基础后应用**: PlayerObject → UserObject/HeroObject
3. ✅ **依赖审查优先**: 先检查 SharedRust
4. ✅ **TODO 追踪**: 系统化管理 TODO
5. ✅ **文档驱动**: 每次移植创建审查报告

---

**下一步行动**:
1. 🚨 立即开始移植 PlayerObject.cs（最高优先级）
2. 📋 创建详细的 PlayerObject 依赖审查清单
3. 🔍 审查所有现有模块的依赖关系
4. 📝 为每个模块创建移植报告
5. 🧹 清理所有不符合规范的代码

**预计时间表**:
- Week 1: PlayerObject 移植 + UserObject/HeroObject 重构
- Week 2: 补全缺失模块 + 完善现有模块
- Week 3-4: 解决所有 TODO

**目标**: 将 MirObjects 完整性从 40% 提升到 90%+
