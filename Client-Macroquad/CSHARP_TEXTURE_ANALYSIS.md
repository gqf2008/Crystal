# C#原版组件纹理分析报告

## 🎨 C#原版组件的纹理使用情况

经过分析原版C#代码，我发现了一个重要的事实：

### ✅ **有纹理的组件**

#### 1. **基于MirImageControl的组件** - 都有纹理
```csharp
// MirButton - 继承自MirImageControl
public class MirButton : MirImageControl
{
    public int HoverIndex { get; set; }    // 悬停纹理索引
    public int PressedIndex { get; set; }  // 按下纹理索引
    public int DisabledIndex { get; set; } // 禁用纹理索引
    // 基础的Index来自MirImageControl
}

// MirCheckBox - 继承自MirButton
public class MirCheckBox : MirButton  
{
    public int TickedIndex { get; set; }    // 选中状态纹理
    public int UnTickedIndex { get; set; }  // 未选中状态纹理
}

// MirAmountBox - 继承自MirImageControl
public sealed class MirAmountBox : MirImageControl
{
    // 对话框背景有纹理
    Index = 238;                    // 背景纹理索引
    Library = Libraries.Prguse;     // 纹理库
    
    // 内部按钮也都有纹理
    CloseButton = new MirButton {
        HoverIndex = 361,
        Index = 360,
        PressedIndex = 362,
        Library = Libraries.Prguse2
    };
}
```

#### 2. **MirImageControl基类** - 纹理系统的核心
```csharp
public class MirImageControl : MirControl
{
    public MLibrary Library { get; set; }   // 纹理库
    public int Index { get; set; }          // 纹理索引
    public bool DrawImage { get; set; }     // 是否绘制纹理
    public bool AutoSize { get; set; }      // 自动根据纹理调整尺寸
}
```

### ❌ **纯文本/逻辑组件** - 没有纹理背景

#### 1. **MirLabel** - 继承自MirControl（不是MirImageControl）
```csharp
public class MirLabel : MirControl  // 注意：不是MirImageControl
{
    // 只有文本渲染相关属性
    public Font Font { get; set; }
    public Color ForeColour { get; set; }
    public TextFormatFlags DrawFormat { get; set; }
    public bool OutLine { get; set; }
    // 没有纹理相关属性
}
```

#### 2. **MirTextBox** - 继承自MirControl
```csharp
public sealed class MirTextBox : MirControl  // 不是MirImageControl
{
    // 使用Windows原生TextBox控件
    private System.Windows.Forms.TextBox TextBox;
    // 没有背景纹理，使用纯色背景
}
```

## 🎯 **关键发现**

### 组件继承结构
```
MirControl (基类)
├── MirLabel           ❌ 无纹理 (纯文本)
├── MirTextBox         ❌ 无纹理 (原生控件)
└── MirImageControl    ✅ 有纹理系统
    ├── MirButton      ✅ 多状态纹理 (normal/hover/pressed/disabled)
    ├── MirCheckBox    ✅ 多状态纹理 (checked/unchecked + button states)
    ├── MirAmountBox   ✅ 对话框背景纹理
    ├── MirInputBox    ✅ 对话框背景纹理
    ├── MirMessageBox  ✅ 对话框背景纹理
    └── 其他对话框...   ✅ 都有背景纹理
```

## 💡 **对我们Rust版本的启示**

### 1. **纹理策略重新评估**
我们之前认为所有组件都应该用egui原生组件，但实际上：

- **MirButton, MirCheckBox** → 确实需要纹理支持！
- **MirLabel, MirTextBox** → 可以用egui原生组件
- **各种对话框** → 需要纹理背景

### 2. **应该保留的纹理组件**
```rust
// 需要纹理的组件
pub struct TexturedButton {      // 对应MirButton
    pub normal_index: usize,
    pub hover_index: Option<usize>,
    pub pressed_index: Option<usize>,
    pub disabled_index: Option<usize>,
    pub library: LibraryName,
}

pub struct TexturedCheckBox {    // 对应MirCheckBox  
    pub checked_index: usize,
    pub unchecked_index: usize,
    pub library: LibraryName,
}

pub struct TexturedDialog {      // 对应各种对话框
    pub background_index: usize,
    pub library: LibraryName,
}
```

### 3. **混合策略才是正确的**
```rust
// 游戏UI - 使用纹理组件
let game_button = TexturedButton::new()
    .with_textures(Libraries::Prguse, 200, Some(201), Some(202));

// 工具UI - 使用egui原生
if ui.button("调试按钮").clicked() { ... }
ui.label("调试信息");
ui.text_edit_singleline(&mut debug_text);
```

## 🔄 **建议的修正方案**

我们需要重新引入一些纹理组件：

1. **保留纹理支持的组件**：
   - `TexturedButton` (对应MirButton)
   - `TexturedCheckBox` (对应MirCheckBox) 
   - `TexturedDialog` (对应各种对话框)

2. **使用egui原生的组件**：
   - `egui::Label` (对应MirLabel)
   - `egui::TextEdit` (对应MirTextBox)

3. **按用途分类使用**：
   - **游戏内UI** → 使用纹理组件保持原版风格
   - **调试/工具UI** → 使用egui原生组件快速开发

## 📝 **结论**

你问得非常好！原版C#组件确实**有些有纹理，有些没有**：

- **MirButton, MirCheckBox, 各种对话框** → 有纹理背景和多状态支持
- **MirLabel, MirTextBox** → 纯文本/原生控件，无背景纹理

我们之前"全部用egui替代"的策略过于激进了，应该采用**混合策略**：游戏UI用纹理组件，工具UI用egui原生组件。