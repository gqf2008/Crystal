# GameScene UI 实现 TODO

## 📋 总体目标
实现传奇2游戏主场景的所有UI交互部分，游戏逻辑由ECS系统处理。

---

## 🎯 阶段一：核心UI框架（优先级：⭐⭐⭐⭐⭐）

### 1. MainDialog - 主界面底部工具栏
**文件**: `src/scenes/dialogs/game/main_dialog.rs`

**功能清单**:
- [x] 底部工具栏背景（根据分辨率适配：800/1024/1280+）
- [x] 生命值球（HealthOrb）+ 数值显示（Prguse[4] 动态裁剪）
- [x] 魔法值球（ManaOrb）+ 数值显示（Prguse[4] 右半部分）
- [x] 经验条（ExperienceBar）+ 百分比显示（Prguse[7/8] 动态裁剪）
- [x] 负重条（WeightBar）+ 数值显示（Prguse[76] 动态裁剪）
- [x] 角色名称、等级显示
- [x] 金币显示
- [x] 背包空格显示
- [x] 功能按钮组：
  - [x] 背包按钮（InventoryButton）- Prguse[1903-1905]
  - [x] 角色按钮（CharacterButton）- Prguse[1900-1902]
  - [x] 技能按钮（SkillButton）- Prguse[1906-1908]
  - [x] 任务按钮（QuestButton）- Prguse[1909-1911]
  - [x] 选项按钮（OptionButton）- Prguse[1912-1914]
  - [x] 菜单按钮（MenuButton）- Prguse[1960-1962]
  - [x] 商城按钮（GameShopButton）- Prguse[826-828]
- [ ] 攻击模式显示（AMode/PMode/SMode）
- [ ] 英雄信息面板（HeroInfoPanel）
- [ ] 英雄行为面板（HeroBehaviourPanel）
- [ ] 英雄菜单/召唤按钮

**纹理资源**:
- 主背景: Prguse[0/1/2] (根据分辨率)
- 左右扩展: Prguse[12/13]
- 按钮: Prguse[1900-1914, 1960-1962, 826-828]
- 生命/魔法球: Prguse[4]
- 经验条: Prguse[7/8]
- 负重条: Prguse[76]

**状态**: ✅ 基础功能已完成，待实现攻击模式和英雄面板

---

### 2. BeltDialog - 快捷栏（血瓶框）
**文件**: `src/scenes/dialogs/game/belt_dialog.rs`

**功能清单**:
- [ ] 快捷栏背景（水平/垂直两种布局）
- [ ] 6个物品格子
- [ ] 物品图标显示
- [ ] 物品数量显示
- [ ] 物品拖拽（与背包交互）
- [ ] 数字键提示（1-6）
- [ ] 旋转按钮（切换水平/垂直布局）
- [ ] 关闭按钮
- [ ] 半透明背景层

**纹理资源**:
- 水平布局: Prguse[1932-1933]
- 垂直布局: Prguse[1944-1945]
- 旋转按钮: Prguse[1926-1928, 1938-1940]
- 关闭按钮: Prguse[1923-1925, 1935-1937]

**布局位置**:
- 水平: MainDialog上方居中 (230, -150 相对偏移)
- 垂直: 左侧 (0, 200)

**状态**: ⏳ 待实现

---

### 3. ChatDialog - 聊天窗口
**文件**: `src/scenes/game/dialogs/chat_dialog.rs`

**功能清单**:
- [ ] 聊天窗口背景
- [ ] 聊天消息滚动列表
- [ ] 多频道支持（全部/公会/组队/私聊等）
- [ ] 频道切换标签
- [ ] 聊天输入框
- [ ] 发送按钮
- [ ] 消息过滤选项
- [ ] 表情功能
- [ ] 链接点击（物品/坐标等）
- [ ] 聊天记录保存

**纹理资源**:
- 待查询原工程

---

### 3. InventoryDialog - 背包
**文件**: `src/scenes/game/dialogs/inventory_dialog.rs`

