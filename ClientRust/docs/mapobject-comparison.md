# MapObject C# vs Rust 字段对照表

## C# MapObject.cs (lines 11-141)

### 静态字段
```csharp
public static Font ChatFont = new Font(Settings.FontName, 10F);
public static List<MirLabel> LabelList = new List<MirLabel>();

public static UserObject User;
public static UserHeroObject Hero;
public static HeroObject HeroObject;
public static MapObject MouseObject, TargetObject, MagicObject;

private static uint mouseObjectID;
public static uint MouseObjectID { get; set; } // with logic
private static uint lastTargetObjectId;
private static uint targetObjectID;
public static uint TargetObjectID { get; set; } // with logic
private static uint magicObjectID;
public static uint MagicObjectID { get; set; } // with logic
```

### 抽象属性
```csharp
public abstract ObjectType Race { get; }
public abstract bool Blocking { get; }
```

### 实例字段 (核心)
```csharp
// === Identity ===
public uint ObjectID;
public string Name = string.Empty;

// === Position ===
public Point CurrentLocation;  // 当前位置
public Point MapLocation;      // 地图格子位置

// === Direction ===
public MirDirection Direction;

// === State Flags ===
public bool Dead;
public bool Hidden;
public bool SitDown;           // ❌ Rust missing
public bool Sneaking;          // ❌ Rust missing
public PoisonType Poison;
public long DeadTime;          // ❌ Rust missing
public byte AI;
public bool InTrapRock;        // ❌ Rust missing
public int JumpDistance;       // ❌ Rust missing

// === Visual ===
public bool Blend = true;

// === Health/Mana ===
public long BlindTime;         // ❌ Rust missing
public byte BlindCount;        // ❌ Rust missing
private byte percentHealth;
public virtual byte PercentHealth { get; set; }  // ❌ Rust missing
public long HealthTime;        // ❌ Rust missing
private byte percentMana;
public virtual byte PercentMana { get; set; }    // ❌ Rust missing

// === Target ===
public uint LastTargetObjectId => lastTargetObjectId;  // ❌ Rust missing

// === Action System ===
public List<QueuedAction> ActionFeed = new List<QueuedAction>();  // ❌ Rust missing
public QueuedAction NextAction { get; }                           // ❌ Rust missing

// === Effects & Buffs ===
public List<Effect> Effects = new List<Effect>();  // ❌ Rust missing
public List<BuffType> Buffs = new List<BuffType>(); // ⚠️ Rust has BuffState abstraction

// === Graphics Assets ===
public MLibrary BodyLibrary;   // ❌ Rust missing

// === Colors ===
public Color DrawColour = Color.White;   // ❌ Rust missing
public Color NameColour = Color.White;   // ⚠️ Rust has name_colour as i32
public Color LightColour = Color.White;  // ❌ Rust missing

// === Labels (UI) ===
public MirLabel NameLabel;     // ❌ Rust missing
public MirLabel ChatLabel;     // ❌ Rust missing
public MirLabel GuildLabel;    // ❌ Rust missing
public long ChatTime;          // ❌ Rust missing

// === Drawing ===
public int DrawFrame;          // ❌ Rust missing
public int DrawWingFrame;      // ❌ Rust missing
public Point DrawLocation;     // ❌ Rust missing
public Point Movement;         // ❌ Rust missing
public Point FinalDrawLocation; // ❌ Rust missing
public Point OffSetMove;       // ❌ Rust missing
public Rectangle DisplayRectangle; // ❌ Rust missing
public int Light;              // ⚠️ Rust has light as u8
public int DrawY;              // ❌ Rust missing

// === Animation Timing ===
public long NextMotion;        // ❌ Rust missing
public long NextMotion2;       // ❌ Rust missing
public MirAction CurrentAction; // ⚠️ Rust has in AnimationState
public byte CurrentActionLevel; // ❌ Rust missing
public bool SkipFrames;        // ❌ Rust missing
public FrameLoop FrameLoop = null; // ❌ Rust missing

// === Sound ===
public int StruckWeapon;       // ❌ Rust missing

// === Damage Display ===
public MirLabel TempLabel;     // ❌ Rust missing
public static List<MirLabel> DamageLabelList = new List<MirLabel>();  // ❌ Rust missing
public List<Damage> Damages = new List<Damage>(); // ❌ Rust missing
```

## Rust map_object.rs 当前实现

