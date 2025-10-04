# C# Client 直接移植路线图

**策略**: 按照 C# Client 的原始结构直接移植，不做架构改动  
**目标**: 快速完成功能移植，让游戏能跑起来  
**重构**: 等移植完成后再根据实际需求进行

---

## 📂 C# Client 模块结构

```
Client/
├── MirObjects/         ← 游戏对象（优先级最高）
├── MirScenes/          ← 场景系统
├── MirControls/        ← UI 控件
├── MirGraphics/        ← 图形渲染
├── MirSounds/          ← 音频系统
├── MirNetwork/         ← 网络层（已完成）
├── Forms/              ← 窗体
├── Resolution/         ← 分辨率管理
└── Utils/              ← 工具类
```

## 🎯 移植优先级和顺序

### Phase 1: MirObjects 模块 (对应 `src/objects/`)

**目标**: 所有游戏对象能正常创建、更新和渲染

#### 1.1 核心对象 (P0 - 必须) ⏰ 预计 3-5 天

```
✅ MapObject.cs        → map_object.rs     (基本完成)
🔄 UserObject.cs       → user_object.rs    (大量 TODO)
🔄 MonsterObject.cs    → monster_object.rs (大量 TODO)
✅ NPCObject.cs        → npc_object.rs     (基本完成)
⏳ ItemObject.cs       → item_object.rs    (未完成)
⏳ SpellObject.cs      → spell_object.rs   (未完成)
⏳ HeroObject.cs       → hero_object.rs    (未完成)
✅ MapCode.cs          → map_code.rs       (基本完成 - CellInfo + MapReader)
```

**任务清单**:
- [ ] 完成 `UserObject` 的所有 TODO
  - [ ] 实现完整的 `update()` 方法
  - [ ] 实现 `draw()` 方法
  - [ ] 实现装备渲染
  - [ ] 实现动作动画
- [ ] 完成 `MonsterObject` 的所有 TODO
  - [ ] 实现 AI 逻辑
  - [ ] 实现攻击动画
  - [ ] 实现死亡效果
- [ ] 移植 `ItemObject`
  - [ ] 掉落物品渲染
  - [ ] 拾取交互
- [ ] 移植 `SpellObject`
  - [ ] 技能效果渲染
  - [ ] 飞行物轨迹
- [ ] 移植 `HeroObject`
  - [ ] 英雄跟随逻辑
  - [ ] 英雄战斗

#### 1.2 辅助类 (P1) ⏰ 预计 2-3 天

```
✅ Frames.cs          → frames.rs         (完成)
🔄 Effect.cs          → effect.rs         (部分完成)
🔄 Damage.cs          → damage.rs         (部分完成)
⏳ PathFinder.cs      → pathfinder.rs     (未完成)
⏳ DecoObject.cs      → (暂时跳过)
```

**MapCode.cs 移植详情** (2025-10-04 完成):
- ✅ `CellInfo` 类: 包含地形数据、对象管理、钓鱼点等 (650+ lines)
- ✅ `MapReader` 类: 支持 4 种地图格式（Type 0-3）
- ⏳ 待完成: Type 4-7, 100 格式支持
- 📄 文档: `docs/map-code-reorganization.md`

**任务清单**:
- [ ] 完成 `Effect` 系统
  - [ ] 各种特效类型
  - [ ] 粒子效果
- [ ] 完成 `Damage` 显示
  - [ ] 伤害数字飘字
  - [ ] 颜色和动画
- [ ] 实现 `PathFinder`
  - [ ] A* 寻路算法
  - [ ] 碰撞检测

### Phase 2: MirScenes 模块 (对应 `src/scenes/`)

**目标**: 场景能正常切换和显示

#### 2.1 核心场景 (P0) ⏰ 预计 4-6 天

```
🔄 GameScene.cs       → game_scene.rs     (大量 TODO)
   ├── MapControl     → ⚠️ 先用 C# 的方式实现
   │   ├── 地图数据   → 直接在 GameScene 中
   │   ├── 对象管理   → Objects: HashMap<u32, Box<dyn MapObject>>
   │   └── 渲染逻辑   → draw_map() 方法
   └── 其他功能
   
✅ LoginScene.cs      → login_scene.rs    (基本完成?)
✅ SelectScene.cs     → select_scene.rs   (基本完成?)
```