**功能清单**:
- [ ] 背包窗口背景
- [ ] 物品格子（46格）
- [ ] 物品图标显示
- [ ] 物品数量显示
- [ ] 物品拖拽
- [ ] 物品右键菜单（使用/丢弃/拆分等）
- [ ] 物品详情tooltip
- [ ] 金币显示
- [ ] 关闭按钮
- [ ] 整理按钮
- [ ] 重量显示

**纹理资源**:
- 待查询原工程

---

### 4. CharacterDialog - 角色/技能面板
**文件**: `src/scenes/game/dialogs/character_dialog.rs`

**功能清单**:
- [ ] 窗口背景
- [ ] 标签页切换（角色/技能）
- [ ] **角色页面**:
  - [ ] 角色3D模型/纸娃娃显示
  - [ ] 装备栏（武器/头盔/项链/戒指等）
  - [ ] 装备拖拽
  - [ ] 属性显示（攻击/防御/魔法等）
  - [ ] 详细属性列表
- [ ] **技能页面**:
  - [ ] 技能列表
  - [ ] 技能图标
  - [ ] 技能等级显示
  - [ ] 技能拖拽到快捷栏
  - [ ] 技能升级按钮
  - [ ] 技能说明tooltip
- [ ] 关闭按钮

**纹理资源**:
- 待查询原工程

---

## 🎯 阶段二：常用功能（优先级：⭐⭐⭐⭐）

### 5. MiniMapDialog - 小地图
**文件**: `src/scenes/game/dialogs/minimap_dialog.rs`

**功能清单**:
- [ ] 小地图窗口
- [ ] 地图绘制
- [ ] 玩家位置标记
- [ ] NPC位置标记
- [ ] 怪物位置标记
- [ ] 组队成员标记
- [ ] 地图缩放
- [ ] 大地图切换
- [ ] 透明度调节

---

### 6. MenuDialog - 游戏菜单
**文件**: `src/scenes/game/dialogs/menu_dialog.rs`

**功能清单**:
- [ ] 菜单窗口
- [ ] 退出游戏
- [ ] 返回角色选择
- [ ] 游戏设置
- [ ] 帮助文档
- [ ] 关于信息

---

### 7. OptionDialog - 游戏设置
**文件**: `src/scenes/game/dialogs/option_dialog.rs`

**功能清单**:
- [ ] 设置窗口
- [ ] 图形设置（分辨率/特效等）
- [ ] 音效设置（音量/开关）
- [ ] 游戏设置（攻击模式/显示选项等）
- [ ] 键位设置
- [ ] 保存/取消按钮

---

### 8. BeltDialog - 快捷栏
**文件**: `src/scenes/game/dialogs/belt_dialog.rs`

**功能清单**:
- [ ] 快捷栏背景
- [ ] 6个快捷格子
- [ ] 技能/物品图标显示
- [ ] 快捷键显示（F1-F6）
- [ ] 拖拽绑定
- [ ] 快捷键触发
- [ ] 冷却时间显示

---

## 🎯 阶段三：NPC交互（优先级：⭐⭐⭐）

### 9. NPCDialog - NPC对话
**文件**: `src/scenes/game/dialogs/npc_dialog.rs`

**功能清单**:
- [ ] NPC头像
- [ ] 对话文本显示
- [ ] 选项按钮列表
- [ ] 上一页/下一页
- [ ] 关闭按钮

---

### 10. NPCGoodsDialog - NPC商店
**文件**: `src/scenes/game/dialogs/npc_goods_dialog.rs`

**功能清单**:
- [ ] 商店窗口
- [ ] 商品列表
- [ ] 商品图标/名称/价格
- [ ] 购买/出售切换
- [ ] 数量选择
- [ ] 金币显示
- [ ] 确认/取消按钮

---

### 11. 其他NPC对话框
- [ ] NPCDropDialog - 物品回收
- [ ] NPCAwakeDialog - 物品觉醒
- [ ] CraftDialog - 物品制作
- [ ] RefineDialog - 物品精炼
- [ ] SocketDialog - 宝石镶嵌

---

## 🎯 阶段四：社交功能（优先级：⭐⭐⭐）

### 12. TradeDialog - 交易
**文件**: `src/scenes/game/dialogs/trade_dialog.rs`

**功能清单**:
- [ ] 交易窗口
- [ ] 自己物品栏
- [ ] 对方物品栏
- [ ] 金币输入
- [ ] 锁定/确认按钮

