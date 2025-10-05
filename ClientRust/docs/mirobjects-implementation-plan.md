# MirObjects 模块实施计划

## 当前状态分析 (2025-10-05)

### ✅ 已完成的文件
- `map_object.rs` - 基础MapObject (部分实现,需对照C#补充)
- `player_object.rs` - PlayerObject (部分实现,需对照C#补充)  
- `user_object.rs` - UserObject (骨架存在)
- `monster_object.rs` - MonsterObject (骨架存在)
- `npc_object.rs` - NPCObject (骨架存在)
- `item_object.rs` - ItemObject (骨架存在)
- `hero_object.rs` - HeroObject (骨架存在)
- `spell_object.rs` - SpellObject (骨架存在)
- `effect.rs` - Effect (骨架存在)
- `damage.rs` - Damage (骨架存在)
- `frames.rs` - Frame/FrameSet (骨架存在)
- `pathfinder.rs` - PathFinder (骨架存在)
- `map_code.rs` - MapReader (骨架存在)

### 🎯 Phase 1: 完善核心基类 (MapObject + PlayerObject)

**目标:** 严格对照C#原版,补充缺失的字段和方法

#### Task 1.1: MapObject 完善 (3-5小时)
**C# 参考:** `Client/MirObjects/MapObject.cs` (600行)

**需要补充的关键内容:**

1. **静态字段 (C# lines 12-49):**
```csharp
public static UserObject User;
public static UserHeroObject Hero;
public static HeroObject HeroObject;
public static MapObject MouseObject, TargetObject, MagicObject;
public static uint MouseObjectID, TargetObjectID, MagicObjectID;
```
→ Rust实现: 使用全局状态管理器或传递上下文

2. **核心字段 (C# lines 54-93):**
```csharp
public uint ObjectID;
public string Name = string.Empty;
public Point CurrentLocation, MapLocation;
public MirDirection Direction;
public bool Dead, Hidden, SitDown, Sneaking;
public PoisonType Poison;
public long DeadTime;
public byte AI;
public bool InTrapRock;
public int JumpDistance;
public bool Blend = true;
public long BlindTime;
public byte BlindCount;
public byte PercentHealth, PercentMana;
public long HealthTime;
```
→ Rust: 对照补充缺失字段

3. **Action系统 (C# lines 97-102):**
```csharp
public List<QueuedAction> ActionFeed = new List<QueuedAction>();
public QueuedAction NextAction { get; }
```
→ Rust: `Vec<QueuedAction>`

4. **Effect和Buff (C# lines 104-106):**
```csharp
public List<Effect> Effects = new List<Effect>();
public List<BuffType> Buffs = new List<BuffType>();
```

5. **渲染相关 (C# lines 108-121):**
```csharp
public MLibrary BodyLibrary;
public Color DrawColour, NameColour, LightColour;
public MirLabel NameLabel, ChatLabel, GuildLabel;
public long ChatTime;
public int DrawFrame, DrawWingFrame;
public Point DrawLocation, Movement, FinalDrawLocation, OffSetMove;
public Rectangle DisplayRectangle;
public int Light, DrawY;
public long NextMotion, NextMotion2;
public MirAction CurrentAction;
public byte CurrentActionLevel;
public bool SkipFrames;
public FrameLoop FrameLoop = null;
```

6. **核心方法:**
- `Remove()` (C# line 153-176)
- `Process()` - abstract
- `Draw()` - abstract  
- `MouseOver()` - abstract
- `AddBuffEffect()` (C# line 213-352)
- `RemoveBuffEffect()` (C# line 353-445)

#### Task 1.2: PlayerObject 完善 (5-7小时)
**C# 参考:** `Client/MirObjects/PlayerObject.cs` (5286行! 超大类)

**关键内容:**

1. **基础字段 (C# lines 12-36):**
```csharp
public MirGender Gender;
public MirClass Class;
public byte Hair;
public ushort Level;
public MLibrary WeaponLibrary1, WeaponEffectLibrary1, WeaponLibrary2, HairLibrary, WingLibrary, MountLibrary;
public int Armour, Weapon, WeaponEffect, ArmourOffSet, HairOffSet, WeaponOffSet, WingOffset, MountOffset;
public int DieSound, FlinchSound, AttackSound;
public FrameSet Frames;
public Frame Frame, WingFrame;
public int FrameIndex, FrameInterval, EffectFrameIndex, EffectFrameInterval, SlowFrameIndex;
public byte SkipFrameUpdate = 0;
```

2. **魔法系统 (C# lines 57-64):**
```csharp
public Spell Spell;
public byte SpellLevel;
public bool Cast;
public uint TargetID;
public List<uint> SecondaryTargetIDs;
public Point TargetPoint;
public bool MagicShield;
public Effect ShieldEffect;
```

3. **Buff系统 (C# lines 66-79):**
```csharp
public bool ElementalBarrier;
public Effect ElementalBarrierEffect;
public byte WingEffect;
public bool ElementalBuff;
public bool Concentrating;
public InterruptionEffect ConcentratingEffect;
public bool ConcentrateInterrupted;
public bool HasElements;
public bool ElementCasted;
public int ElementEffect;
public int ElementsLevel;
public int ElementOrbMax;
public SpellEffect CurrentEffect;
```

4. **状态字段 (C# lines 81-89):**
```csharp
public bool RidingMount, Sprint, FastRun, Fishing, FoundFish;
public long StanceTime, MountTime, FishingTime;
public long BlizzardStopTime, ReincarnationStopTime, SlashingBurstTime;
public short MountType = -1, TransformType = -1;
public string GuildName;
public string GuildRankName;
public Point FishingPoint;
public LevelEffects LevelEffects;
```

5. **核心方法:**
- `Load(S.ObjectPlayer info)` (C# line 98-168) - ⭐ 关键方法
- `Update(S.PlayerUpdate info)` (C# line 169-178)
- `SetLibraries()` (C# line 193-265) - 加载纹理库
- `SetAction()` (C# line 267-395) - 设置动作
- `DrawName()` (C# line 397-498)
- `Draw()` (C# line 500-1240) - ⭐ 超大方法,渲染逻辑
- `DrawBody()`, `DrawWeapon()`, `DrawHair()` 等子方法
- `Process()` (C# line 2500-2800) - 更新逻辑
- `ProcessFrames()` (C# line 2802-3100)
- `NextMotionFrame()` (C# line 3102-3400)

### 🎯 Phase 2: UserObject 实现 (重中之重!)

**C# 参考:** `Client/MirObjects/UserObject.cs` (3000+行)

**UserObject是玩家自己,最复杂的对象类:**

1. **Input处理:**
- `ProcessInput()` - 键盘鼠标输入
- `Move()`, `Attack()`, `UseSpell()` 等

2. **Inventory系统:**
- `Inventory` - 背包
- `Equipment` - 装备栏
- `QuestInventory` - 任务物品

3. **Magic系统:**
- `Magics` - 技能列表
- `NextMagicObject` - 施法目标
- `ClearMagic()`, `BeginMagic()` 等

4. **状态管理:**
- `HP`, `MP`, `MaxHP`, `MaxMP`
- `Gold` - 金币
- `Poison` - 中毒
- `Dead` - 死亡状态

### 🎯 Phase 3: MonsterObject + NPCObject

**MonsterObject (C# ~2000行):**
- AI行为
- 动画播放
- 血条显示

**NPCObject (C# ~500行):**
- 静态显示
- 交互逻辑

### 🎯 Phase 4: 辅助类

- **Effect.cs** - 特效系统
- **Damage.cs** - 伤害显示
- **Frames.cs** - 帧动画

---

## 实施策略

### ⚠️ 关键原则
1. **严格对照C#字段名** - 除了Rust命名规范(snake_case),字段含义完全一致
2. **禁止过度抽象** - 不创造C#没有的trait/接口
3. **保持类层次** - MapObject → PlayerObject → UserObject
4. **方法签名一致** - 参数类型、返回值尽量对应
5. **注释对应行号** - 每个方法标注C#源文件行号

### 📝 实施顺序 (推荐)

**Week 1: 核心基类**
- Day 1-2: MapObject 补充 (字段+核心方法)
- Day 3-5: PlayerObject 补充 (字段+Load/Draw方法)

**Week 2: UserObject**
- Day 1-3: UserObject 基础 (字段+构造)
- Day 4-5: Input系统 (移动+攻击)

**Week 3: 渲染集成**
- Day 1-2: 集成DXManager (Draw方法)
- Day 3-4: 动画系统 (Process/NextFrame)
- Day 5: 测试+调试

**Week 4: Monster + NPC**
- Day 1-3: MonsterObject
- Day 4: NPCObject
- Day 5: 集成测试

---

## 当前优先级

**🔥 立即开始:**
1. MapObject 字段补充
2. PlayerObject Load() 方法实现
3. 测试渲染一个玩家对象

**📋 后续任务:**
- UserObject 完整实现
- Monster/NPC 渲染
- 地图集成

---

## 预期成果

**完成后将能够:**
✅ 加载并显示玩家角色 (Load from ObjectPlayer packet)
✅ 播放行走/攻击动画
✅ 渲染装备/武器/翅膀
✅ 显示名字/血条/Buff图标
✅ 加载并显示怪物
✅ 加载并显示NPC

**为后续打下基础:**
→ 地图系统 (显示对象在地图上)
→ 战斗系统 (攻击判定)
→ UI系统 (角色面板)

