# ecs/ui - 游戏UI系统

**文件数**: 22+  
**代码行数**: ~4,500  
**状态**: ✅ 核心完成

---

## 📚 目录

1. [模块概述](#-模块概述)
2. [UI架构](#-ui架构)
3. [对话框系统](#-对话框系统)
4. [UI组件](#-ui组件)
5. [使用指南](#-使用指南)

---

## 📖 模块概述

`ui` 目录包含游戏的所有UI组件和对话框实现。

### 设计原则

1. **ECS集成**: UI作为ECS系统的一部分
2. **层级管理**: 对话框层级和焦点管理
3. **事件驱动**: 通过事件系统交互
4. **可复用**: 通用UI组件可复用

### 文件结构

```
ui/
├── mod.rs                      # UI模块入口
├── components.rs               # 基础UI组件（HUD等）
├── button_widget.rs            # 按钮组件
├── dialog_manager.rs           # 对话框管理器
├── hotkey_help.rs              # 快捷键帮助
└── dialogs/                    # 对话框集合
    ├── mod.rs
    ├── main_dialog.rs          # 主界面（血条/MP条/经验条）
    ├── inventory_dialog.rs     # 背包对话框
    ├── character_dialog.rs     # 角色属性对话框
    ├── skills_dialog.rs        # 技能对话框
    ├── skillbar_dialog.rs      # 技能栏
    ├── magic_learning_dialog.rs # 魔法学习
    ├── quest_dialog.rs         # 任务对话框
    ├── chat_dialog.rs          # 聊天对话框
    ├── minimap_dialog.rs       # 小地图
    ├── trade_dialog.rs         # 交易对话框
    ├── friends_dialog.rs       # 好友对话框
    ├── group_dialog.rs         # 组队对话框
    ├── guild_dialog.rs         # 公会对话框
    ├── buff_dialog.rs          # Buff显示
    └── options_dialog.rs       # 选项对话框
```

---

## 🏗 UI架构

### DialogManager - 对话框管理器

**职责**: 管理所有对话框的显示、隐藏、层级

```rust
pub struct DialogManager {
    /// 所有对话框
    dialogs: HashMap<DialogType, Box<dyn Dialog>>,
    
    /// 对话框显示顺序（Z-order）
    display_order: Vec<DialogType>,
    
    /// 当前焦点对话框
    focused_dialog: Option<DialogType>,
}

pub enum DialogType {
    Main,           // 主界面
    Inventory,      // 背包
    Character,      // 角色
    Skills,         // 技能
    SkillBar,       // 技能栏
    Quest,          // 任务
    Chat,           // 聊天
    MiniMap,        // 小地图
    Trade,          // 交易
    Friends,        // 好友
    Group,          // 组队
    Guild,          // 公会
    Buff,           // Buff
    Options,        // 选项
}
```

#### 主要方法

```rust
impl DialogManager {
    pub fn new() -> Self;
    
    /// 显示对话框
    pub fn show(&mut self, dialog_type: DialogType);
    
    /// 隐藏对话框
    pub fn hide(&mut self, dialog_type: DialogType);
    
    /// 切换对话框显示状态
    pub fn toggle(&mut self, dialog_type: DialogType);
    
    /// 是否显示
    pub fn is_visible(&self, dialog_type: DialogType) -> bool;
    
    /// 设置焦点
    pub fn set_focus(&mut self, dialog_type: DialogType);
    
    /// 更新所有对话框
    pub fn update(&mut self, ctx: &mut Context, world: &World);
    
    /// 绘制所有对话框
    pub fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas, world: &World);
    
    /// 处理鼠标事件
    pub fn handle_mouse_down(&mut self, x: f32, y: f32) -> bool;
    pub fn handle_mouse_up(&mut self, x: f32, y: f32) -> bool;
    pub fn handle_mouse_move(&mut self, x: f32, y: f32) -> bool;
    
    /// 处理键盘事件
    pub fn handle_key_down(&mut self, key: KeyCode) -> bool;
}
```

### Dialog Trait

所有对话框实现 `Dialog` trait：

```rust
pub trait Dialog {
    /// 获取对话框类型
    fn dialog_type(&self) -> DialogType;
    
    /// 更新
    fn update(&mut self, ctx: &mut Context, world: &World);
    
    /// 绘制
    fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas, world: &World);
    
    /// 是否可见
    fn is_visible(&self) -> bool;
    
    /// 设置可见性
    fn set_visible(&mut self, visible: bool);
    
    /// 获取边界
    fn bounds(&self) -> Rect;
    
    /// 鼠标事件
    fn on_mouse_down(&mut self, x: f32, y: f32) -> bool;
    fn on_mouse_up(&mut self, x: f32, y: f32) -> bool;
    fn on_mouse_move(&mut self, x: f32, y: f32) -> bool;
    
    /// 键盘事件
    fn on_key_down(&mut self, key: KeyCode) -> bool;
}
```

---

## 📦 对话框系统

### 1. MainDialog - 主界面

**职责**: 显示玩家状态（HP/MP/经验）

```rust
pub struct MainDialog {
    visible: bool,
    position: Point,
    
    /// 生命值条
    health_bar: HealthBar,
    
    /// 魔法值条
    mana_bar: ManaBar,
    
    /// 经验值条
    exp_bar: ExperienceBar,
    
    /// 等级显示
    level_display: LevelDisplay,
}
```

**显示内容**:
- ✅ HP条（红色）
- ✅ MP条（蓝色）
- ✅ 经验条（黄色）
- ✅ 等级数字
- ✅ 角色名称

**位置**: 屏幕顶部

### 2. InventoryDialog - 背包对话框

**职责**: 管理背包物品

```rust
pub struct InventoryDialog {
    visible: bool,
    position: Point,
    
    /// 背包格子 (8x5 = 40格)
    grid: ItemGrid,
    
    /// 金币显示
    gold_display: GoldDisplay,
    
    /// 当前拖拽的物品
    dragging_item: Option<DraggedItem>,
}
```

**功能**:
- ✅ 显示背包物品
- ✅ 物品拖拽
- ✅ 物品使用
- ✅ 物品丢弃
- ✅ 物品整理
- ✅ 金币显示

**快捷键**: `I` 或 `B`

### 3. CharacterDialog - 角色属性对话框

**职责**: 显示角色详细属性

```rust
pub struct CharacterDialog {
    visible: bool,
    position: Point,
    
    /// 装备栏
    equipment_slots: EquipmentSlots,
    
    /// 属性显示
    stats_display: StatsDisplay,
}

pub struct EquipmentSlots {
    pub weapon: EquipmentSlot,
    pub armor: EquipmentSlot,
    pub helmet: EquipmentSlot,
    pub necklace: EquipmentSlot,
    pub bracelet_l: EquipmentSlot,
    pub bracelet_r: EquipmentSlot,
    pub ring_l: EquipmentSlot,
    pub ring_r: EquipmentSlot,
    pub boots: EquipmentSlot,
    pub belt: EquipmentSlot,
}
```

**显示内容**:
- ✅ 装备栏（10个槽位）
- ✅ 基础属性（HP/MP/攻击/防御）
- ✅ 等级和经验
- ✅ 装备耐久度

**快捷键**: `C`

### 4. SkillsDialog - 技能对话框

**职责**: 显示和管理技能

```rust
pub struct SkillsDialog {
    visible: bool,
    position: Point,
    
    /// 技能列表
    skills: Vec<SkillSlot>,
    
    /// 技能描述
    description_panel: DescriptionPanel,
}

pub struct SkillSlot {
    pub spell: Spell,
    pub level: u8,
    pub experience: u16,
    pub key_binding: Option<KeyCode>,
}
```

**功能**:
- ✅ 显示已学技能
- ✅ 技能等级
- ✅ 技能经验
- ✅ 技能绑定
- ✅ 技能描述
- 🚧 技能升级

**快捷键**: `S` 或 `K`

### 5. SkillBarDialog - 技能栏

**职责**: 快捷技能栏

```rust
pub struct SkillBarDialog {
    visible: bool,
    position: Point,
    
    /// 技能槽位 (F1-F8)
    slots: [Option<SkillBarSlot>; 8],
}
```

**功能**:
- ✅ 8个快捷栏槽位
- ✅ 技能冷却显示
- ✅ 技能快捷键（F1-F8）
- ✅ 拖拽设置技能

**位置**: 屏幕底部

### 6. QuestDialog - 任务对话框

**职责**: 显示任务列表和进度

```rust
pub struct QuestDialog {
    visible: bool,
    position: Point,
    
    /// 任务列表
    quests: Vec<QuestEntry>,
    
    /// 选中的任务
    selected_quest: Option<usize>,
    
    /// 任务详情面板
    details_panel: QuestDetailsPanel,
}

pub struct QuestEntry {
    pub quest_id: i32,
    pub title: String,
    pub progress: f32,
    pub status: QuestStatus,
}
```

**功能**:
- ✅ 显示任务列表
- ✅ 任务进度
- ✅ 任务描述
- ✅ 任务目标
- ✅ 任务奖励
- 🚧 任务追踪
- 🚧 任务放弃

**快捷键**: `Q`

### 7. ChatDialog - 聊天对话框

**职责**: 聊天消息显示和输入

```rust
pub struct ChatDialog {
    visible: bool,
    position: Point,
    
    /// 聊天消息列表
    messages: Vec<ChatMessage>,
    
    /// 输入框
    input_box: TextInput,
    
    /// 聊天频道
    current_channel: ChatChannel,
}

pub struct ChatMessage {
    pub sender: String,
    pub message: String,
    pub channel: ChatChannel,
    pub timestamp: Instant,
}

pub enum ChatChannel {
    All,        // 综合
    World,      // 世界
    Guild,      // 公会
    Group,      // 组队
    Private,    // 私聊
    System,     // 系统
}
```

**功能**:
- ✅ 显示聊天消息
- ✅ 发送消息
- ✅ 频道切换
- ✅ 消息历史
- ✅ 表情支持
- 🚧 消息过滤
- 🚧 私聊窗口

**位置**: 屏幕左下角

### 8. MiniMapDialog - 小地图

**职责**: 显示小地图

```rust
pub struct MiniMapDialog {
    visible: bool,
    position: Point,
    
    /// 地图纹理
    map_texture: Option<Image>,
    
    /// 玩家位置
    player_position: Point,
    
    /// 地图缩放
    zoom: f32,
}
```

**功能**:
- ✅ 显示地图轮廓
- ✅ 玩家位置标记
- ✅ 队友位置
- 🚧 NPC标记
- 🚧 怪物标记
- 🚧 地图缩放

**快捷键**: `Tab` 或 `M`

### 9. TradeDialog - 交易对话框

**职责**: 玩家间交易

```rust
pub struct TradeDialog {
    visible: bool,
    position: Point,
    
    /// 自己的交易槽
    my_slots: [Option<TradeSlot>; 10],
    
    /// 对方的交易槽
    their_slots: [Option<TradeSlot>; 10],
    
    /// 金币数量
    my_gold: u64,
    their_gold: u64,
    
    /// 确认状态
    my_confirmed: bool,
    their_confirmed: bool,
}
```

**功能**:
- ✅ 物品放置
- ✅ 金币交易
- ✅ 确认机制
- ✅ 取消交易
- 🚧 交易记录

### 10. FriendsDialog - 好友对话框

**职责**: 好友列表管理

```rust
pub struct FriendsDialog {
    visible: bool,
    position: Point,
    
    /// 好友列表
    friends: Vec<FriendEntry>,
    
    /// 黑名单
    blocked: Vec<String>,
}

pub struct FriendEntry {
    pub name: String,
    pub online: bool,
    pub level: u16,
    pub location: String,
}
```

**功能**:
- ✅ 好友列表
- ✅ 在线状态
- ✅ 添加好友
- ✅ 删除好友
- 🚧 好友分组
- 🚧 好友备注

### 11. GroupDialog - 组队对话框

**职责**: 组队信息显示

```rust
pub struct GroupDialog {
    visible: bool,
    position: Point,
    
    /// 队伍成员
    members: Vec<GroupMember>,
}

pub struct GroupMember {
    pub name: String,
    pub hp_percent: f32,
    pub mp_percent: f32,
    pub level: u16,
}
```

**功能**:
- ✅ 队员列表
- ✅ HP/MP显示
- ✅ 队长标识
- ✅ 离队按钮
- 🚧 队员位置

### 12. GuildDialog - 公会对话框

**职责**: 公会信息管理

```rust
pub struct GuildDialog {
    visible: bool,
    position: Point,
    
    /// 公会成员
    members: Vec<GuildMember>,
    
    /// 公会公告
    notice: String,
}
```

**功能**:
- ✅ 成员列表
- ✅ 在线状态
- ✅ 公告显示
- 🚧 权限管理
- 🚧 公会仓库
- 🚧 公会技能

### 13. BuffDialog - Buff显示

**职责**: 显示增益/减益效果

```rust
pub struct BuffDialog {
    visible: bool,
    position: Point,
    
    /// Buff列表
    buffs: Vec<BuffDisplay>,
}

pub struct BuffDisplay {
    pub buff_type: BuffType,
    pub icon: u32,
    pub remaining_time: Duration,
}
```

**功能**:
- ✅ Buff图标显示
- ✅ 剩余时间
- ✅ Buff描述
- ✅ 可取消的Buff

**位置**: 屏幕右上角

### 14. OptionsDialog - 选项对话框

**职责**: 游戏设置

```rust
pub struct OptionsDialog {
    visible: bool,
    position: Point,
    
    /// 设置选项
    options: GameOptions,
}

pub struct GameOptions {
    pub music_volume: f32,
    pub sound_volume: f32,
    pub show_damage: bool,
    pub show_hp_bar: bool,
    pub show_names: bool,
    // ... 更多选项
}
```

**功能**:
- ✅ 音量设置
- ✅ 显示设置
- ✅ 快捷键设置
- 🚧 画质设置
- 🚧 界面设置

---

## 🔧 UI组件

### ButtonWidget - 按钮组件

```rust
pub struct ButtonWidget {
    /// 位置和大小
    pub bounds: Rect,
    
    /// 按钮状态
    pub state: ButtonState,
    
    /// 图像索引
    pub normal_image: u32,
    pub hover_image: u32,
    pub pressed_image: u32,
    
    /// 是否启用
    pub enabled: bool,
}

pub enum ButtonState {
    Normal,
    Hover,
    Pressed,
    Disabled,
}

impl ButtonWidget {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self;
    
    pub fn contains(&self, x: f32, y: f32) -> bool;
    
    pub fn update(&mut self, mouse_pos: Point);
    
    pub fn draw(&self, ctx: &mut Context, canvas: &mut Canvas);
    
    pub fn is_clicked(&self) -> bool;
}
```

### HealthBar - 生命值条

```rust
pub struct HealthBar {
    pub position: Point,
    pub width: f32,
    pub height: f32,
    pub current: u32,
    pub maximum: u32,
}

impl HealthBar {
    pub fn percentage(&self) -> f32 {
        self.current as f32 / self.maximum as f32
    }
    
    pub fn draw(&self, ctx: &mut Context, canvas: &mut Canvas) {
        // 绘制背景
        let bg_rect = Rect::new(self.position.x, self.position.y, self.width, self.height);
        draw_rect(canvas, bg_rect, Color::BLACK);
        
        // 绘制HP条
        let hp_width = self.width * self.percentage();
        let hp_rect = Rect::new(self.position.x, self.position.y, hp_width, self.height);
        draw_rect(canvas, hp_rect, Color::RED);
        
        // 绘制文字
        let text = format!("{}/{}", self.current, self.maximum);
        draw_text(canvas, &text, self.position, Color::WHITE);
    }
}
```

### ItemGrid - 物品网格

```rust
pub struct ItemGrid {
    pub position: Point,
    pub rows: usize,
    pub cols: usize,
    pub cell_size: f32,
    pub items: Vec<Option<UserItem>>,
}

impl ItemGrid {
    pub fn new(x: f32, y: f32, rows: usize, cols: usize) -> Self;
    
    /// 获取格子索引
    pub fn get_cell_index(&self, x: f32, y: f32) -> Option<usize>;
    
    /// 获取格子位置
    pub fn get_cell_position(&self, index: usize) -> Point;
    
    /// 绘制网格
    pub fn draw(&self, ctx: &mut Context, canvas: &mut Canvas);
}
```

---

## 📖 使用指南

### 创建对话框管理器

```rust
use crate::ecs::ui::*;

// 创建对话框管理器
let mut dialog_manager = DialogManager::new();

// 显示主界面
dialog_manager.show(DialogType::Main);

// 显示技能栏
dialog_manager.show(DialogType::SkillBar);
```

### 切换对话框

```rust
// 按 I 键切换背包
if key == KeyCode::I {
    dialog_manager.toggle(DialogType::Inventory);
}

// 按 C 键切换角色
if key == KeyCode::C {
    dialog_manager.toggle(DialogType::Character);
}

// 按 S 键切换技能
if key == KeyCode::S {
    dialog_manager.toggle(DialogType::Skills);
}
```

### 更新和绘制

```rust
// 游戏循环
fn update(&mut self, ctx: &mut Context) {
    // 更新UI
    self.dialog_manager.update(ctx, &self.world);
}

fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas) {
    // 绘制UI（最后绘制，覆盖在游戏画面上）
    self.dialog_manager.draw(ctx, canvas, &self.world);
}
```

### 处理输入

```rust
fn on_mouse_down(&mut self, x: f32, y: f32) {
    // UI优先处理点击
    if self.dialog_manager.handle_mouse_down(x, y) {
        return; // UI消费了事件
    }
    
    // 否则处理游戏世界交互
    self.handle_world_click(x, y);
}
```

### 自定义对话框

```rust
pub struct CustomDialog {
    visible: bool,
    position: Point,
    // ... 自定义数据
}

impl Dialog for CustomDialog {
    fn dialog_type(&self) -> DialogType {
        DialogType::Custom
    }
    
    fn update(&mut self, ctx: &mut Context, world: &World) {
        // 更新逻辑
    }
    
    fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas, world: &World) {
        if !self.visible {
            return;
        }
        
        // 绘制对话框背景
        self.draw_background(ctx, canvas);
        
        // 绘制内容
        // ...
    }
    
    fn is_visible(&self) -> bool {
        self.visible
    }
    
    fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }
    
    fn bounds(&self) -> Rect {
        Rect::new(self.position.x, self.position.y, 400.0, 300.0)
    }
    
    fn on_mouse_down(&mut self, x: f32, y: f32) -> bool {
        if !self.visible || !self.bounds().contains([x, y]) {
            return false;
        }
        
        // 处理点击
        true
    }
    
    // 实现其他方法...
}
```

---

## 📊 开发状态

### 完成度统计

| 对话框 | 完成度 | 说明 |
|--------|--------|------|
| **MainDialog** | 100% | 完成 |
| **InventoryDialog** | 95% | 核心功能完成 |
| **CharacterDialog** | 90% | 主要功能完成 |
| **SkillsDialog** | 85% | 基础功能完成 |
| **SkillBarDialog** | 100% | 完成 |
| **QuestDialog** | 80% | 显示完成，追踪待实现 |
| **ChatDialog** | 90% | 核心功能完成 |
| **MiniMapDialog** | 70% | 基础显示完成 |
| **TradeDialog** | 80% | 基础功能完成 |
| **FriendsDialog** | 75% | 主要功能完成 |
| **GroupDialog** | 85% | 核心功能完成 |
| **GuildDialog** | 70% | 基础功能完成 |
| **BuffDialog** | 95% | 核心功能完成 |
| **OptionsDialog** | 80% | 主要设置完成 |

### 已实现功能

- [x] 对话框管理器
- [x] 对话框层级管理
- [x] 焦点管理
- [x] 拖拽系统
- [x] 按钮组件
- [x] 输入框组件
- [x] 生命值/魔法值条
- [x] 物品网格
- [x] 技能栏
- [x] 聊天系统

### 未实现功能

- [ ] 完整的拖拽反馈
- [ ] 对话框动画
- [ ] UI皮肤系统
- [ ] 自定义UI布局
- [ ] 更多UI组件（滑块、复选框等）

---

## 🔗 相关文档

- **ECS系统**: `../systems/README.md` - UI系统详解
- **组件定义**: `../components/README.md` - UI相关组件
- **场景系统**: `../scenes/README.md` - 场景如何使用UI

---

**文档版本**: v1.0  
**最后更新**: 2025-10-28  
**维护者**: Crystal Mir2 Team
