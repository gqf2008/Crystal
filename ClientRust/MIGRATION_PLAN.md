# Client 工程完整移植计划

## 概述
完整移植 C# Client 工程到 Rust，保持原有的架构和游戏逻辑。

## Client 工程结构分析

### 1. 核心模块
```
Client/
├── Program.cs                 # 入口点
├── Settings.cs                # 配置管理
├── KeyBindSettings.cs         # 键盘绑定
├── MirScenes/                 # 场景系统 ⭐核心
│   ├── GameScene.cs          # 游戏主场景 (12,297行)
│   ├── LoginScene.cs         # 登录场景
│   ├── SelectScene.cs        # 角色选择场景
│   └── Dialogs/              # 各种对话框 (40+个)
├── MirObjects/               # 游戏对象 ⭐核心
│   ├── MapObject.cs          # 地图对象基类
│   ├── UserObject.cs         # 玩家对象
│   ├── PlayerObject.cs       # 其他玩家
│   ├── MonsterObject.cs      # 怪物对象
│   ├── NPCObject.cs          # NPC对象
│   ├── ItemObject.cs         # 地面物品
│   ├── HeroObject.cs         # 英雄对象
│   ├── UserHeroObject.cs     # 玩家的英雄
│   ├── Effect.cs             # 特效系统
│   ├── Frames.cs             # 动画帧管理
│   ├── SpellObject.cs        # 法术对象
│   ├── Damage.cs             # 伤害显示
│   └── PathFinder.cs         # 寻路算法
├── MirControls/              # UI控件系统
│   ├── MirControl.cs         # 控件基类
│   ├── MirButton.cs          # 按钮
│   ├── MirLabel.cs           # 标签
│   ├── MirItemCell.cs        # 物品格子
│   └── ...                   # 20+个控件
├── MirGraphics/              # 图形渲染
│   ├── DXManager.cs          # DirectX管理器
│   ├── MLibrary.cs           # 资源库管理
│   └── ParticleEngine.cs     # 粒子系统
├── MirSounds/                # 音效系统
│   └── SoundManager.cs       # 音效管理器
├── MirNetwork/               # 网络层
│   └── Network.cs            # 网络通信
└── Utils/                    # 工具类
    └── FileHelper.cs         # 文件辅助

```

### 2. GameScene.cs 核心功能分析

#### 2.1 静态成员（游戏状态）
```csharp
- User: UserObject              // 当前玩家
- Hero: UserHeroObject          // 玩家的英雄
- Gold, Credit: uint            // 金币、点券
- Storage[80]: UserItem         // 仓库
- GuildStorage[112]: UserItem   // 公会仓库
- ItemInfoList: List<ItemInfo>  // 物品信息列表
- QuestInfoList: List<...>      // 任务列表
- HeroStorage[8]: ...           // 英雄存储
- MapInfoList: Dictionary       // 地图信息
```

#### 2.2 对话框实例（约40个）
```csharp
- MainDialog: 主界面
- ChatDialog: 聊天
- InventoryDialog: 背包
- CharacterDialog: 角色面板
- GuildDialog: 公会
- QuestListDialog: 任务列表
- TradeDialog: 交易
- NPCDialog: NPC对话
... 等等
```

#### 2.3 核心方法
```csharp
- ProcessPacket(): 处理服务器数据包（2000+行）
- OnMouseMove(), OnMouseClick(): 鼠标交互
- Draw(): 渲染主循环
- Update(): 逻辑更新
- 各种数据包处理函数（300+个）
```

## Rust 移植架构设计

### 阶段 0：协议层 ✅ (已完成 40%)
```
src/protocol.rs         # 数据包定义和解析
src/state.rs           # 客户端状态管理
src/ui.rs              # 临时UI占位符
```

### 阶段 1：核心对象系统 ✅ (已完成 - 100%)
```
src/game/objects/
├── mod.rs             # 模块导出 ✅
├── map_object.rs      # MapObject基类 ✅
├── frames.rs          # 动画系统 ✅
├── user_object.rs     # UserObject (玩家) ✅
├── monster_object.rs  # MonsterObject ✅
├── npc_object.rs      # NPCObject ✅
├── item_object.rs     # ItemObject ✅
├── hero_object.rs     # HeroObject ✅
├── spell_object.rs    # SpellObject ✅
├── effect.rs          # Effect特效 ✅
├── damage.rs          # Damage伤害显示 ✅
└── pathfinder.rs      # PathFinder寻路算法 ✅

总计: 12个文件, 3,413行代码, 34+单元测试
详见: STAGE1_FINAL_REPORT.md
```

### 阶段 2：场景系统
```
src/game/scenes/
├── mod.rs
├── game_scene.rs      # GameScene主场景
├── login_scene.rs     # LoginScene
├── select_scene.rs    # SelectScene
└── dialogs/           # 对话框模块
    ├── mod.rs
    ├── main_dialog.rs
    ├── chat_dialog.rs
    ├── inventory_dialog.rs
    ├── character_dialog.rs
    ├── guild_dialog.rs
    ├── quest_dialogs.rs
    ├── trade_dialogs.rs
    └── ... (40+个对话框)
```

### 阶段 3：控件系统
```
src/game/controls/
├── mod.rs
├── mir_control.rs     # 基类
├── mir_button.rs
├── mir_label.rs
├── mir_item_cell.rs
├── mir_textbox.rs
└── ... (20+个控件)
```

### 阶段 4：图形渲染
```
src/game/graphics/
├── mod.rs
├── renderer.rs        # 渲染器（替代DXManager）
├── mlibrary.rs        # 资源加载
├── particle.rs        # 粒子系统
└── texture.rs         # 纹理管理
```