---

### 13. GroupDialog - 组队
**文件**: `src/scenes/game/dialogs/group_dialog.rs`

**功能清单**:
- [ ] 组队窗口
- [ ] 队员列表
- [ ] 队员血量/等级显示
- [ ] 队长标记
- [ ] 离队/踢人按钮
- [ ] 队伍设置

---

### 14. GuildDialog - 公会
**文件**: `src/scenes/game/dialogs/guild_dialog.rs`

**功能清单**:
- [ ] 公会窗口
- [ ] 公会信息
- [ ] 成员列表
- [ ] 公会仓库
- [ ] 公会设置
- [ ] 职位管理

---

### 15. FriendDialog - 好友
**文件**: `src/scenes/game/dialogs/friend_dialog.rs`

**功能清单**:
- [ ] 好友窗口
- [ ] 好友列表
- [ ] 在线状态
- [ ] 添加好友
- [ ] 删除好友
- [ ] 私聊快捷方式

---

## 🎯 阶段五：高级功能（优先级：⭐⭐）

### 16. QuestDialog 系列 - 任务系统
- [ ] QuestListDialog - 任务列表
- [ ] QuestDetailDialog - 任务详情
- [ ] QuestDiaryDialog - 任务日志
- [ ] QuestTrackingDialog - 任务追踪

---

### 17. MailDialog 系列 - 邮件系统
- [ ] MailListDialog - 邮件列表
- [ ] MailReadLetterDialog - 读信
- [ ] MailReadParcelDialog - 读包裹
- [ ] MailComposeLetterDialog - 写信
- [ ] MailComposeParcelDialog - 写包裹

---

### 18. 英雄系统
- [ ] NewHeroDialog - 创建英雄
- [ ] HeroDialog - 英雄属性
- [ ] HeroInventoryDialog - 英雄背包
- [ ] HeroBeltDialog - 英雄快捷栏
- [ ] HeroManageDialog - 英雄管理

---

### 19. 特殊功能
- [ ] StorageDialog - 仓库
- [ ] MountDialog - 坐骑
- [ ] FishingDialog - 钓鱼
- [ ] IntelligentCreatureDialog - 守护
- [ ] RankingDialog - 排行榜
- [ ] GameShopDialog - 游戏商城
- [ ] TrustMerchantDialog - 寄售商店
- [ ] BigMapDialog - 大地图
- [ ] InspectDialog - 查看玩家
- [ ] HelpDialog - 帮助

---

## 🎯 阶段六：UI增强功能（优先级：⭐）

### 20. 通用UI组件
**文件**: `src/scenes/game/ui/mod.rs`

**组件清单**:
- [ ] ItemCell - 物品格子组件
- [ ] ItemTooltip - 物品tooltip
- [ ] ProgressBar - 进度条组件
- [ ] TabControl - 标签页控件
- [ ] ScrollView - 滚动视图
- [ ] InputBox - 输入框
- [ ] Button - 按钮组件
- [ ] Label - 文本标签
- [ ] ImageControl - 图片控件

---

### 21. 特效系统
- [ ] 技能冷却动画
- [ ] 物品闪烁特效
- [ ] 按钮悬停特效
- [ ] 窗口淡入淡出
- [ ] 伤害数字飘字

---

## 📐 技术架构

### 目录结构
```
src/scenes/game/
├── mod.rs                  # GameScene主模块
├── dialogs/                # 对话框模块
│   ├── mod.rs
│   ├── main_dialog.rs
│   ├── chat_dialog.rs
│   ├── inventory_dialog.rs
│   ├── character_dialog.rs
│   ├── minimap_dialog.rs
│   ├── menu_dialog.rs
│   ├── option_dialog.rs
│   ├── belt_dialog.rs
│   ├── npc/                # NPC相关对话框
│   │   ├── mod.rs
│   │   ├── npc_dialog.rs
│   │   ├── npc_goods_dialog.rs
│   │   └── ...
│   ├── social/             # 社交功能对话框
│   │   ├── mod.rs
│   │   ├── trade_dialog.rs
│   │   ├── group_dialog.rs
│   │   ├── guild_dialog.rs
│   │   └── friend_dialog.rs
│   └── ...
├── ui/                     # 通用UI组件
│   ├── mod.rs
│   ├── item_cell.rs
│   ├── progress_bar.rs
│   └── ...
└── state.rs               # GameScene状态管理
```

