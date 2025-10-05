# MirScenes 完整模块移植详细计划

## 文件统计总览

### 主场景文件 (3个)
| 文件名 | 行数 | 复杂度 | 状态 |
|--------|------|--------|------|
| GameScene.cs | 10,720 | 极高 | ⚠️ 部分完成 |
| LoginScene.cs | 1,233 | 高 | ✅ 已完成对话框 |
| SelectScene.cs | 556 | 中 | ⚠️ 需要 CharacterButton |

### 对话框文件 (36个)
| 文件名 | 行数 | 内部类数 | 复杂度 | 依赖层级 |
|--------|------|---------|--------|----------|
| MainDialogs.cs | 3,715 | 10 | 极高 | L1 基础 |
| NPCDialogs.cs | 2,338 | 4 | 高 | L2 |
| GuildDialog.cs | 2,032 | 2 | 高 | L3 |
| QuestDialogs.cs | 1,550 | 5 | 高 | L2 |
| TrustMerchantDialog.cs | 1,351 | 1 | 中 | L2 |
| IntelligentCreatureDialogs.cs | 1,234 | 4 | 高 | L3 |
| MailDialogs.cs | 1,082 | 6 | 中 | L2 |
| HeroDialogs.cs | 796 | 4 | 中 | L2 |
| BuffDialog.cs | 763 | 1 | 低 | L1 |
| BigMapDialog.cs | 720 | 1 | 中 | L1 |
| GameshopDialog.cs | 666 | 1 | 中 | L2 |
| CharacterDialog.cs | 633 | 1 | 中 | L1 |
| FriendDialog.cs | 484 | 3 | 中 | L2 |
| InventoryDialog.cs | 432 | 2 | 中 | L1 |
| RankingDialog.cs | 396 | 1 | 低 | L2 |
| KeyboardLayoutDialog.cs | 358 | 3 | 低 | L1 |
| HelpDialog.cs | 347 | 1 | 低 | L1 |
| ChatOptionDialog.cs | 340 | 1 | 低 | L1 |
| FishingDialog.cs | 330 | 2 | 中 | L2 |
| NewCharacterDialog.cs | 318 | 1 | 中 | L1 |
| GuildTerritoryDialog.cs | 315 | 1 | 中 | L3 |
| NoticeDialog.cs | 304 | 1 | 低 | L1 |
| ItemRentingDialog.cs | 284 | 2 | 中 | L2 |
| MentorDialog.cs | 272 | 1 | 低 | L2 |
| ItemRentDialog.cs | 242 | 2 | 中 | L2 |
| TradeDialogs.cs | 239 | 2 | 中 | L2 |
| MountDialog.cs | 229 | 1 | 低 | L2 |
| RelationshipDialog.cs | 220 | 1 | 低 | L2 |
| TimerDialog.cs | 209 | 2 | 低 | L1 |
| GroupDialog.cs | 197 | 1 | 低 | L2 |
| RollDialog.cs | 171 | 1 | 低 | L1 |
| ItemRentalDialog.cs | 164 | 1 | 低 | L2 |
| SocketDialog.cs | 103 | 1 | 低 | L1 |
| ChatNoticeDialog.cs | 70 | 1 | 低 | L1 |
| ReportDialog.cs | 66 | 1 | 低 | L1 |
| CompassDialog.cs | 52 | 1 | 低 | L1 |

**总计**: 39 个文件，约 **32,000+ 行代码**

## 内部类结构分析

### MainDialogs.cs (10个类)
```
MainDialog                    // 主对话框
ChatDialog                    // 聊天对话框
  └─ ChatHistory             // 聊天历史记录
ChatControlBar                // 聊天控制栏
SkillBarDialog                // 技能栏对话框
MiniMapDialog                 // 小地图对话框
InspectDialog                 // 检查对话框
OptionDialog                  // 选项对话框
MenuDialog                    // 菜单对话框
MagicButton                   // 魔法按钮
AssignKeyPanel                // 分配按键面板
DuraStatusDialog              // 耐久度状态对话框
CharacterDuraPanel            // 角色耐久度面板
```

### NPCDialogs.cs (4个类)
```
NPCDialog                     // NPC对话框
NPCGoodsDialog                // NPC商品对话框
NPCDropDialog                 // NPC掉落对话框
NPCAwakeDialog                // NPC觉醒对话框
```

### MailDialogs.cs (6个类)
```
MailListDialog                // 邮件列表对话框
MailItemRow                   // 邮件项行
MailComposeLetterDialog       // 撰写信件对话框
MailComposeParcelDialog       // 撰写包裹对话框
MailReadLetterDialog          // 阅读信件对话框
MailReadParcelDialog          // 阅读包裹对话框
```