### 阶段 5：音效系统
```
src/game/sounds/
├── mod.rs
└── sound_manager.rs   # 音效管理
```

## 当前状态评估

### ✅ 已完成
1. **协议层** (100%) ✅
   - Phase A: 16个模块, 2,474行代码
   - Phase B: protocol.rs 路由系统, 2,437行
   - 205个数据包完整实现
   - 详见: PHASE_B_FINAL_REPORT.md

2. **Stage 1: 核心对象系统** (100%) ✅
   - 12个对象模块, 3,413行代码
   - 9个核心对象全部实现
   - 34+单元测试, 全部通过
   - 详见: STAGE1_FINAL_REPORT.md

### 🔄 正在进行
- **Stage 2: 场景系统** (65%)
  - ✅ Scene框架 (scene_trait.rs)
  - ✅ 3个核心场景 (Login, Select, Game)
  - ✅ 23个对话框实现 (8,702行代码, 223测试)
    - ✅ 核心对话框 (4/4): Main, Chat, Inventory, Character
    - ✅ 功能对话框 (3/3): SkillBar, NPC, Storage
    - ✅ 社交对话框 (4/4): Trade, Guild, Friend, Group
    - ✅ 功能性对话框 (4/4): BigMap, QuestList, Mail×2, Help
    - ✅ 游戏系统对话框 (8/8): Belt, Timer, Socket, Buff, Mount, Fishing, Refine, Craft ← NEW
  - ⏳ 17+个剩余对话框
  - ⏳ DialogManager (对话框管理器)
  - ⏳ 场景管理器

### ⏳ 待开始
1. **Stage 3: 控件系统**
   - MirControl基类
   - 20+个UI控件
   - 事件处理

2. **Stage 4: 渲染系统**
   - 图形渲染引擎
   - 资源加载 (MLibrary)
   - 粒子效果

3. **Stage 5: 音效系统**
   - 音效管理器
   - 背景音乐

## 移植策略

### 1. 自底向上原则
```
协议层 → 数据模型 → 对象系统 → 场景系统 → UI系统 → 渲染系统
```

### 2. 保持C#架构
- **类层次结构**: 保持继承关系（用 trait 实现）
- **方法命名**: 尽量保持一致（转为 snake_case）
- **逻辑流程**: 完全遵循 C# 实现

### 3. Rust 特性利用
- **所有权系统**: 避免垃圾回收
- **类型安全**: 编译时检查
- **性能优化**: Zero-cost abstractions

### 4. 测试驱动
- 每个模块完成后立即测试
- 对比 C# 行为确保一致性

## 下一步行动

### ✅ 已完成任务
1. ✅ Phase A: 16个协议模块 (2,474行)
2. ✅ Phase B: protocol.rs路由 (2,437行)
3. ✅ Stage 1: 核心对象系统 (3,413行, 9对象)
4. ✅ 所有单元测试通过 (34+测试)

### 立即任务 (Stage 2 启动)
1. ⏳ 创建 src/game/scenes/ 目录结构
2. ⏳ 实现 GameScene 基础框架
3. ⏳ 开始主对话框 (MainDialog, ChatDialog)
4. ⏳ 场景切换管理器

### 短期计划（2周）
1. 完成 GameScene 核心逻辑
2. 完成 10个基础对话框
3. 实现场景渲染循环
4. 对象管理器集成

### 中期计划（1月）
1. 完成所有40+对话框
2. 完成控件系统 (Stage 3)
3. 开始渲染系统 (Stage 4)
4. 资源加载器

### 长期计划（2月）
1. 完整的渲染引擎
2. 音效系统
3. 网络集成
4. 可玩的游戏客户端 🎮

## 关键挑战

### 技术挑战
1. **DirectX → Rust图形库**: 选择 wgpu 或 glow
2. **WinForms → Rust UI**: 选择 egui 或自定义
3. **多线程模型**: Rust 所有权系统下的线程管理
4. **资源加载**: MLibrary 格式解析

### 架构挑战
1. **回调系统**: C# 事件 → Rust 闭包
2. **继承vs组合**: trait + enum 替代类继承
3. **状态管理**: 避免循环引用

## 成功标准

### 阶段性目标
- [x] 协议层 100% 完成 (205 个数据包) ✅
- [x] Stage 1: 核心对象系统 (9个对象) ✅
- [ ] Stage 2: 场景系统 (GameScene + 40对话框)
- [ ] Stage 3: 控件系统 (20+控件)
- [ ] Stage 4: 渲染系统 (图形引擎)
- [ ] 可以连接服务器并显示地图
- [ ] 可以移动角色
- [ ] 可以看到其他玩家和怪物
- [ ] 可以攻击和使用技能
- [ ] 完整的 UI 交互
- [ ] 与 C# 客户端功能对等

### 完成进度
```
协议层:        [████████████████████████] 100% ✅
Stage 1 对象:  [████████████████████████] 100% ✅
Stage 2 场景:  [███████                 ]  30% ⏳
Stage 3 控件:  [                        ]   0% 
Stage 4 渲染:  [                        ]   0% 
总体进度:      [██████████              ]  38%
```

### 最终目标
**完全替代 C# 客户端，性能更优，跨平台运行** 🎯

---

## 参考文档
- C# Client 源码: `d:\Users\gxh\Documents\GitHub\Crystal\Client\`
- GameScene.cs: 12,297 行 - 核心游戏逻辑
- ServerPackets.cs: 数据包定义参考

## 备注
- **不要重新发明轮子**: 严格参考 C# 实现
- **保持代码可读性**: 清晰的注释和文档
- **渐进式开发**: 每个模块独立测试
