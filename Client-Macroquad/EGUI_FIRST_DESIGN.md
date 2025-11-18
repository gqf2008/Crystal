# egui优先的UI组件设计方案

## 🎯 核心理念

你的建议非常正确！**所有UI绘制都应该由egui处理，而不是macroquad**。

### ✅ **新的设计原则**

1. **egui优先** - 优先使用egui原生组件
2. **按需包装** - 仅在需要纹理时才包装`egui::ImageButton`
3. **统一渲染** - 所有UI绘制由egui统一处理
4. **保持简洁** - 避免不必要的抽象层

## 🔧 **组件分类策略**

### 1. **直接使用egui原生组件**
```rust
// 这些组件无需自定义包装
ui.button("确定");                          // 替代 MirButton
ui.checkbox(&mut checked, "启用音效");       // 替代 MirCheckBox  
ui.label("标签文本");                       // 替代 MirLabel
ui.text_edit_singleline(&mut text);        // 替代 MirTextBox
ui.add(egui::ProgressBar::new(0.6));       // 替代 MirProgressBar
```

### 2. **基于egui::ImageButton的纹理组件**
```rust
// 仅在需要纹理时才创建包装
pub struct TexturedButton {
    // 基于 egui::ImageButton
    normal_texture: egui::TextureId,
    hover_texture: Option<egui::TextureId>,
    pressed_texture: Option<egui::TextureId>,
}

impl TexturedButton {
    pub fn draw(&mut self, ui: &mut egui::Ui) -> bool {
        // 使用 egui::ImageButton，让egui处理所有绘制
        let image_button = egui::ImageButton::new(
            egui::Image::new(self.normal_texture).fit_to_exact_size(size)
        );
        ui.add(image_button).clicked()
    }
}
```

### 3. **游戏特有的复合组件**
```rust
// 对话框等复杂组件，内部全部使用egui
pub struct GameDialog {
    title: String,
    background_texture: Option<egui::TextureId>,
}

impl GameDialog {
    pub fn draw(&mut self, ctx: &egui::Context) {
        egui::Window::new(&self.title)
            .show(ctx, |ui| {
                // 内部全部使用egui组件
                if ui.button("确定").clicked() { ... }
                ui.checkbox(&mut option, "选项");
            });
    }
}
```

## 🎨 **纹理集成方案**

### egui纹理管理
```rust
// 将游戏纹理转换为egui纹理
impl LibraryName {
    pub fn get_egui_texture(&self, ctx: &egui::Context, index: usize) -> Option<EguiTextureInfo> {
        // 从游戏资源加载纹理
        let game_texture = self.load_texture(index)?;
        
        // 转换为egui纹理
        let egui_texture = ctx.load_texture(
            format!("{}_{}", self.name(), index),
            game_texture.image_data,
            egui::TextureOptions::default()
        );
        
        Some(EguiTextureInfo {
            egui_texture: Some(egui_texture),
            width: game_texture.width,
            height: game_texture.height,
        })
    }
}
```

### ImageButton纹理按钮
```rust
pub struct TexturedButton {
    library: LibraryName,
    normal_index: usize,
    hover_index: Option<usize>,
    text: String,
}

impl TexturedButton {
    pub fn draw(&mut self, ui: &mut egui::Ui) -> bool {
        // 获取egui纹理
        if let Some(info) = self.library.get_egui_texture(ui.ctx(), self.normal_index) {
            if let Some(texture_id) = info.egui_texture.map(|t| t.id()) {
                // 使用egui::ImageButton - 让egui处理所有状态和绘制
                let image_button = egui::ImageButton::new(
                    egui::Image::new(texture_id)
                        .fit_to_exact_size(egui::vec2(info.width as f32, info.height as f32))
                );
                
                let response = ui.add(image_button);
                
                // 文本叠加（如果需要）
                if !self.text.is_empty() {
                    ui.painter().text(
                        response.rect.center(),
                        egui::Align2::CENTER_CENTER,
                        &self.text,
                        egui::TextStyle::Button.resolve(ui.style()),
                        ui.visuals().text_color()
                    );
                }
                
                return response.clicked();
            }
        }
        
        // fallback到普通按钮
        ui.button(&self.text).clicked()
    }
}
```

## 📊 **新旧方案对比**

| 方面 | 旧方案（macroquad绘制） | 新方案（egui绘制） |
|------|----------------------|------------------|
| **绘制方式** | macroquad手动绘制 | egui统一处理 |
| **状态管理** | 手动处理hover/pressed | egui自动处理 |
| **布局系统** | 手动计算位置 | egui布局引擎 |
| **事件处理** | 手动碰撞检测 | egui事件系统 |
| **性能** | 每帧重绘 | egui智能更新 |
| **兼容性** | 依赖macroquad | 跨平台兼容 |
| **维护性** | 需要维护所有逻辑 | 依赖成熟库 |

## 🛠️ **实现计划**

### 第一阶段：基础组件
```rust
// 1. 简单封装（保持兼容性）
pub struct CheckBox {
    // 内部使用 egui::Checkbox
}

// 2. 纹理组件
pub struct TexturedButton {
    // 基于 egui::ImageButton
}

pub struct TexturedCheckBox {  
    // 基于 egui::ImageButton
}
```

### 第二阶段：复合组件
```rust
// 对话框等复杂组件
pub struct Dialog {
    // 使用 egui::Window + 纹理背景
}

pub struct GameShop {
    // 内部全部使用egui组件
}
```

### 第三阶段：迁移现有代码
```rust
// 从这样：
let mut button = MirButton::new("id");
if button.draw_at(ui, pos, size) { ... }

// 改成这样：
if ui.button("按钮").clicked() { ... }

// 或者需要纹理时：
let mut textured_btn = TexturedButton::new()
    .with_texture(LibraryName::Prguse, 200);
if textured_btn.draw(ui) { ... }
```

## 🎉 **优势总结**

### 1. **开发效率**
- 不需要重复实现UI逻辑
- egui提供完整的布局和事件系统
- 专注于游戏逻辑而非UI细节

### 2. **用户体验**  
- egui的交互体验经过充分优化
- 自动处理hover、focus、键盘导航等
- 响应式布局，适配不同屏幕尺寸

### 3. **维护成本**
- 依赖成熟的开源项目
- bug修复和功能更新由egui社区负责
- 代码量大幅减少

### 4. **扩展性**
- 可以使用egui的所有功能
- 支持主题、动画、自定义绘制
- 易于添加新的UI功能

## 💡 **关键洞察**

你的观点完全正确：

1. **egui确实有ImageButton** - 我们应该利用这个现成的组件
2. **包装而非重写** - 在egui基础上添加纹理支持，而不是重新实现UI逻辑  
3. **egui绘制，非macroquad** - 让专业的UI库处理UI，让游戏引擎处理游戏逻辑

这种方案既保持了原版游戏的纹理风格，又享受了现代UI库的便利性！