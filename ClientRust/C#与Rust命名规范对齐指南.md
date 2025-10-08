# C# 与 Rust 命名规范对齐指南

## 命名规范差异

### C# 命名规范
```csharp
public class GameScene {           // PascalCase 类名
    public bool HasHero;            // PascalCase 字段
    public UserObject User;         // PascalCase 字段
    public void ProcessPacket() {}  // PascalCase 方法
    private int moveTime;           // camelCase 私有字段
}
```

### Rust 命名规范
```rust
pub struct GameScene {             // PascalCase 类型名
    pub has_hero: bool,             // snake_case 字段 (强制)
    pub user: Option<UserObject>,   // snake_case 字段 (强制)
}

impl GameScene {
    pub fn process_packet() {}      // snake_case 方法 (强制)
}
```

---

## 对齐策略

### ✅ 类型名 - 严格对齐 C#

| C# 类型名 | Rust 类型名 | 说明 |
|-----------|-------------|------|
| `GameScene` | `GameScene` | ✅ 完全一致 |
| `MapControl` | `MapControl` | ✅ 完全一致 |
| `UserObject` | `UserObject` | ✅ 完全一致 |
| `MonsterObject` | `MonsterObject` | ✅ 完全一致 |
| `HeroSpawnState` | `HeroSpawnState` | ✅ 完全一致 |
| `PanelType` | `PanelType` | ✅ 完全一致 |
| `OutPutMessage` | `OutputMessage` | ⚠️ 修正 C# 拼写错误 |

### ⚠️ 字段名 - 转换为 snake_case + 注释标注

**原则**: Rust 字段必须用 snake_case,但注释中标注 C# 原始字段名

```rust
pub struct GameScene {
    /// 是否拥有英雄 (C#: public bool HasHero)
    pub has_hero: bool,
    
    /// 英雄召唤状态 (C#: public HeroSpawnState HeroSpawnState)
    pub hero_spawn_state: HeroSpawnState,
    
    /// 金币 (C#: public static uint Gold)
    pub gold: u32,
    
    /// 点数 (C#: public static uint Credit)
    pub credit: u32,
    
    /// 新邮件计数器 (C#: public int NewMailCounter)
    pub new_mail_counter: i32,
}
```

**转换表**:

| C# 字段名 | Rust 字段名 | 注释标注 |
|-----------|-------------|----------|
| `HasHero` | `has_hero` | `(C#: HasHero)` |
| `HeroSpawnState` | `hero_spawn_state` | `(C#: HeroSpawnState)` |
| `Gold` | `gold` | `(C#: Gold)` |
| `Credit` | `credit` | `(C#: Credit)` |
| `NewMail` | `new_mail` | `(C#: NewMail)` |
| `NewMailCounter` | `new_mail_counter` | `(C#: NewMailCounter)` |
| `PickedUpGold` | `picked_up_gold` | `(C#: PickedUpGold)` |
| `SelectedCell` | `selected_cell` | `(C#: SelectedCell)` |
| `HoverItem` | `hover_item` | `(C#: HoverItem)` |
| `SelectedItem` | `selected_item` | `(C#: SelectedItem)` |
| `MoveTime` | `move_time` | `(C#: MoveTime)` |
| `AttackTime` | `attack_time` | `(C#: AttackTime)` |
| `SpellTime` | `spell_time` | `(C#: SpellTime)` |

### ⚠️ 方法名 - 转换为 snake_case + 注释标注

```rust
impl GameScene {
    /// 构造函数 (C#: public GameScene())
    pub fn new() -> Self { }
    
    /// 处理网络包 (C#: protected override void ProcessPacket(Packet p))
    pub fn process_packet(&mut self, packet: ServerPacket) { }
    
    /// 使用技能 (C#: public void UseSpell(int key))
    pub fn use_spell(&mut self, key: i32) { }
    
    /// 输出消息 (C#: public void AddMessage(string message, OutputMessageType type))
    pub fn output_message(&mut self, message: String, msg_type: OutputMessageType) { }
}
```

**转换表**:

| C# 方法名 | Rust 方法名 | 注释标注 |
|-----------|-------------|----------|
| `ProcessPacket()` | `process_packet()` | `(C#: ProcessPacket)` |
| `UseSpell()` | `use_spell()` | `(C#: UseSpell)` |
| `DrawControl()` | `draw_control()` | `(C#: DrawControl)` |
| `OnKeyDown()` | `on_key_down()` | `(C#: OnKeyDown)` |
| `OnMouseMove()` | `on_mouse_move()` | `(C#: OnMouseMove)` |

---

## 为什么不能完全一致?

### Rust 编译器强制 snake_case