**GameScene 关键功能**:
- [ ] 地图加载和显示
  ```rust
  // 按照 C# 的方式，不使用独立的 map 模块
  pub struct GameScene {
      // 地图数据 (对应 MapControl.M2CellInfo)
      map_cells: Option<Vec<Vec<CellInfo>>>,
      map_width: i32,
      map_height: i32,
      
      // 相机参数 (对应 MapControl 静态变量)
      offset_x: i32,
      offset_y: i32,
      view_range_x: i32,
      view_range_y: i32,
      
      // 对象管理 (对应 MapControl.Objects)
      objects: HashMap<u32, Box<dyn MapObject>>,
      objects_list: Vec<u32>,
      
      // ... 其他字段
  }
  ```
- [ ] 对象管理和渲染
- [ ] 输入处理
- [ ] UI 集成

#### 2.2 场景基础设施 (P1) ⏰ 预计 1-2 天

```
✅ Scene trait        → scene_trait.rs    (完成)
⏳ 场景切换管理
⏳ 过场动画
```

### Phase 3: MirScenes/Dialogs 模块 (对应 `src/scenes/dialogs/`)

**目标**: 所有 UI 对话框能显示和交互

#### 3.1 核心 UI (P0) ⏰ 预计 5-7 天

```
🔄 MainDialog.cs      → main_dialog.rs    (大量 TODO)
🔄 ChatDialog.cs      → chat_dialog.rs    (TODO)
🔄 InventoryDialog.cs → inventory_dialog.rs (TODO)
🔄 CharacterDialog.cs → character_dialog.rs (TODO)
⏳ SkillBarDialog.cs  → skillbar_dialog.rs (TODO)
... (还有 20+ 对话框)
```

**任务清单**:
- [ ] 实现所有对话框的 `draw()` 方法
- [ ] 实现鼠标交互
- [ ] 实现键盘快捷键
- [ ] 对话框之间的通信

### Phase 4: MirGraphics 模块 (对应 `src/graphics/`)

**目标**: 完整的图形渲染系统

#### 4.1 纹理和渲染 (P0) ⏰ 预计 3-4 天

```
✅ MLibrary          → texture_loader.rs  (MLibrary 完成)
✅ SpriteRenderer    → sprite_renderer.rs (完成)
🔄 CharacterRenderer → character_renderer.rs (基本完成)
⏳ 地图渲染         → 在 GameScene 中实现
⏳ UI 渲染          → 在各 Dialog 中实现
```

**任务清单**:
- [ ] 完成角色渲染的所有层级
- [ ] 实现地图瓦片渲染（在 GameScene 中）
- [ ] 实现特效渲染
- [ ] 优化批量渲染

### Phase 5: MirSounds 模块 (对应 `src/sounds/`)

**目标**: 音效和音乐播放

⏰ 预计 2-3 天

```
🔄 SoundManager      → mod.rs (基本框架)
⏳ 音效加载
⏳ 音乐播放
⏳ 3D 音效定位
```

### Phase 6: 其他模块

#### 6.1 MirControls (如果需要)

```
⏳ 按钮、标签等 UI 控件
⏳ 输入处理
```

#### 6.2 Forms

```
⏳ CMain.cs → 主窗体逻辑
```

#### 6.3 Utils

```
⏳ 各种工具函数
```

---

## 📅 时间估算

| 阶段 | 预计时间 | 说明 |
|------|---------|------|
| Phase 1: Objects | 5-8 天 | 核心对象和辅助类 |
| Phase 2: Scenes | 5-8 天 | GameScene 是重点 |
| Phase 3: Dialogs | 5-7 天 | 大量 UI，可并行开发 |
| Phase 4: Graphics | 3-4 天 | 大部分已完成 |
| Phase 5: Sounds | 2-3 天 | 相对独立 |
| Phase 6: 其他 | 3-5 天 | 按需实现 |
| **总计** | **23-35 天** | **约 1-1.5 个月全职开发** |

## 🎯 里程碑

