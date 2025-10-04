# 依赖关系纠正日志

**日期**: 2025年10月4日  
**问题**: ItemSets, QueuedAction, QueuedActionType 定义错误  
**严重性**: 低级错误 - 未遵循 C# 项目的依赖关系

---

## 问题分析

### 1. ItemSets 错误

**错误定义** (ClientRust):
```rust
// 自创的结构，完全不对
pub struct ItemSets {
    pub set_id: i32,
    pub set_name: String,
    pub parts_equipped: i32,
    pub full_set: bool,
}
```

**正确定义** (C# Shared/Data/ItemData.cs):
```csharp
public class ItemSets
{
    public ItemSet Set;
    public List<ItemType> Type;
    private byte Amount { get; }
    public bool SetComplete { get; }
}
```

**Rust 对应** (SharedRust/src/data/item.rs):
```rust
pub struct ItemSetStatus {
    pub set: ItemSet,
    pub types: Vec<ItemType>,
    pub count: u8,
}
```

**结论**: ItemSets 是 **Shared 项目**的结构，应该从 SharedRust 导入 `ItemSetStatus`

---

### 2. QueuedActionType 错误

**错误定义** (ClientRust):
```rust
// 完全是自己发明的！C# 中不存在这个枚举
pub enum QueuedActionType {
    Move,
    Attack,
    Spell,
    Harvest,
}
```

**正确定义** (C# Client/MirObjects/PlayerObject.cs):
```csharp
public class QueuedAction
{
    public MirAction Action;  // ← 使用 MirAction，不是什么 QueuedActionType
    public Point Location;
    public MirDirection Direction;
    public List<object> Params;
}
```

**结论**: QueuedActionType 根本不存在！应该使用 **MirAction**（SharedRust/src/enums.rs）

---

### 3. QueuedAction 错误

**错误定义** (ClientRust):
```rust
pub struct QueuedAction {
    pub action_type: QueuedActionType,  // ← 错误：使用了不存在的类型
    pub location: Point,
    pub direction: MirDirection,
}
```

**正确定义** (Rust 修正后):
```rust
pub struct QueuedAction {
    pub action: MirAction,      // ← 正确：使用 MirAction
    pub location: Point,
    pub direction: MirDirection,
    // pub params: Vec<Box<dyn Any>>,  // C#: List<object> Params
    // Note: params 字段暂时省略，C# 中也很少使用
}
```

**结论**: QueuedAction 是 **Client 项目**的结构，但应该使用 SharedRust 的 MirAction 类型

---

## C# 项目依赖关系

```
Shared 项目 (基础层)
├── Enums.cs (MirAction, MirDirection, ...)
├── Data/ItemData.cs (ItemSets, UserItem, ...)
└── ...

Client 项目 (依赖 Shared)
├── MirObjects/
│   ├── MapObject.cs (使用 Shared 的 MirAction)
│   ├── PlayerObject.cs (定义 QueuedAction, 使用 Shared 的 MirAction)
│   └── UserObject.cs (使用 Shared 的 ItemSets)
└── ...
```

**依赖规则**:
- Client **依赖** Shared
- Client 可以使用 Shared 的所有类型
- Client 可以定义自己的类型 (如 QueuedAction)
- Client **不能**重复定义 Shared 的类型 (如 ItemSets)
- Client 定义的类型**必须**使用 Shared 的类型 (如 MirAction)

---

## Rust 项目依赖关系

```
SharedRust (基础层)
├── src/enums.rs (MirAction, MirDirection, ...)
├── src/data/item.rs (ItemSetStatus, UserItem, ...)
└── ...

ClientRust (依赖 SharedRust)
├── src/objects/
│   ├── map_object.rs (使用 SharedRust 的 MirAction)
│   ├── user_object.rs (使用 SharedRust 的 ItemSetStatus, MirAction)
│   │                   定义 QueuedAction (Client 特有)
│   └── ...
└── ...
```

---

## 修复内容

### 1. user_object.rs - imports

**修复前**:
```rust
use mir2_shared::{
    data::{
        stats::Stats, 
        client_data::{ClientMagic, ...},
    },
    enums::{MirDirection, Spell, ...},  // ← 缺少 MirAction
    Point, UserItem,
};
```

**修复后**:
```rust
use mir2_shared::{
    data::{
        stats::Stats, 
        client_data::{ClientMagic, ...},
        item::ItemSetStatus,  // ← 添加 ItemSetStatus
    },
    enums::{MirDirection, MirAction, Spell, ...},  // ← 添加 MirAction
    Point, UserItem,
};
```

### 2. user_object.rs - UserObject 字段

**修复前**:
```rust
pub struct UserObject {
    pub magics: Vec<ClientMagic>,
    pub item_sets: Vec<ItemSets>,  // ← 错误类型
    pub mir_set: Vec<EquipmentSlot>,
}
```

**修复后**:
```rust
pub struct UserObject {
    pub magics: Vec<ClientMagic>,
    pub item_sets: Vec<ItemSetStatus>,  // ← 正确：使用 SharedRust 的类型
    pub mir_set: Vec<EquipmentSlot>,
}
```

### 3. user_object.rs - 删除错误定义

**删除的代码** (~30 lines):
```rust
// ❌ 完全错误的定义
pub struct ItemSets {
    pub set_id: i32,
    pub set_name: String,
    pub parts_equipped: i32,
    pub full_set: bool,
}

pub struct QueuedAction {
    pub action_type: QueuedActionType,  // ← 使用不存在的类型
    pub location: Point,
    pub direction: MirDirection,
}

pub enum QueuedActionType {  // ← 根本不存在！
    Move,
    Attack,
    Spell,
    Harvest,
}
```

**替换为** (~10 lines):
```rust
// ✅ 正确的定义
/// Queued action for delayed execution
/// Mirrors C#: Client/MirObjects/PlayerObject.cs QueuedAction class
#[derive(Debug, Clone)]
pub struct QueuedAction {
    pub action: MirAction,      // ← 正确：使用 SharedRust 的 MirAction
    pub location: Point,
    pub direction: MirDirection,
    // pub params: Vec<Box<dyn Any>>,  // C#: List<object> Params
    // Note: C# rarely uses Params field, so we omit it for now
}
```

### 4. objects/mod.rs - 导出

**修复前**:
```rust
pub use user_object::{
    UserObject, ItemSets, QueuedAction, QueuedActionType,  // ← 导出错误类型
};
```

**修复后**:
```rust
pub use user_object::{
    UserObject, QueuedAction,  // ← 只导出 Client 特有的类型
};

// Re-export SharedRust types used by UserObject
// (follows C# dependency: Client depends on Shared)
pub use mir2_shared::data::item::ItemSetStatus;  // C# Shared/Data/ItemData.cs ItemSets
```

---

## 验证结果

### 编译测试
```
✅ cargo build
   Compiling mir2_client v0.1.0
   Finished `dev` profile in 5.25s
   441 warnings, 0 errors
```

### 单元测试
```
✅ cargo test
   test objects::user_object::tests::test_inventory_operations ... ok
   test objects::user_object::tests::test_user_object_creation ... ok
```

---

## 经验教训

### 为什么会犯这种低级错误？

1. **未仔细查看 C# 源码**: 看到字段名 `ItemSets` 就假设是本地类型，没有搜索定义位置
2. **发明了不存在的类型**: QueuedActionType 完全是自己编造的
3. **忽略依赖关系**: 没有检查 C# 项目的 Project References
4. **未遵循 "先检查 SharedRust" 原则**: 定义前应该先搜索 SharedRust

### 如何避免此类错误？

#### ✅ 正确的工作流程

**步骤 1: 看到新类型时，先搜索 C# 定义**
```bash
# 在 C# 项目中搜索
grep -r "class ItemSets" Shared/
grep -r "class QueuedAction" Client/
```

**步骤 2: 确认定义位置和依赖关系**
- 在 `Shared/` 中定义 → 应该在 SharedRust 中查找
- 在 `Client/` 中定义 → 可以在 ClientRust 中定义
- 检查使用的类型是否来自 Shared

**步骤 3: 在 Rust 中搜索对应类型**
```bash
# 在 SharedRust 中搜索
grep -r "struct ItemSets" SharedRust/src/
grep -r "enum MirAction" SharedRust/src/
```

**步骤 4: 对比 C# 和 Rust 定义**
- 字段是否匹配？
- 类型是否对应？
- 是否有注释说明？

**步骤 5: 定义前 Checklist**
- [ ] 检查 C# 定义位置 (Shared or Client?)
- [ ] 检查 SharedRust 是否已有定义
- [ ] 确认字段类型来自哪里
- [ ] 确认是否需要重新导出
- [ ] 添加注释说明对应的 C# 文件

#### ❌ 错误的做法

1. ~~看到字段名就直接定义结构~~
2. ~~自己发明类型名 (如 QueuedActionType)~~
3. ~~不看 C# 源码就猜测结构~~
4. ~~不检查 SharedRust 就重复定义~~

---

## 清理统计

| 项目 | 删除行数 | 原因 |
|------|----------|------|
| ItemSets 错误定义 | 6 lines | 应该使用 SharedRust 的 ItemSetStatus |
| QueuedActionType 错误定义 | 5 lines | 不存在，应该使用 MirAction |
| QueuedAction 错误定义 | 5 lines | action_type 字段类型错误 |
| **总计** | **~16 lines** | **未遵循 C# 依赖关系** |

| 项目 | 添加内容 |
|------|----------|
| 正确的 QueuedAction | 8 lines (使用 MirAction) |
| 注释说明 | 5 lines |
| ItemSetStatus 导入 | 2 lines |
| MirAction 导入 | 1 line |

**净效果**: 删除 ~16 lines 错误代码，添加 ~16 lines 正确代码

---

## 总结

这次错误的根本原因是**未遵循 C# 项目的依赖关系**：

1. **ItemSets** 是 Shared 项目的类型，不应该在 Client 中重复定义
2. **QueuedActionType** 根本不存在，是自己发明的
3. **QueuedAction** 应该使用 SharedRust 的 MirAction，不是自创枚举

**核心原则**:
- ✅ **依赖关系清晰**: Client → Shared，ClientRust → SharedRust
- ✅ **先查后定义**: 先检查 C# 和 SharedRust，再定义
- ✅ **类型来源明确**: 共享类型在 SharedRust，客户端特有在 ClientRust
- ✅ **注释说明**: 明确标注对应的 C# 文件和类

**避免低级错误**:
1. 定义前先搜索 C# 定义位置
2. 检查 SharedRust 是否已有
3. 确认字段类型来源
4. 遵循项目依赖关系
5. 添加注释说明对应关系

---

**状态**: ✅ 已修复并验证通过