### QuestDialogs.cs (5个类)
```
QuestListDialog               // 任务列表对话框
QuestDetailDialog             // 任务详情对话框
QuestDiaryDialog              // 任务日记对话框
QuestTrackingDialog           // 任务追踪对话框
QuestProgressBar              // 任务进度条
```

### HeroDialogs.cs (4个类)
```
HeroBeltDialog                // 英雄腰带对话框
HeroInventoryDialog           // 英雄物品栏对话框
HeroManageDialog              // 英雄管理对话框
HeroBehaviourPanel            // 英雄行为面板
```

### IntelligentCreatureDialogs.cs (4个类)
```
IntelligentCreatureDialog     // 智能生物对话框
IntelligentCreatureOptionsDialog           // 选项
IntelligentCreatureOptionsGradeDialog      // 等级选项
IntelligentCreatureRuleDialog              // 规则
```

## 依赖关系分析

### Layer 1: 基础对话框（无或少依赖）
**优先级: 最高** - 这些可以独立实现

```
CompassDialog.cs (52行)       ← 仅依赖 MirControls
ReportDialog.cs (66行)         ← 仅依赖 MirControls
ChatNoticeDialog.cs (70行)     ← 仅依赖 MirControls
SocketDialog.cs (103行)        ← 仅依赖 MirControls
RollDialog.cs (171行)          ← 依赖 Network
TimerDialog.cs (209行)         ← 依赖 ServerPackets
GroupDialog.cs (197行)         ← 依赖 Network
NoticeDialog.cs (304行)        ← 仅依赖 MirControls
NewCharacterDialog.cs (318行)  ← 依赖 Network
KeyboardLayoutDialog.cs (358行) ← 仅依赖 MirControls
HelpDialog.cs (347行)          ← 仅依赖 MirControls
ChatOptionDialog.cs (340行)    ← 仅依赖 MirControls
BuffDialog.cs (763行)          ← 仅依赖 MirControls
BigMapDialog.cs (720行)        ← 仅依赖 MirControls
CharacterDialog.cs (633行)     ← 依赖 Network + Objects
InventoryDialog.cs (432行)     ← 依赖 Network + Objects
```

### Layer 2: 中级对话框（依赖 L1 + Network/Objects）
**优先级: 中**

```
TradeDialogs.cs (239行)        ← 依赖 Network
ItemRentalDialog.cs (164行)    ← 依赖 Network
RelationshipDialog.cs (220行)  ← 依赖 Network
MountDialog.cs (229行)         ← 依赖 Network
ItemRentDialog.cs (242行)      ← 依赖 Network
MentorDialog.cs (272行)        ← 依赖 Network
ItemRentingDialog.cs (284行)   ← 依赖 Network
FishingDialog.cs (330行)       ← 依赖 Network + Objects
FriendDialog.cs (484行)        ← 依赖 Network
RankingDialog.cs (396行)       ← 依赖 Network
GameshopDialog.cs (666行)      ← 依赖 Network
HeroDialogs.cs (796行)         ← 依赖 Network + Objects
MailDialogs.cs (1,082行)       ← 依赖 Network
TrustMerchantDialog.cs (1,351行) ← 依赖 Network + Objects
QuestDialogs.cs (1,550行)      ← 依赖 Network + Objects
NPCDialogs.cs (2,338行)        ← 依赖 Network + Objects
```

### Layer 3: 高级对话框（依赖 L1 + L2）
**优先级: 低**

```
GuildTerritoryDialog.cs (315行) ← 依赖 Guild
GuildDialog.cs (2,032行)       ← 依赖 Network + Objects
IntelligentCreatureDialogs.cs (1,234行) ← 依赖复杂系统
```

### Layer 0: 超大型复合对话框
**优先级: 特殊处理** - 需要拆分

```
MainDialogs.cs (3,715行, 10个类) ← 包含多个基础对话框
  ├─ MainDialog
  ├─ ChatDialog
  ├─ SkillBarDialog
  ├─ MiniMapDialog
  ├─ InspectDialog
  ├─ OptionDialog
  ├─ MenuDialog
  ├─ DuraStatusDialog
  └─ 其他辅助类
```

## 共享数据结构需求

