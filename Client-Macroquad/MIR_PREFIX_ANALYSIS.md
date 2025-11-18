# 去掉 `Mir` 前缀的设计决策

## 🤔 为什么要有 `Mir` 前缀？

### 历史背景
- **Legend of Mir 2（传奇2）** 是原版游戏名称
- 原版C#客户端使用 `Mir` 作为所有UI组件的前缀：
  - `MirButton`, `MirLabel`, `MirTextBox`
  - `MirImageControl`, `MirControl` 等
- 这是为了区分游戏特有的UI组件和.NET Framework的标准组件

## 🚫 现在 `Mir` 前缀的问题

### 1. **冗余性**
```rust
// 旧的命名方式
use egui::Button;           // egui的按钮
use crate::ui::MirButton;   // 我们的自定义按钮？？？

// 导致混乱：什么时候用哪个？
```

### 2. **与现代Rust惯例不符**
```rust
// Rust标准库不使用前缀
use std::collections::HashMap;  // 不是 StdHashMap
use std::fs::File;              // 不是 StdFile

// egui也不使用前缀
use egui::{Button, Label, TextEdit};  // 不是 EguiButton, EguiLabel
```

### 3. **维护负担**
- 需要维护两套组件系统
- 代码量增加了84%的冗余
- 新手难以理解什么时候用什么组件

### 4. **语义混乱**
```rust
// 这样的代码让人困惑
fn create_ui() {
    let egui_button = egui::Button::new("确定");  
    let mir_button = MirButton::new("取消");    // 为什么需要两种按钮？
}
```

## ✅ 去掉前缀后的优势

### 1. **简洁清晰**
```rust
// 新的命名方式
pub struct Dialog { ... }           // 而不是 MirDialog
pub trait Control { ... }           // 而不是 MirControl  
pub struct UiResponse { ... }       // 而不是 MirResponse
```

### 2. **符合Rust惯例**
```rust
// 通过模块来组织命名空间
use crate::ui::components::{Dialog, Control};
use crate::ui::layouts::Layout;
use egui::{Button, Label, TextEdit};
```

### 3. **更好的API设计**
```rust
// 旧方式：混乱
let button = MirButton::new("id").with_text("按钮");
if button.draw(...) { ... }

// 新方式：直接使用egui
if ui.button("按钮").clicked() { ... }
```

### 4. **减少认知负担**
```rust
// 开发者只需要学习一套API
// 游戏特有的功能用特定名称标识
pub struct GameDialog { ... }       // 明确这是游戏对话框
pub struct ShopWindow { ... }       // 明确这是商店窗口
pub struct InventoryPanel { ... }   // 明确这是背包面板
```

## 🎯 新的命名约定

### 基础trait
```rust
pub trait Control { ... }           // 替代 MirControl
pub trait ImageControl { ... }      // 替代 MirImageControl
pub struct UiResponse { ... }       // 替代 MirResponse
pub struct Layout { ... }           // 替代 MirLayout
```

### 具体组件
```rust
// 游戏特有的用具体名称
pub struct Dialog { ... }           // 通用对话框
pub struct GameDialog { ... }       // 游戏特有对话框
pub struct ShopWindow { ... }       // 商店窗口
pub struct InventoryPanel { ... }   // 背包面板
pub struct ChatWindow { ... }       // 聊天窗口

// 简单UI直接用egui
ui.button("确定");                   // 不需要 MirButton
ui.label("标签");                    // 不需要 MirLabel
ui.text_edit_singleline(&mut text); // 不需要 MirTextBox
```

## 📊 对比表

| 方面 | 使用`Mir`前缀 | 去掉前缀 |
|------|-------------|----------|
| **代码量** | ~2500行（16个文件） | ~400行（4个文件） |
| **学习成本** | 需要学习两套API | 只需学习egui |
| **维护性** | 需要维护自定义组件 | 依赖成熟的egui |
| **可读性** | `MirButton` vs `egui::Button` | 直接使用 `ui.button()` |
| **扩展性** | 需要包装所有egui功能 | 直接使用egui所有功能 |

## 🔄 迁移策略

### 第一阶段：重命名基础trait
```rust
MirControl     → Control
MirImageControl → ImageControl  
MirResponse    → UiResponse
MirLayout      → Layout
```

### 第二阶段：简化组件
```rust
MirDialog      → Dialog (保留，游戏特有)
MirButton      → 直接使用 egui::Button
MirLabel       → 直接使用 egui::Label
MirTextBox     → 直接使用 egui::TextEdit
```

### 第三阶段：重构业务逻辑
```rust
// 从这样：
let mut button = MirButton::new("id");
if button.draw(ui, pos, size) { ... }

// 改成这样：
if ui.button("文本").clicked() { ... }
```

## 🎉 预期效果

1. **代码减少84%** - 删除冗余的组件包装
2. **学习成本降低** - 只需要学习egui API
3. **维护性提升** - 依赖成熟的开源项目
4. **开发效率提升** - 更少的代码，更直观的API
5. **Bug减少** - egui经过充分测试

## 💡 结论

去掉 `Mir` 前缀是正确的决策，因为：

1. **历史包袱** - `Mir`前缀来自原版C#，在Rust+egui环境下不再需要
2. **现代化** - 符合Rust社区的命名惯例
3. **实用性** - egui功能完善，不需要重复造轮子
4. **简洁性** - 更少的代码，更清晰的意图

**建议**：保留必要的游戏特有组件（如`Dialog`），其他全部使用egui原生组件。