### Dialog Trait 扩展
```rust
pub trait GameDialog {
    fn show(&mut self, ctx: &egui::Context, state: &mut GameState);
    fn update(&mut self, dt: f32, state: &mut GameState);
    fn handle_input(&mut self, state: &mut GameState) -> bool;
    fn is_visible(&self) -> bool;
    fn set_visible(&mut self, visible: bool);
}
```

### GameState 结构
```rust
pub struct GameState {
    // 玩家数据
    pub player: PlayerData,
    
    // 对话框状态
    pub dialogs: DialogManager,
    
    // UI状态
    pub ui_state: UIState,
    
    // 网络通信
    pub network: NetworkClient,
}
```

---

## 🔧 开发规范

### 1. 命名规范
- 对话框：`XxxDialog`
- UI组件：`XxxControl` 或 `XxxWidget`
- 事件：`XxxEvent`
- 状态：`XxxState`

### 2. 代码组织
- 每个对话框独立文件
- 相关对话框归类到子目录
- 通用组件抽取到 ui 模块
- 保持与原工程的功能对应

### 3. 渲染顺序
1. 游戏世界（ECS系统渲染）
2. MainDialog（最底层UI）
3. 其他对话框（按Z-order）
4. Tooltip和浮动提示（最上层）

### 4. 事件处理
- 从上到下检测鼠标事件
- 对话框捕获事件后阻止穿透
- 快捷键全局监听
- 拖拽状态管理

---

## 📝 实现检查清单

### 每个对话框完成时需确认：
- [ ] 基本显示正常
- [ ] 可拖动（如果需要）
- [ ] 关闭按钮工作
- [ ] 快捷键响应
- [ ] 数据绑定正确
- [ ] 事件处理完整
- [ ] 与其他对话框交互正常
- [ ] 编译无警告
- [ ] 性能无明显问题

---

## 🎮 测试计划

### 单元测试
- [ ] 每个对话框创建测试bin
- [ ] UI组件独立测试

### 集成测试
- [ ] 对话框组合显示测试
- [ ] 拖拽交互测试
- [ ] 快捷键冲突测试

### 性能测试
- [ ] 多对话框同时显示
- [ ] 长时间运行内存测试
- [ ] 帧率稳定性测试

---

## 📅 里程碑

### Milestone 1: 核心UI框架 (估计：2-3周)
完成 MainDialog, ChatDialog, InventoryDialog, CharacterDialog

### Milestone 2: 常用功能 (估计：1-2周)
完成 MiniMap, Menu, Option, Belt

### Milestone 3: NPC交互 (估计：1-2周)
完成所有NPC相关对话框

### Milestone 4: 社交功能 (估计：1-2周)
完成交易、组队、公会、好友

### Milestone 5: 高级功能 (估计：2-3周)
完成任务、邮件、英雄、特殊功能

### Milestone 6: 优化完善 (估计：1周)
性能优化、bug修复、体验优化

---

## 📚 参考资料

- 原工程路径：`Client/MirScenes/Dialogs/`
- 纹理资源：`Data/Prguse.lib`, `Data/Title.lib` 等
- 网络协议：`Server/MirNetwork/` 和 `Client/MirNetwork/`

---

## 🔄 更新日志

- 2025-11-16: 创建 TODO 文档，规划整体架构
- 2025-11-16: ✅ 完成 MainDialog 基础版本
  - 实现底部工具栏背景自适应
  - 实现生命值/魔法值显示（文字版）
  - 实现经验条和负重条
  - 实现7个功能按钮（背包/角色/技能/任务/选项/菜单/商城）
  - 实现按钮悬停提示和点击事件
  - 创建测试程序 `test_main_dialog`
  - 文件：`src/scenes/dialogs/game/main_dialog.rs` (约370行)
  - 实现按钮悬停提示和点击事件
  - 创建测试程序 `test_main_dialog`
