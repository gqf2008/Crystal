# Phase 1 架构修正说明

## 问题发现

用户指出了一个关键架构错误:
> "ItemCell、UserItem 等等结构都是定义在 MirControls 模块(对应 rust 的 controls 模块)里的,为什么会在 game_scene_v2 模块里定义?"

## 根因分析

检查 C# 源代码发现:

### ✅ 实际的 C# 模块组织:

1. **`UserItem`** - **Shared/Data/ItemData.cs line 277**
   - 这是**共享数据结构**(Shared 项目)
   - 包含物品的所有数据字段(UniqueID, ItemIndex, Dura, Count, Slots 等)
   - 被客户端和服务器共用

2. **`MirItemCell`** - **Client/MirControls/MirItemCell.cs line 11**
   - 这是**UI 控件**(继承自 MirImageControl)
   - 负责显示物品图标、处理鼠标交互
   - 内部引用 `UserItem` 数据

3. **`ClientMagic`** - **Shared/Data/ClientData.cs**
   - 共享数据结构

4. **`ClientBuff`** - **Shared/Data/ClientData.cs**
   - 共享数据结构

5. **`ClientQuestInfo`** - **Shared/Data/ClientData.cs**
   - 共享数据结构

### ❌ 错误的初始设计:

在 `game_scene_v2.rs` 中重复定义了这些结构,违反了"单一数据源"原则。

## 修正方案

### ✅ 正确的 Rust 模块映射:

```
C# Shared 项目                → Rust mir2_shared crate
├── UserItem                  → mir2_shared::UserItem ✅
├── ClientMagic               → mir2_shared::data::client_data::ClientMagic ✅
├── ClientBuff                → mir2_shared::data::client_data::ClientBuff ✅
└── ClientQuestInfo           → mir2_shared::data::client_data::ClientQuestInfo ✅

C# Client/MirControls         → Rust controls 模块
├── MirItemCell (UI 控件)     → controls::MirItemCell (TODO)
├── MirLabel                  → controls::MirLabel (TODO)
└── InventoryDialog           → controls::InventoryDialog (TODO)

C# Client/MirObjects          → Rust objects 模块
├── UserObject                → objects::UserObject ✅
├── MonsterObject             → objects::MonsterObject ✅
└── MapObject                 → objects::MapObject ✅

C# Client/MirScenes           → Rust scenes 模块
├── GameScene                 → scenes::GameScene (旧版)
└── GameScene (重构)          → scenes::game_scene_v2::GameScene ✅
```

## 已完成的修正

### 1. 移除重复定义

**删除了以下重复结构**:
- ❌ `ItemCell` - 不需要,直接用 `Option<UserItem>`
- ❌ `UserItem` - 已在 mir2_shared 中定义
- ❌ `ClientMagic` - 已在 mir2_shared 中定义
- ❌ `Buff` - 应使用 mir2_shared::data::client_data::ClientBuff

### 2. 更新导入语句

```rust
use mir2_shared::{
    enums::*,
    packets::server as S,
    Point,
    UserItem,                         // ✅ Shared/Data/ItemData.cs line 277
    data::client_data::{
        ClientMagic,                  // ✅ SharedRust/src/data/client_data.rs line 70
        ClientBuff,                   // ✅ SharedRust/src/data/client_data.rs line 764
        ClientQuestInfo,              // ✅ SharedRust/src/data/client_data.rs line 392
    },
};
```

### 3. 更新数据字段

```rust
pub struct GameScene {
    // 物品系统 - 直接使用 mir2_shared::UserItem
    inventory: [Option<UserItem>; 46],
    storage: [Option<UserItem>; 80],
    
    // 技能系统 - 使用 mir2_shared::data::client_data::ClientMagic
    magics: Vec<ClientMagic>,
    
    // Buff 系统 - 使用 mir2_shared::data::client_data::ClientBuff
    buffs: Vec<ClientBuff>,
    
    // 任务系统 - 使用 mir2_shared::data::client_data::ClientQuestInfo
    quests: Vec<ClientQuestInfo>,
    
    // ... 其他字段
}
```

## 待确认的结构

以下结构**暂时**定义在 `game_scene_v2.rs`,需要后续检查 mir2_shared 是否已有:

### 需要检查 mir2_shared:
- [ ] `Friend` - 好友信息
- [ ] `Relationship` - 关系信息
- [ ] `GuildObject` / `GuildInfo` - 公会信息
- [ ] `Mail` / `ClientMail` - 邮件信息
- [ ] `Rank` / `RankCharacterInfo` - 排行榜信息

### 确认应在客户端定义:
- [x] `QuestTracker` - 任务追踪 UI 状态(可能属于 controls 模块)
- [x] `OutputMessage` - 屏幕输出消息(UI 层)
- [x] `OutputMessageType` - 输出消息类型

## UI 控件层 (Phase 3 工作)

当实现 Phase 3 (UI 控件树)时,需要在 `controls` 模块创建:

```rust
// controls/mir_item_cell.rs (对应 C# MirItemCell)
pub struct MirItemCell {
    // UI 控件属性
    pub location: Point,
    pub size: Size,
    pub visible: bool,
    
    // 数据引用 (不拥有数据)
    pub item: Option<UserItem>,  // 引用 GameScene.inventory 中的数据
    pub locked: bool,
    pub grid_type: MirGridType,
    
    // ... UI 相关字段
}

impl Control for MirItemCell {
    fn draw(&mut self, canvas: &mut Canvas) {
        // 绘制物品图标
    }
    
    fn on_mouse_down(&mut self, button: MouseButton, location: Point) -> bool {
        // 处理点击、拖拽等交互
    }
}
```

## 架构原则总结

### ✅ 单一数据源 (Single Source of Truth)
- 数据定义在 **mir2_shared** (Shared 项目)
- UI 控件定义在 **controls** 模块 (MirControls)
- 游戏对象定义在 **objects** 模块 (MirObjects)
- 场景逻辑定义在 **scenes** 模块 (MirScenes)

### ✅ 数据与表现分离
- **数据层**: mir2_shared (UserItem, ClientMagic, ClientBuff)
- **表现层**: controls (MirItemCell, InventoryDialog)
- **逻辑层**: scenes::GameScene (管理数据,协调 UI)

### ✅ 避免重复定义
- 不要在多个模块重复定义相同结构
- 优先检查 mir2_shared 是否已有定义
- 新结构应放在最合适的模块

## 下一步行动

### Phase 1 继续:
1. [ ] 检查 mir2_shared 中是否有 Friend/Relationship/Guild/Mail/Rank 结构
2. [ ] 如果有,替换临时定义;如果没有,考虑是否应添加到 mir2_shared
3. [ ] 完善其他 GameScene 数据字段

### Phase 2 准备:
1. [ ] MapControl 使用现有的 tile_texture_manager
2. [ ] 实现六层渲染逻辑
3. [ ] 集成现有的地图加载器

### Phase 3 准备:
1. [ ] 设计 controls 模块的完整 Control trait
2. [ ] 实现 MirItemCell (参考 C# MirItemCell.cs)
3. [ ] 实现基础对话框 (MainDialog, InventoryDialog)

## 总结

这次架构修正确保了 Rust 实现与 C# 原版的模块组织保持一致:
- ✅ 数据结构来自 mir2_shared (对应 C# Shared 项目)
- ✅ UI 控件将在 controls 模块实现 (对应 C# MirControls)
- ✅ GameScene 作为中枢,管理数据并协调各模块

这是**正确的分层架构**,为后续 Phase 2/3/4 的实现奠定了坚实基础!
