# ECS架构: Resources vs Components 划分指南

## 目录
- [核心原则](#核心原则)
- [Resources (资源)](#resources-资源)
- [Components (组件)](#components-组件)
- [实际示例](#实际示例)
- [决策流程图](#决策流程图)
- [常见误区](#常见误区)

---

## 核心原则

### Resources: 全局、共享、只读数据
**定义**: 游戏全局共享的数据,通常在初始化时加载,运行时**只读**或**极少修改**

**特征**:
- ✅ **单例性质**: 整个游戏只有一份
- ✅ **生命周期长**: 从加载到游戏结束
- ✅ **跨场景共享**: 所有场景都能访问
- ✅ **只读为主**: 修改频率极低(配置、静态数据)

**适用场景**:
- 游戏资产库 (图像、音效、地图文件)
- 配置文件 (settings.json)
- 静态数据表 (道具模板、怪物属性表)
- 全局服务 (网络连接、输入管理器)

---

### Components: 实体级、可变、运行时数据
**定义**: 附加到ECS实体上的数据,表示该实体的**属性**或**状态**

**特征**:
- ✅ **多实例**: 可以有N个玩家、N个怪物
- ✅ **可变性**: 每帧都可能改变(位置、血量)
- ✅ **实体绑定**: 只对特定实体有效
- ✅ **系统处理**: 由Systems查询和修改

**适用场景**:
- 实体位置 (Position, Transform)
- 实体状态 (Health, Mana, Stamina)
- 行为标记 (LocalPlayer, RemotePlayer, Monster)
- 临时数据 (MovementTarget, AttackAnimation)

---

## 实际示例

### 当前项目中的Resources

#### 1. 图像库系统 (GraphicsLibraries)
**位置**: `src/graphics/libraries.rs`

**用途**: 存储所有游戏图像资源 (怪物、地图、UI)

**为什么是Resource?**
- ✅ 全局单例 (通过 `lazy_static!` 实现)
- ✅ 只读访问 (加载后不修改)
- ✅ 所有场景共享 (LoginScene、GameScene都需要绘制图像)

```rust
// 使用示例
if let Some(lib_arc) = get_library(LibraryName::Prguse) {
    if let Ok(mut lib) = lib_arc.try_lock() {
        lib.draw(ctx, canvas, 65, 0.0, 0.0, false)?;
    }
}
```

#### 2. 网络连接 (NetContext)
**位置**: `src/network/mod.rs`

**用途**: 管理TCP连接、发送/接收数据包

**为什么是Resource?**
- ✅ 全局单例 (整个游戏只有一个网络连接)
- ✅ 跨场景使用 (LoginScene发送登录、GameScene发送移动)
- ✅ 服务性质 (提供send/recv接口)

```rust
// 使用示例 (在Scene中)
net_ctx.send(GameEvent::LoginRequest { ... })?;
```

#### 3. 客户端配置 (ClientSettings)
**位置**: `src/settings.rs`

**用途**: 存储用户配置 (分辨率、音量、按键绑定)

**为什么是Resource?**
- ✅ 全局单例 (一个游戏实例只有一套配置)
- ✅ 极少修改 (只在设置界面修改)
- ✅ 持久化存储 (需要保存到文件)

---

### 当前项目中的Components

#### 1. 玩家数据 (PlayerData)
**位置**: `src/ecs/components/player.rs`

**用途**: 存储玩家的基础属性

```rust
pub struct PlayerData {
    pub id: u32,
    pub name: String,
    pub class: MirClass,
    pub gender: MirGender,
    pub level: u16,
    pub exp: i64,
    pub max_experience: i64,
    pub gold: u32,
    pub credit: u32,
}
```

**为什么是Component?**
- ✅ 多实例 (本地玩家 + 其他玩家)
- ✅ 频繁修改 (经验、金币每时每刻都在变)
- ✅ 实体特定 (每个玩家的数据不同)

#### 2. 位置组件 (Position)
**位置**: `src/ecs/components/position.rs`

**用途**: 存储实体的坐标

```rust
pub struct Position {
    pub x: i32,
    pub y: i32,
}
```

**为什么是Component?**
- ✅ 多实例 (每个玩家、怪物、NPC都有位置)
- ✅ 高频修改 (移动时每帧更新)
- ✅ 系统查询 (RenderSystem需要查询所有Position)

#### 3. 角色选择数据 (CharacterSelectData)
**位置**: `src/ecs/components/character_select.rs`

**用途**: 存储登录后服务器返回的角色列表

```rust
pub struct CharacterSelectData {
    pub character: CharacterSummary,
}
```

**为什么是Component?**
- ✅ 多实例 (一个账号可以有多个角色)
- ✅ 临时数据 (只在SelectScene使用)
- ✅ 场景切换时清理 (进入游戏后删除这些实体)

#### 4. 标记组件 (LocalPlayer, RemotePlayer)
**位置**: `src/ecs/components/player.rs`

**用途**: 区分本地玩家和远程玩家

```rust
pub struct LocalPlayer;  // 空结构体,仅用作标记

pub struct RemotePlayer {
    pub id: u32,
}
```

**为什么是Component?**
- ✅ 实体标记 (通过查询 `query::<&LocalPlayer>()` 快速找到本地玩家)
- ✅ 行为区分 (本地玩家可以控制,远程玩家只能观察)

---

## 决策流程图

```
开始
  |
  V
这个数据是否只有一份? ----是----> 使用 Resource
  |
  否
  |
  V
这个数据是否附加到实体上? ----否----> 使用 Resource
  |
  是
  |
  V
这个数据是否会频繁修改? ----是----> 使用 Component
  |
  否
  |
  V
这个数据是否在运行时动态创建/删除? ----是----> 使用 Component
  |
  否
  |
  V
考虑使用 Resource (如果是全局配置)
或 Component (如果是实体属性)
```

---

## 典型案例分析

### ✅ 正确: 怪物属性表 → Resource

**场景**: 游戏有100种怪物类型,每种怪物有固定的血量、攻击力、经验值

**设计**:
```rust
// Resource: 怪物模板库 (只读)
pub struct MonsterTemplates {
    templates: HashMap<u32, MonsterTemplate>,
}

pub struct MonsterTemplate {
    id: u32,
    name: String,
    base_hp: i32,
    base_attack: i32,
    exp_reward: i32,
}

// Component: 怪物实例 (可变)
pub struct MonsterData {
    template_id: u32,      // 引用模板ID
    current_hp: i32,       // 当前血量(可变)
    position: (i32, i32),  // 当前位置(可变)
}
```

**解释**:
- MonsterTemplate 是静态数据,从配置文件加载,只读 → Resource
- MonsterData 是运行时实例,每个怪物一份,频繁修改 → Component

---

### ✅ 正确: 地图文件 → Resource + Component混合

**场景**: 游戏有100张地图,每张地图的静态数据(瓦片)很大

**设计**:
```rust
// Resource: 地图文件库 (只读,按需加载)
pub struct MapFileLibrary {
    loaded_maps: HashMap<String, MapFile>,
}

// Component: 当前加载的地图实例
pub struct MapData {
    map_index: i32,
    width: i32,
    height: i32,
    // ... 当前地图的动态数据
}
```

**解释**:
- MapFileLibrary 缓存地图文件,避免重复加载 → Resource
- MapData 表示"当前玩家所在的地图",场景切换时修改 → Component

---

### ❌ 错误: 把所有配置都放Component

**错误示例**:
```rust
// ❌ 错误: 把全局配置附加到某个实体上
pub struct GlobalSettings {
    volume: f32,
    resolution: (u32, u32),
}

// 在某个实体上挂载配置组件
world.spawn((GlobalSettings { ... },));
```

**问题**:
1. 配置应该是全局的,不应该附加到实体
2. 查询时需要 `query::<&GlobalSettings>()`,浪费性能
3. 如果实体被删除,配置就丢失了

**正确做法**: 使用 Resource (或静态变量)

---

### ❌ 错误: 把实体属性放Resource

**错误示例**:
```rust
// ❌ 错误: 把玩家数据放全局变量
pub static mut PLAYER_DATA: Option<PlayerData> = None;
```

**问题**:
1. 无法支持多玩家 (无法存储其他玩家数据)
2. 无法利用ECS查询优化
3. 线程不安全 (`static mut` 需要 unsafe)

**正确做法**: 使用 Component,每个玩家一个实体

---

## 常见误区

### 误区1: "配置文件就一定是Resource"
**反例**: 玩家的按键绑定

- 如果每个玩家可以有自己的按键绑定 → Component (KeyBindings)
- 如果全局只有一套按键绑定 → Resource (GlobalKeyBindings)

### 误区2: "可变数据就一定是Component"
**反例**: 网络统计数据

```rust
// Resource: 网络统计 (全局,可变,但不是实体属性)
pub struct NetworkStats {
    total_packets_sent: AtomicU64,
    total_bytes_received: AtomicU64,
}
```

虽然这个数据会修改,但它是**全局统计**,不属于某个实体 → Resource

### 误区3: "Resource性能更好,能用Resource就用Resource"
**错误**: Resource主要考虑**语义**,而非性能

- ECS的Component查询已经高度优化 (缓存友好)
- 强行用Resource会破坏ECS架构优势

---

## 总结表格

| 维度 | Resources | Components |
|------|-----------|-----------|
| **数量** | 单例 (全局唯一) | 多实例 (每个实体一份) |
| **生命周期** | 长期 (游戏全程) | 动态 (随实体创建/销毁) |
| **修改频率** | 低 (配置、静态数据) | 高 (位置、状态) |
| **访问方式** | 全局访问 | 通过查询访问 |
| **典型用途** | 资产、配置、服务 | 实体属性、状态 |
| **示例** | 图像库、网络连接 | 位置、血量、装备 |

---

## 实践建议

### 1. 优先使用Component
遇到新数据时,默认考虑Component,除非明确需要全局共享

### 2. 避免过度抽象
不要为了"优雅"强行把所有数据放Resource

### 3. 根据场景调整
- **单机游戏**: Resource可以多一些 (全局配置)
- **多人游戏**: Component为主 (每个玩家不同)

### 4. 文档化你的选择
在代码中注释为什么选择Resource或Component

```rust
/// 怪物模板库 (Resource)
/// 
/// 为什么是Resource?
/// - 只读静态数据,从配置文件加载
/// - 所有怪物共享同一份模板
/// - 不随实体创建/销毁而变化
pub struct MonsterTemplates { ... }
```

---

## 参考资料

### 本项目相关文件
- `src/graphics/libraries.rs` - 图像库Resource示例
- `src/ecs/components/player.rs` - 玩家Component示例
- `src/network/mod.rs` - 网络服务Resource示例

### 外部资源
- [ECS FAQ - Bevy Engine](https://bevyengine.org/learn/book/next/getting-started/ecs/)
- [hecs Documentation](https://docs.rs/hecs/latest/hecs/)
- [Data-Oriented Design Book](https://www.dataorienteddesign.com/dodbook/)

---

**更新日期**: 2025-10-31  
**维护者**: Crystal Mir2 Team