```rust
pub struct MapObject {
    // === Identity ===
    object_id: u32,              // ✅ = ObjectID
    object_type: MapObjectType,  // ⚠️ Extra field (internal)
    
    // === Position and Direction ===
    location: Point,             // ⚠️ = CurrentLocation? or MapLocation?
    direction: MirDirection,     // ✅ = Direction
    
    // === Display Information ===
    name: String,                // ✅ = Name
    name_colour: i32,            // ✅ = NameColour (as ARGB)
    
    // === State Flags ===
    dead: bool,                  // ✅ = Dead
    hidden: bool,                // ✅ = Hidden
    poison: PoisonType,          // ✅ = Poison
    
    // === Monster/NPC specific ===
    ai: u8,                      // ✅ = AI
    light: u8,                   // ✅ = Light
    
    // === Private State ===
    buffs: BuffState,            // ⚠️ Abstraction! C# has List<BuffType>
    animation: AnimationState,   // ⚠️ Abstraction! C# has separate fields
    last_update: Instant,        // ⚠️ Extra field (internal)
}
```

## 缺失字段总结 (按优先级)

### 🔥 P0 - 核心字段 (必须立即补充)
1. `current_location: Point` - 当前位置
2. `map_location: Point` - 地图位置
3. `action_feed: Vec<QueuedAction>` - 动作队列 (C#核心!)
4. `effects: Vec<Effect>` - 特效列表
5. `buffs: Vec<BuffType>` - Buff列表 (替换BuffState)
6. `dead_time: i64` - 死亡时间
7. `sit_down: bool` - 坐下状态
8. `sneaking: bool` - 潜行状态

### ⚠️ P1 - 渲染字段 (Draw方法需要)
9. `draw_location: Point` - 绘制位置
10. `movement: Point` - 移动偏移
11. `final_draw_location: Point` - 最终绘制位置
12. `draw_frame: i32` - 当前绘制帧
13. `draw_wing_frame: i32` - 翅膀帧
14. `current_action: MirAction` - 当前动作
15. `next_motion: i64` - 下一帧时间
16. `skip_frames: bool` - 跳帧标志

### 📋 P2 - UI/Display字段
17. `percent_health: u8` - 血量百分比
18. `percent_mana: u8` - 魔法百分比
19. `health_time: i64` - 血条显示时间
20. `chat_time: i64` - 聊天显示时间
21. `blind_time: i64` - 致盲时间
22. `blind_count: u8` - 致盲层数

### 📦 P3 - 辅助字段
23. `in_trap_rock: bool`
24. `jump_distance: i32`
25. `struck_weapon: i32`
26. `damages: Vec<Damage>`

## 建议重构方案

### 选项A: 最小改动 (推荐)
保留现有结构,补充缺失字段:
```rust
pub struct MapObject {
    // === Identity ===
    pub object_id: u32,
    pub name: String,
    pub name_colour: i32,
    
    // === Position (C# has 2 separate Points!) ===
    pub current_location: Point,  // NEW
    pub map_location: Point,       // NEW
    pub direction: MirDirection,
    
    // === State Flags ===
    pub dead: bool,
    pub hidden: bool,
    pub sit_down: bool,            // NEW
    pub sneaking: bool,            // NEW
    pub poison: PoisonType,
    pub dead_time: i64,            // NEW
    
    // === Action System (C# 核心!) ===
    pub action_feed: Vec<QueuedAction>,  // NEW
    pub current_action: MirAction,       // NEW
    pub next_motion: i64,                // NEW
    pub skip_frames: bool,               // NEW
    
    // === Effects & Buffs ===
    pub effects: Vec<Effect>,      // NEW
    pub buffs: Vec<BuffType>,      // NEW (replace BuffState)
    
    // === Drawing ===
    pub draw_location: Point,      // NEW
    pub draw_frame: i32,           // NEW
    pub movement: Point,           // NEW
    
    // === Health/Mana ===
    pub percent_health: u8,        // NEW
    pub percent_mana: u8,          // NEW
    pub health_time: i64,          // NEW
    
    // ... other fields
}
```

### 选项B: 完全重写
删除所有抽象(BuffState, AnimationState),严格对照C#一对一实现。

---

## 推荐行动

**立即执行:**
1. 补充P0字段 (action_feed, current_location等)
2. 删除或重构BuffState/AnimationState抽象
3. 实现核心方法: Remove(), AddBuffEffect(), RemoveBuffEffect()

**时间估算:**
- 字段补充: 2小时
- 方法实现: 3-4小时
- 测试验证: 1小时
**总计: 6-7小时**

