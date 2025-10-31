# GameScene 初始化架构文档

> 文档创建时间: 2025-10-31  
> 用途: GameScene 进入游戏场景的完整初始化流程和 ECS 系统启动说明

---

## 📋 目录

1. [概述](#概述)
2. [初始化阶段划分](#初始化阶段划分)
3. [实体创建清单](#实体创建清单)
4. [ECS系统启动](#ecs系统启动)
5. [依赖关系分析](#依赖关系分析)
6. [当前问题](#当前问题)
7. [实现方案](#实现方案)

---

## 概述

GameScene 是传奇游戏的核心场景，负责游戏世界的渲染和逻辑更新。进入游戏场景需要完成以下工作：

1. **基础设施初始化** - 相机、UI、渲染配置等
2. **游戏世界构建** - 玩家实体、地图数据、事件总线
3. **ECS 系统启动** - 6 层并行调度器的所有系统

---

## 初始化阶段划分

### 阶段 1: 基础设施初始化
**时机**: `GameScene::initialize(ctx, world)` 首次调用  
**触发**: 进入 GameScene 后首次 `update()`

```rust
// 位置: src/ecs/scenes/game_scene.rs::initialize()
fn initialize(&mut self, ctx: &mut Context, world: &mut World) -> GameResult {
    if self.is_initialized() {
        return Ok(());
    }
    
    // 1. 初始化图形库
    initialize_all_libraries("Data")?;
    
    // 2. 创建基础实体（见下文清单）
    // ...
    
    // 3. 系统调度器已在构造函数中创建
    // self.system_scheduler = UpdateRenderParallelScheduler::new()
    
    Ok(())
}
```

**状态**: ✅ 已完成

### 阶段 2: 游戏世界构建
**时机**: 收到服务器 `UserInformation` 事件时  
**触发**: `NetworkEventSystem::process_event(world, &UserInformation {...})`

```rust
// 位置: src/ecs/systems/network_event_system.rs
GameEvent::UserInformation { location_x, location_y, hp, mp, gold } => {
    // 1. 创建 GlobalEvents 事件总线
    // 2. 创建 LocalPlayer 实体（完整组件）
    // 3. 加载地图数据
    // 4. 创建地图瓦片
    // 5. 初始化相机跟随
}
```

**状态**: ❌ 缺失 - 需要实现

### 阶段 3: ECS 系统运行
**时机**: 每帧 `GameScene::update()` 自动调用  
**调度器**: `UpdateRenderParallelScheduler`

**状态**: ✅ 系统已创建，等待实体数据

---

## 实体创建清单

### ✅ 基础设施实体（阶段 1 - 已完成）

| 实体 | 组件 | 用途 | 创建位置 |
|------|------|------|----------|
| Camera | Position, Camera, Draggable | 视角控制、屏幕坐标转换 | `self.camera_entity` |
| TimeTracker | TimeTracker | 帧率统计、动画计数 | `self.time_entity` |
| RenderConfig | RenderConfig | 渲染开关、FPS限制、LOD | `self.config_entity` |
| VisibleArea | VisibleArea | 可见区域缓存（优化渲染） | `self.visible_area_entity` |
| DebugCounters | DebugCounters | 调试日志计数器 | `self.debug_counters_entity` |
| MouseInput | MouseInput | 鼠标状态（按下、双击、位置） | 匿名实体 |
| MainDialog | MainDialog | 主UI面板（血条、技能栏等） | `self.main_dialog_entity` |
| InventoryDialog | InventoryDialog | 背包界面 | `self.inventory_dialog_entity` |
| CharacterDialog | CharacterDialog | 角色属性界面 | `self.character_dialog_entity` |
| SkillBarDialog×2 | SkillBarDialog | 技能快捷栏（F1-F8） | `self.skillbar_entities[0/1]` |
| ChatDialog | ChatDialog | 聊天框 | `self.chat_dialog_entity` |
| MagicLearningDialog | MagicLearningDialog | 技能学习界面 | `self.magic_learning_dialog_entity` |
| QuestDialog | QuestDialog | 任务界面 | `self.quest_dialog_entity` |
| TradeDialog | TradeDialog | 交易界面 | `self.trade_dialog_entity` |
| SkillsDialog | SkillsDialog | 技能树界面 | 匿名实体 |
| OptionsDialog | OptionsDialog | 设置界面 | 匿名实体 |
| HotkeyHelpPanel | HotkeyHelpPanel | 快捷键帮助面板 | 匿名实体 |

### ❌ 游戏世界实体（阶段 2 - 缺失）

| 实体 | 组件 | 用途 | 数据来源 |
|------|------|------|----------|
| **GlobalEvents** | GlobalEvents | 输入事件总线（鼠标、键盘） | 初始化时创建 |
| **LocalPlayer** | 见下文详细列表 | 本地玩家角色 | UserInformation + CharacterList |
| **MapData** | MapData | 当前地图数据（文件路径、尺寸） | UserInformation.map_name |
| **MapTile×N** | MapTile | 地图瓦片（可见区域内） | 地图文件 .map |

#### LocalPlayer 完整组件列表

```rust
world.spawn((
    // ===== 核心标识 =====
    LocalPlayer,                    // 标记组件：本地玩家
    
    // ===== 位置与朝向 =====
    Position {
        x: location_x as f32,       // 从 UserInformation
        y: location_y as f32,
    },
    Direction(MirDirection::Up),    // 初始朝向（或从角色选择数据）
    
    // ===== 角色数据 =====
    PlayerData {
        object_id: 1,               // 玩家唯一ID（从服务器）
        name: character_name,        // 从 CharacterList.selected
        class: MirClass::Warrior,    // 从 CharacterList.selected
        gender: MirGender::Male,     // 从 CharacterList.selected
        level: 1,                    // 从服务器或角色数据
        experience: 0,
        max_experience: 100,
        // ... 其他属性
    },
    
    // ===== 外观 =====
    PlayerAppearance {
        class: MirClass::Warrior,
        gender: MirGender::Male,
        hair: 0,
        weapon: -1,
        weapon_effect: -1,
        armour: 0,
        // ...
    },
    
    // ===== 生命与法力 =====
    Health {
        current: hp,                // 从 UserInformation
        max: max_hp,                // 从服务器或默认值
    },
    Mana {
        current: mp,                // 从 UserInformation
        max: max_mp,                // 从服务器或默认值
    },
    
    // ===== 装备与物品 =====
    Equipment::default(),           // 空装备栏（后续更新）
    Inventory::new(46),             // 46格背包
    
    // ===== 技能 =====
    MagicList::default(),           // 已学技能
    LearnableMagicList::default(),  // 可学技能
    
    // ===== 运动与动画 =====
    Velocity { dx: 0.0, dy: 0.0 }, // 移动速度
    AnimationState::default(),      // 动画状态机
    
    // ===== 音效 =====
    SoundEmitter::default(),        // 音效发射器
));
```

---

## ECS系统启动

### 系统调度器架构

**调度器**: `UpdateRenderParallelScheduler`  
**执行模式**: `ExecutionMode::Parallel` (Layer 5 并行)  
**创建位置**: `GameScene::new()` 构造函数

```rust
self.system_scheduler = UpdateRenderParallelScheduler::new(ExecutionMode::Parallel);
```

### 六层系统架构

#### Layer 1: 输入与网络 (50-199)
```rust
Priority 110: PlayerControlSystem
  职责: 从 GlobalEvents 读取输入 → 生成玩家命令
  依赖: GlobalEvents 组件 ⚠️
  输入: 鼠标点击、键盘按键
  输出: 移动命令、攻击命令、技能释放
```

#### Layer 2: AI 与决策 (200-299)
```rust
Priority 200: MonsterAISystem
  职责: 怪物AI（巡逻、追击、攻击决策）
  
Priority 220: NpcDialogueSystem
  职责: NPC对话、任务、商店交互
```

#### Layer 3: 战斗与技能 (300-399)
```rust
Priority 300: SkillSystem
  职责: 技能释放、冷却、效果应用
  
Priority 310: CombatSystem
  职责: 伤害计算、命中判定、死亡判断
```

#### Layer 4: 移动与物理 (400-499)
```rust
Priority 400: MovementSystem
  职责: 实体移动、路径追踪、速度计算
  
Priority 410: CollisionSystem
  职责: 碰撞检测、障碍物判断
```

#### Layer 5: 状态更新 (500-599) [可并行执行]
```rust
Priority 500: AnimationSystem
  职责: 动画状态机、帧切换
  
Priority 510: ParticleSystem
  职责: 粒子效果更新、生命期管理
  
Priority 515: HealthRegenSystem
  职责: 血量/法力自动回复
  
Priority 520: SoundSystem
  职责: 音效触发、3D音效位置
  
Priority 530: CameraSystem
  职责: 相机矩阵计算、震动效果
```

#### Layer 6: 网络同步 (600-699)
```rust
Priority 595: ClientPredictionSystem
  职责: 客户端预测、状态回滚
  
Priority 600: NetworkSendSystem
  职责: 收集状态变化、发送数据包
  
Priority 610: SyncSystem
  职责: 状态同步验证、断线重连
```

### 渲染系统（独立调用）

**调用位置**: `GameScene::draw()`  
**渲染顺序**: 地图 → 实体 → 特效 → UI → 调试

```rust
// TODO: 使用 UpdateRenderParallelScheduler::render()
// self.system_scheduler.render(ctx, canvas, world)?;
```

---

## 依赖关系分析

### 关键依赖链

```
[输入事件] 
    ↓
GlobalEvents 组件 ⚠️ 必须存在
    ↓
PlayerControlSystem (读取事件)
    ↓
MovementSystem / CombatSystem
    ↓
AnimationSystem / SoundSystem
    ↓
CameraSystem (计算视图矩阵)
    ↓
[渲染输出]
```

### 系统间依赖

| 依赖方 | 被依赖方 | 依赖内容 |
|--------|----------|----------|
| PlayerControlSystem | GlobalEvents | 必须存在，否则无法读取输入 |
| CameraSystem | LocalPlayer + Position | 相机跟随玩家 |
| AnimationSystem | PlayerData + AnimationState | 根据动作更新动画 |
| RenderSystem | Camera + Position | 世界坐标 → 屏幕坐标 |
| All Update Systems | TimeTracker | delta_time 时间增量 |

---

## 当前问题

### 问题 1: GlobalEvents 组件缺失

**日志现象**:
```
⚠️ PlayerControlSystem: GlobalEvents 组件未找到
```

**原因**: `GameScene::initialize()` 未创建 GlobalEvents 实体

**影响**:
- PlayerControlSystem 每帧警告
- 无法处理玩家输入（鼠标点击、键盘按键）
- 玩家无法移动、攻击、使用技能

**解决**: 在 `initialize()` 中创建
```rust
world.spawn((GlobalEvents::new(),));
```

### 问题 2: LocalPlayer 实体缺失

**日志现象**:
```
⚠️ draw#1: 未找到LocalPlayer!
```

**原因**: 未处理 `UserInformation` 事件创建玩家实体

**影响**:
- 相机无法跟随玩家（draw 时查询失败）
- 无法渲染玩家角色
- 所有玩家相关系统无法工作

**解决**: 在 `NetworkEventSystem` 处理 `UserInformation` 时创建

---

## 实现方案

### 方案 A: 分离初始化 ✅ 推荐

**优点**: 职责清晰、符合事件驱动架构

```rust
// 步骤 1: GameScene::initialize() - 创建基础事件总线
fn initialize(&mut self, ctx: &mut Context, world: &mut World) -> GameResult {
    // ... 现有代码 ...
    
    // 🆕 创建全局事件总线（空）
    world.spawn((GlobalEvents::new(),));
    
    println!("✅ GameScene初始化完成！");
    Ok(())
}

// 步骤 2: NetworkEventSystem::UserInformation - 创建游戏世界
impl NetworkEventSystem {
    pub fn process_event(world: &mut World, event: &GameEvent) {
        match event {
            GameEvent::UserInformation { location_x, location_y, hp, mp, gold } => {
                Self::initialize_game_world(world, *location_x, *location_y, *hp, *mp, *gold);
            }
            // ...
        }
    }
    
    fn initialize_game_world(
        world: &mut World,
        location_x: i32,
        location_y: i32,
        hp: i32,
        mp: i32,
        gold: u32,
    ) {
        tracing::info!("🎮 初始化游戏世界...");
        
        // 1. 从 CharacterList 获取角色信息
        let (character_name, class, gender) = {
            let mut query = world.query::<&CharacterList>();
            if let Some((_, char_list)) = query.iter().next() {
                let selected = &char_list.characters[char_list.selected_index];
                (selected.name.clone(), selected.class, selected.gender)
            } else {
                panic!("CharacterList not found!");
            }
        };
        
        // 2. 创建 LocalPlayer 实体（完整组件）
        world.spawn((
            LocalPlayer,
            Position { x: location_x as f32, y: location_y as f32 },
            Direction(MirDirection::Up),
            PlayerData {
                object_id: 1,
                name: character_name.clone(),
                class,
                gender,
                level: 1,
                // ...
            },
            PlayerAppearance { class, gender, /* ... */ },
            Health { current: hp, max: 100 },
            Mana { current: mp, max: 100 },
            Equipment::default(),
            Inventory::new(46),
            MagicList::default(),
            LearnableMagicList::default(),
            Velocity { dx: 0.0, dy: 0.0 },
            AnimationState::default(),
            SoundEmitter::default(),
        ));
        
        // 3. 加载地图数据
        // TODO: 需要地图名称（从服务器或默认）
        // Self::load_map(world, "0");
        
        tracing::info!("✅ 游戏世界初始化完成: {} ({}, {})", character_name, location_x, location_y);
    }
}
```

### 方案 B: 统一初始化（不推荐）

在 `GameScene::initialize()` 中创建所有实体，包括假的 LocalPlayer

**缺点**: 
- 需要假数据，后续还要更新
- UserInformation 数据到达时需要查找并更新实体
- 违反事件驱动原则

---

## 下一步行动

### 立即实施（修复当前问题）

1. ✅ **在 `GameScene::initialize()` 添加 GlobalEvents**
   ```rust
   world.spawn((GlobalEvents::new(),));
   ```

2. ✅ **实现 `NetworkEventSystem::initialize_game_world()`**
   - 创建 LocalPlayer 实体（完整组件）
   - 从 CharacterList 读取角色信息
   - 设置初始位置、血量、法力

3. ⏳ **测试验证**
   - 确认 PlayerControlSystem 不再报警
   - 确认 draw 能找到 LocalPlayer
   - 确认相机跟随玩家

### 后续迭代

4. ⏳ **实现地图加载**
   - 从服务器获取地图名称
   - 加载 .map 文件
   - 创建 MapData 和 MapTile 实体

5. ⏳ **完善玩家数据**
   - 从服务器同步完整属性（力量、敏捷等）
   - 同步装备数据
   - 同步技能列表

6. ⏳ **实现其他实体**
   - 怪物实体（MonsterData + AI组件）
   - NPC实体（NPCData + 对话组件）
   - 其他玩家实体（RemotePlayer + 网络同步）

---

## 参考代码位置

| 功能 | 文件路径 |
|------|----------|
| GameScene | `src/ecs/scenes/game_scene.rs` |
| NetworkEventSystem | `src/ecs/systems/network_event_system.rs` |
| 组件定义 | `src/ecs/components/` |
| 系统定义 | `src/ecs/systems/update/` |
| 系统调度器 | `src/ecs/update_render_parallel_scheduler.rs` |
| CharacterList | `src/ecs/components/character_select.rs` |

---

## 附录：调试命令

```bash
# 查看详细日志
$env:RUST_LOG="trace"; cargo run --bin mir2x

# 只看特定模块
$env:RUST_LOG="mir2_client::ecs::systems=debug"; cargo run --bin mir2x

# 查看所有警告
cargo run --bin mir2x 2>&1 | Select-String "WARN"
```

---

**文档维护**: 此文档应随代码实现同步更新