### 里程碑 1: 基础对象系统 (1 周)
- ✅ 所有核心对象能创建和更新
- ✅ MapObject、UserObject、MonsterObject、NPCObject 完成

### 里程碑 2: 游戏场景可显示 (2 周)
- ✅ GameScene 能加载地图
- ✅ 能看到玩家角色
- ✅ 能看到怪物和 NPC
- ✅ 基本的相机跟随

### 里程碑 3: 基本交互 (3 周)
- ✅ 角色能移动
- ✅ 能攻击怪物
- ✅ 能拾取物品
- ✅ 基本 UI 能显示

### 里程碑 4: 完整游戏 (4-5 周)
- ✅ 所有 UI 对话框
- ✅ 所有技能效果
- ✅ 完整音效
- ✅ 游戏可玩

---

## 🔧 实施原则

### 1. **严格遵循 C# 结构**
```rust
// ✅ 正确做法：直接对应 C# 的类和字段
pub struct GameScene {
    // 对应 MapControl.M2CellInfo[,]
    map_cells: Option<Vec<Vec<CellInfo>>>,
    
    // 对应 MapControl.Objects
    objects: HashMap<u32, Box<dyn MapObject>>,
    
    // 对应 MapControl.OffSetX/Y
    offset_x: i32,
    offset_y: i32,
}

// ❌ 错误做法：过早抽象
pub struct GameScene {
    map_data: MapData,        // 新设计的抽象
    map_renderer: MapRenderer, // 新设计的抽象
    camera: Camera,           // 新设计的抽象
}
```

### 2. **逐文件移植**
- 一次移植一个 C# 文件
- 保持类名、方法名尽量一致
- 字段对应关系清晰

### 3. **功能优先**
- 先让功能跑起来
- 不追求完美的代码结构
- TODO 注释标记未完成部分

### 4. **增量测试**
- 每完成一个对象就测试
- 尽早发现问题
- 避免积累技术债务

---

## 📝 当前行动计划

### 本周目标 (10.04 - 10.10)

**Day 1-2**: 完成 UserObject
- [ ] 补全所有 TODO
- [ ] 实现 update() 和 draw()
- [ ] 测试角色显示

**Day 3-4**: 完成 MonsterObject
- [ ] 补全所有 TODO
- [ ] 实现 AI 和动画
- [ ] 测试怪物显示

**Day 5**: 完成 ItemObject 和 SpellObject
- [ ] 基本实现
- [ ] 能显示即可

**Day 6-7**: 开始 GameScene
- [ ] 按 C# 的方式重新组织代码
- [ ] 实现地图加载和显示
- [ ] 测试能看到地图

### 下周目标 (10.11 - 10.17)

**完成 GameScene 核心功能**
- 对象管理
- 输入处理
- 基本渲染

---

## 🚫 移植期间禁止的事项

1. ❌ 不要创建新的抽象模块（如独立的 map 模块）
2. ❌ 不要改变 C# 的类结构
3. ❌ 不要优化"不完美"的设计
4. ❌ 不要过度设计接口和 trait
5. ❌ 不要纠结"Rust 的最佳实践"

**记住**: 移植完成 > 代码完美

---

## ✅ 移植完成标准

### 最小可玩版本 (MVP)
- [ ] 能登录
- [ ] 能选择角色
- [ ] 能进入游戏
- [ ] 能看到地图
- [ ] 能移动角色
- [ ] 能看到其他玩家/怪物
- [ ] 能攻击怪物
- [ ] 基本 UI 能用

### 完整版本
- [ ] 所有 C# Client 的功能都能用
- [ ] 网络同步正常
- [ ] UI 完整
- [ ] 音效正常
- [ ] 无明显 bug

---

## 🔄 移植完成后的重构计划

**等移植完成后，再根据实际情况决定**：

1. 是否使用已经实现的 `src/map/` 模块
2. 是否需要提取其他公共模块
3. 是否需要架构优化
4. 是否需要性能优化

**优先级**: 功能完整 > 性能 > 代码质量

---

**文档版本**: v1.0  
**创建日期**: 2025-10-04  
**状态**: 执行中  
**下一次更新**: 每周五