### 从 SharedRust 需要的结构
```rust
// 已存在
SelectInfo                    ✅
UserItem                      ✅
ClientMail                    ✅
ClientFriend                  ✅
GuildMember                   ✅
ClientQuestInfo               ✅
ClientRecipeInfo              ✅
GameShopItem                  ✅

// 需要确认/补充
ClientBuff                    ⚠️
ClientTimer                   ⚠️
MapRecord                     ⚠️
ClientHeroInformation         ⚠️
IntelligentCreatureInfo       ⚠️
ItemRentalInfo                ⚠️
```

## 移植策略

### 阶段 1: 基础设施（已完成）
✅ Scene trait
✅ ClientState
✅ LoginScene 对话框
✅ MapControl
✅ DialogManager 基础

### 阶段 2: Layer 1 基础对话框（2-3天）
**批次 1: 最简单的 (6个，共约900行)**
```
1. CompassDialog (52)         - 指南针显示
2. ReportDialog (66)          - 举报对话框
3. ChatNoticeDialog (70)      - 聊天通知
4. SocketDialog (103)         - 宝石镶嵌
5. RollDialog (171)           - 骰子
6. NoticeDialog (304)         - 系统通知
```

**批次 2: 简单的 (5个，共约1600行)**
```
7. TimerDialog (209)          - 计时器 (已有)
8. GroupDialog (197)          - 组队
9. KeyboardLayoutDialog (358) - 键盘布局 (已有)
10. HelpDialog (347)          - 帮助
11. ChatOptionDialog (340)    - 聊天选项
12. NewCharacterDialog (318)  - 新角色 (已有部分)
```

**批次 3: 中等的 (4个，共约2500行)**
```
13. BuffDialog (763)          - Buff (已有)
14. BigMapDialog (720)        - 大地图 (已有)
15. CharacterDialog (633)     - 角色 (已有)
16. InventoryDialog (432)     - 物品栏 (已有)
```

### 阶段 3: Layer 2 中级对话框（3-4天）
**批次 4: 交易相关 (4个，共约900行)**
```
17. TradeDialogs (239)        - 交易
18. ItemRentalDialog (164)    - 物品租赁
19. ItemRentDialog (242)      - 租赁
20. ItemRentingDialog (284)   - 租赁中
```

**批次 5: 社交相关 (4个，共约1200行)**
```
21. RelationshipDialog (220)  - 关系
22. MentorDialog (272)        - 师徒
23. FriendDialog (484)        - 好友
24. RankingDialog (396)       - 排行榜
```

**批次 6: 游戏系统 (4个，共约1500行)**
```
25. MountDialog (229)         - 坐骑
26. FishingDialog (330)       - 钓鱼
27. GameshopDialog (666)      - 商城
28. HeroDialogs (796)         - 英雄
```

**批次 7: 复杂系统 (3个，共约5200行)**
```
29. MailDialogs (1,082)       - 邮件
30. TrustMerchantDialog (1,351) - 商人
31. QuestDialogs (1,550)      - 任务
32. NPCDialogs (2,338)        - NPC
```

### 阶段 4: Layer 3 高级对话框（2-3天）
```
33. GuildTerritoryDialog (315) - 公会领地
34. GuildDialog (2,032)        - 公会
35. IntelligentCreatureDialogs (1,234) - 智能生物
```

### 阶段 5: MainDialogs 拆分（3-5天）
**拆分为独立文件**:
```
MainDialogs.cs (3,715行) 拆分为:
  ├─ main_dialog.rs         (已有)
  ├─ chat_dialog.rs         (已有)
  ├─ skillbar_dialog.rs     (已有)
  ├─ minimap_dialog.rs      (需要)
  ├─ inspect_dialog.rs      (已有)
  ├─ option_dialog.rs       (已有)
  ├─ menu_dialog.rs         (已有)
  ├─ dura_status_dialog.rs  (需要)
  └─ magic_button.rs        (需要)
```

### 阶段 6: 场景完善（2-3天）
```
1. SelectScene + CharacterButton
2. GameScene 集成所有对话框
3. MapControl 完善
```

## 移植模板

### 单文件单类模板
```rust
// dialog_name.rs
// Mirrors Client/MirScenes/Dialogs/DialogName.cs

use mir2_shared::{/* 需要的共享类型 */};
use super::dialog_manager::Dialog;

#[derive(Debug)]
pub struct DialogName {
    pub visible: bool,
    // 字段...
}

impl DialogName {
    pub fn new() -> Self {
        Self {
            visible: false,
            // 初始化...
        }
    }
    
    // 方法...
}

impl Dialog for DialogName {
    fn show(&mut self) { self.visible = true; }
    fn hide(&mut self) { self.visible = false; }
    fn is_visible(&self) -> bool { self.visible }
    fn update(&mut self, _delta: f32) { }
    fn draw(&self) { }
}

#[cfg(test)]
mod tests {
    use super::*;
    // 测试...
}
```

