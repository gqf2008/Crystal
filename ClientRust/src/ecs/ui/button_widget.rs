// ButtonWidget - 简化按钮UI的辅助结构
// 提供自动的状态管理和事件检测

use ggez::graphics::Color;

/// 按钮状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonState {
    Normal,
    Hovered,
    Pressed,
    Disabled,
}

/// 简单按钮部件
/// 
/// 用于管理按钮的位置、状态和事件检测
/// 不包含渲染逻辑(纹理由外部提供)
/// 
/// # 示例
/// ```rust
/// let button = ButtonWidget::new(1, 100.0, 200.0, 96.0, 32.0, 340)
///     .with_tooltip("开始游戏");
/// 
/// // 在事件处理中
/// if button.on_mouse_down(mouse_x, mouse_y) {
///     println!("按钮被按下!");
/// }
/// 
/// // 在绘制中
/// let texture_idx = button.get_texture_index();
/// let color = button.get_color();
/// ```
pub struct ButtonWidget {
    /// 按钮ID
    pub id: u32,
    
    /// 位置
    pub x: f32,
    pub y: f32,
    
    /// 尺寸
    pub width: f32,
    pub height: f32,
    
    /// 当前状态
    pub state: ButtonState,
    
    /// 是否启用
    pub enabled: bool,
    
    /// 纹理索引 (normal, hover, pressed, disabled)
    pub texture_indices: [i32; 4],
    
    /// 工具提示文本
    pub tooltip: Option<String>,
}

impl ButtonWidget {
    /// 创建新按钮
    pub fn new(id: u32, x: f32, y: f32, width: f32, height: f32, base_texture: i32) -> Self {
        Self {
            id,
            x,
            y,
            width,
            height,
            state: ButtonState::Normal,
            enabled: true,
            texture_indices: [
                base_texture,       // Normal
                base_texture + 1,   // Hovered
                base_texture + 2,   // Pressed
                base_texture,       // Disabled (使用 normal,但会有颜色调制)
            ],
            tooltip: None,
        }
    }
    
    /// 添加工具提示 (Builder 模式)
    pub fn with_tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }
    
    /// 设置是否启用 (Builder 模式)
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        if !enabled {
            self.state = ButtonState::Disabled;
        }
        self
    }
    
    /// 获取工具提示文本(如果鼠标悬停)
    pub fn get_tooltip(&self) -> Option<&str> {
        if self.state == ButtonState::Hovered && self.tooltip.is_some() {
            self.tooltip.as_deref()
        } else {
            None
        }
    }
    
    /// 检查点是否在按钮内
    pub fn contains(&self, x: f32, y: f32) -> bool {
        if !self.enabled {
            return false;
        }
        
        x >= self.x && x <= self.x + self.width &&
        y >= self.y && y <= self.y + self.height
    }
    
    /// 更新鼠标悬停状态
    pub fn update_hover(&mut self, mouse_x: f32, mouse_y: f32) {
        if !self.enabled {
            self.state = ButtonState::Disabled;
            return;
        }
        
        if self.contains(mouse_x, mouse_y) {
            if self.state != ButtonState::Pressed {
                self.state = ButtonState::Hovered;
            }
        } else if self.state != ButtonState::Pressed {
            self.state = ButtonState::Normal;
        }
    }
    
    /// 处理鼠标按下
    pub fn on_mouse_down(&mut self, mouse_x: f32, mouse_y: f32) -> bool {
        if self.contains(mouse_x, mouse_y) {
            self.state = ButtonState::Pressed;
            true
        } else {
            false
        }
    }
    
    /// 处理鼠标释放 (返回是否触发点击)
    pub fn on_mouse_up(&mut self, mouse_x: f32, mouse_y: f32) -> bool {
        let was_pressed = self.state == ButtonState::Pressed;
        
        if self.contains(mouse_x, mouse_y) {
            self.state = ButtonState::Hovered;
            was_pressed  // 只有之前按下且释放在按钮内才算点击
        } else {
            self.state = ButtonState::Normal;
            false
        }
    }
    
    /// 获取当前应使用的纹理索引
    pub fn get_texture_index(&self) -> i32 {
        match self.state {
            ButtonState::Normal => self.texture_indices[0],
            ButtonState::Hovered => self.texture_indices[1],
            ButtonState::Pressed => self.texture_indices[2],
            ButtonState::Disabled => self.texture_indices[3],
        }
    }
    
    /// 获取颜色调制(禁用时变灰)
    pub fn get_color(&self) -> Color {
        if self.enabled {
            Color::WHITE
        } else {
            Color::from_rgba(128, 128, 128, 128)
        }
    }
}

/// 按钮组管理器 - 管理多个按钮的状态
pub struct ButtonGroup {
    pub buttons: Vec<ButtonWidget>,
}

impl ButtonGroup {
    pub fn new() -> Self {
        Self {
            buttons: Vec::new(),
        }
    }
    
    pub fn add(&mut self, button: ButtonWidget) {
        self.buttons.push(button);
    }
    
    /// 更新所有按钮的悬停状态
    pub fn update_hover(&mut self, mouse_x: f32, mouse_y: f32) {
        for button in &mut self.buttons {
            button.update_hover(mouse_x, mouse_y);
        }
    }
    
    /// 处理鼠标按下,返回被按下的按钮ID
    pub fn on_mouse_down(&mut self, mouse_x: f32, mouse_y: f32) -> Option<u32> {
        for button in &mut self.buttons {
            if button.on_mouse_down(mouse_x, mouse_y) {
                return Some(button.id);
            }
        }
        None
    }
    
    /// 处理鼠标释放,返回被点击的按钮ID
    pub fn on_mouse_up(&mut self, mouse_x: f32, mouse_y: f32) -> Option<u32> {
        for button in &mut self.buttons {
            if button.on_mouse_up(mouse_x, mouse_y) {
                return Some(button.id);
            }
        }
        None
    }
    
    /// 根据ID获取按钮
    pub fn get_mut(&mut self, id: u32) -> Option<&mut ButtonWidget> {
        self.buttons.iter_mut().find(|b| b.id == id)
    }
}
