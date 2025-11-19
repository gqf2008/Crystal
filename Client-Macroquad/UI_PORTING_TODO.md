# UI 组件移植进度 (C# -> Rust/Macroquad/egui)

本文档跟踪从 C# `Client/MirControls` 到 Rust `Client-Macroquad/src/ui/components` 的移植进度。

## 1. 基础组件 (Basic Components)

| 组件名 (C#) | 组件名 (Rust) | 状态 | 说明 |
| :--- | :--- | :--- | :--- |
| `MirButton` | `TexturedButton` | ✅ 完成 | 支持纹理状态切换、自定义尺寸、Tooltip。已修复 egui 默认边框问题。 |
| `MirCheckBox` | `TexturedCheckBox` | ✅ 完成 | 支持原生样式和纹理样式 (Prguse: 2086/2087)。 |
| `MirLabel` | `TexturedLabel` | ✅ 完成 | 封装 egui::Label，支持颜色、对齐、字体大小。待实现：描边/阴影效果。 |
| `MirImageControl` | `egui::Image` | ⚠️ 部分 | egui 原生支持，需封装以支持 Mir 资源索引。 |
| `MirTextBox` | `egui::TextEdit` | ⏳ 待定 | egui 原生支持，可能需要封装以支持特定背景/边框。 |
| `MirAnimatedControl`| - | ❌ 未开始 | 用于播放动画序列。 |

## 2. 复合组件 (Composite Components)

| 组件名 (C#) | 组件名 (Rust) | 状态 | 说明 |
| :--- | :--- | :--- | :--- |
| `MirMessageBox` | `TexturedMessageBox` | ✅ 完成 | 模态对话框，支持 OK/Cancel/Yes/No 按钮组合。背景 Prguse:360。按钮 Title:200-212。 |
| `MirDialog` | `TexturedDialog` | ✅ 完成 | 基础窗口容器，支持拖拽、关闭按钮 (Prguse2: 360-362)。 |

## 3. 游戏窗口 (Game Dialogs)

> **注意**: `src/scenes/dialogs/game/` 目录下已存在大部分窗口的实现。目前的任务是将它们重构为使用新的 `TexturedDialog` 和 `TexturedButton` 组件，以统一风格并减少代码重复。

| 窗口名 | 对应 C# 文件 | 现有 Rust 文件 | 状态 | 纹理资源 (Library: Index) |
| :--- | :--- | :--- | :--- | :--- |
| **背包窗口** | `InventoryDialog.cs` | `inventory_dialog.rs` | ⚠️ 需重构 | 背景 Title:196。标签页 Title:197/737, 168/738, 198/739。 |
| **角色窗口** | `CharacterDialog.cs` | `character_dialog.rs` | ⚠️ 需重构 | 背景 Title:504。页面 Prguse:340(装备), Title:506(状态), 507(属性), 508(技能)。标签 Title:500-503。 |
| **技能窗口** | (包含在 CharacterDialog) | - | ❌ 未开始 | 包含在角色窗口的 SkillPage (Title:508) 中。 |
| **小地图** | `MiniMapDialog.cs` | `minimap_dialog.rs` | ⚠️ 需重构 | 背景 Prguse:2090(大), 2091(小)。按钮 Prguse:2102。 |
| **聊天框** | `ChatDialog.cs` | `chat_dialog.rs` | ⚠️ 需重构 | 背景 Prguse:2201(800x600), 2221(1024x768)。滚动条 Prguse:2012-2014。 |
| **选项设置** | `OptionDialog.cs` | `option_dialog.rs` | ⚠️ 需重构 | 背景 Prguse:1002。 |
| **任务日志** | `QuestDialogs.cs` | `quest_log_dialog.rs` | ⚠️ 需重构 | 背景 Prguse:1047。 |

## 4. 移植计划 (Roadmap)

### 第一阶段：基础建设 (已完成)

- [x] 建立 `TexturedButton` 和 `TexturedCheckBox`。
- [x] 建立 `TexturedDialog` 作为所有窗口的基类。
- [x] 建立 `TexturedMessageBox` 用于系统提示。
- [x] 建立 `test_all_components` 测试程序。

### 第二阶段：核心游戏窗口重构 (Refactoring)

- [ ] **InventoryDialog (背包)**
  - [ ] 使用 `TexturedDialog` 替换手动背景绘制。
  - [ ] 使用 `TexturedButton` 替换手动按钮绘制。
  - [ ] 优化网格渲染逻辑。
- [ ] **CharacterDialog (角色)**
  - [ ] 迁移到组件化架构。
- [ ] **其他窗口**
  - [ ] 逐步重构 Chat, Option, MiniMap 等窗口。

### 第三阶段：交互与逻辑

- [ ] 拖拽系统 (Drag & Drop) - 在 egui 中实现物品拖拽。
- [ ] 窗口层级管理 (Z-Order)。
- [ ] 键盘快捷键绑定。

## 5. 备注与问题

- **字体渲染**: 目前使用 egui 默认字体，后续需要集成游戏原版字体或类似的中文字体。
- **资源加载**: 目前依赖 `ResourceManager`，需要确保所有 UI 相关的 `.lib` 文件都能正确索引。
- **坐标系统**: C# 版使用绝对像素坐标，egui 使用逻辑坐标，移植时需注意布局转换。