### 多类文件模板
```rust
// complex_dialogs.rs
// Mirrors Client/MirScenes/Dialogs/ComplexDialogs.cs

// 主对话框
pub struct MainDialogPart {
    // ...
}

// 辅助类
pub struct HelperClass {
    // ...
}

// 模块导出
pub use main_dialog_part::MainDialogPart;
pub use helper_class::HelperClass;
```

## 每批次移植清单

### 移植步骤（每个对话框）
```
□ 1. 分析 C# 源码
  □ 统计字段数
  □ 统计方法数
  □ 识别依赖
  □ 识别事件处理
  
□ 2. 设计 Rust 结构
  □ 定义主结构
  □ 定义辅助结构/枚举
  □ 规划生命周期
  
□ 3. 实现核心逻辑
  □ 实现构造函数
  □ 实现公开方法
  □ 实现 Dialog trait
  
□ 4. 实现事件处理
  □ 网络事件
  □ UI 事件
  □ 游戏事件
  
□ 5. 编写测试
  □ 单元测试
  □ 集成测试
  
□ 6. 文档和注释
  □ 模块文档
  □ 方法文档
  □ C# 对照注释
  
□ 7. 集成到系统
  □ 更新 mod.rs
  □ 更新 GameScene
  □ 测试编译
```

## 质量检查清单

### 代码质量
- [ ] 所有 pub 项都有文档注释
- [ ] 实现 Debug trait
- [ ] 适当的 Default trait
- [ ] 错误处理完整
- [ ] 无 unsafe 代码（除非必要）
- [ ] 无 unwrap（除非逻辑保证安全）

### 结构对应
- [ ] 字段与 C# 一一对应
- [ ] 方法名与 C# 保持一致（驼峰转蛇形）
- [ ] 事件处理逻辑相同
- [ ] 数据流向相同

### 测试覆盖
- [ ] 创建/销毁测试
- [ ] 显示/隐藏测试
- [ ] 核心逻辑测试
- [ ] 边界条件测试

## 时间估算

### 保守估计
- 阶段 2 (Layer 1): 15 个对话框 × 0.5天 = 7.5天
- 阶段 3 (Layer 2): 16 个对话框 × 1天 = 16天
- 阶段 4 (Layer 3): 3 个对话框 × 2天 = 6天
- 阶段 5 (MainDialogs): 拆分 + 补充 = 5天
- 阶段 6 (场景): 3天
- **总计: 约 37.5 工作日 (7-8周)**

### 激进估计
- 阶段 2: 15 × 0.3天 = 4.5天
- 阶段 3: 16 × 0.7天 = 11.2天
- 阶段 4: 3 × 1.5天 = 4.5天
- 阶段 5: 3天
- 阶段 6: 2天
- **总计: 约 25 工作日 (5周)**

## 当前状态快照

### ✅ 已完成（约8%）
```
LoginDialog (268行)
NewAccountDialog (428行)
ChangePasswordDialog (351行)
MapControl (305行)
select_scene 基础 (220行)
game_scene 框架 (552行)
login_scene 增强 (431行)
---
已有对话框:
- dialog_manager (已有)
- main_dialog (已有)
- chat_dialog (已有)
- inventory_dialog (已有)
- character_dialog (已有)
- skillbar_dialog (已有)
- belt_dialog (已有)
- buff_dialog (已有)
- bigmap_dialog (已有)
等等...
```

### ⚠️ 部分完成（需要补充）
```
很多对话框都有骨架，但缺少完整实现
需要逐个检查和补充
```

### ❌ 未开始（约70%）
```
Layer 1 批次 1-2 的大部分
Layer 2 所有
Layer 3 所有
MainDialogs 拆分
```

## 推荐执行顺序

### 本次任务: 开始 Layer 1 批次 1
```
1. CompassDialog     - 最简单
2. ChatNoticeDialog  - 很简单
3. ReportDialog      - 简单
4. SocketDialog      - 简单
5. NoticeDialog      - 中等
6. RollDialog        - 中等
```

这样可以:
- ✅ 快速积累成就感（简单的先做）
- ✅ 建立模板和最佳实践
- ✅ 验证移植流程
- ✅ 为复杂对话框铺路

---

**下一步**: 从 CompassDialog 开始，这是最简单的（仅52行）