```rust
// ❌ 编译警告:non-snake-case field names
pub struct GameScene {
    pub HasHero: bool,        // warning: field `HasHero` should have a snake case name
    pub HeroSpawnState: bool, // warning: field `HeroSpawnState` should have a snake case name
}

// ✅ 正确:使用 snake_case
pub struct GameScene {
    pub has_hero: bool,
    pub hero_spawn_state: HeroSpawnState,
}
```

### Rust 社区惯例

Rust 标准库和生态系统统一使用 snake_case:

```rust
// 标准库示例
pub struct HashMap<K, V> {
    pub hash_builder: RandomState,  // snake_case
    pub len: usize,                 // snake_case
}

impl HashMap<K, V> {
    pub fn insert(&mut self, k: K, v: V) {}  // snake_case
    pub fn get(&self, k: &K) -> Option<&V> {}  // snake_case
}
```

---

## 模块组织对齐

### C# 命名空间结构
```csharp
namespace Client.MirScenes {
    public sealed class GameScene : MirScene {
        public sealed class MapControl : MirControl { }
    }
}
```

### Rust 模块结构(对齐)
```rust
// src/scenes/mod.rs
pub mod game_scene;

// src/scenes/game_scene.rs
pub struct GameScene { }

// src/scenes/game_scene/mod.rs
pub mod map_control;

// src/scenes/game_scene/map_control.rs
pub struct MapControl { }
```

**对应关系**:
- C# `Client.MirScenes` → Rust `crate::scenes`
- C# `GameScene` → Rust `game_scene.rs`
- C# 嵌套类 `MapControl` → Rust 子模块 `game_scene::MapControl`

---

## 特殊情况处理

### 1. C# 拼写错误修正

```rust
// C# 原名: OutPutMessage (拼写错误)
// Rust 修正: OutputMessage (正确拼写)
pub struct OutputMessage {
    // 注释中标注: (C#: OutPutMessage - 注意拼写)
}
```

### 2. C# 缩写保持

```rust
// C# 原名: NPCDialog, NPCTime, NPCID
// Rust 保持: NpcDialog, npc_time, npc_id (首字母小写其余保持)
pub struct NpcDialog { }
pub npc_time: i64,
pub npc_id: u32,
```

### 3. 静态字段 vs 实例字段

**C# 使用静态字段**:
```csharp
public static UserObject User;
public static uint Gold;
public static long MoveTime;
```

**Rust 使用实例字段**:
```rust
pub struct GameScene {
    pub user: Option<UserObject>,  // C#: static User
    pub gold: u32,                  // C#: static Gold
    pub move_time: i64,             // C#: static MoveTime
}
```

**原因**: Rust 不鼓励全局静态可变状态,使用结构体字段更安全。

---

## 对齐检查清单

### ✅ 完全对齐项

- [x] 类型名使用 PascalCase (GameScene, MapControl, UserObject)
- [x] 枚举名使用 PascalCase (HeroSpawnState, PanelType)
- [x] 枚举值使用 PascalCase (None, Spawning, Spawned)
- [x] 模块路径对应命名空间 (scenes → MirScenes)

### ⚠️ 转换项(不违反对齐)

- [x] 字段名转换为 snake_case (HasHero → has_hero)
- [x] 方法名转换为 snake_case (ProcessPacket → process_packet)
- [x] 注释标注 C# 原始名称
- [x] 静态字段改为实例字段(注释说明)

### ❌ 不对齐项(需避免)

- [ ] ❌ 类型名使用 snake_case (game_scene ✗)
- [ ] ❌ 字段名使用 PascalCase (HasHero ✗,会有编译警告)
- [ ] ❌ 方法名使用 PascalCase (ProcessPacket ✗)
- [ ] ❌ 模块路径不对应 (random_module ✗)

---

## 总结

### 对齐原则优先级

1. **类型名**: 100% 对齐 C# PascalCase ✅
2. **模块组织**: 100% 对应 C# 命名空间 ✅
3. **字段/方法名**: 转换为 snake_case + 注释标注 C# 原名 ⚠️
4. **逻辑结构**: 100% 镜像 C# 架构 ✅

### 核心理念

**不是"违反对齐",而是"遵循两种语言各自的惯例,通过注释建立对应关系"**。

这样做的好处:
- ✅ Rust 代码符合社区规范,易于维护
- ✅ 通过注释清晰标注 C# 对应关系
- ✅ 类型名和架构完全对齐,易于理解
- ✅ 避免编译器警告

**结论**: 当前的命名方式**已经是最佳的对齐策略**,既遵循 Rust 规范,又保持了与 C# 的对应关系。
