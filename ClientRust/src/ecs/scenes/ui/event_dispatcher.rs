//! UI事件分发器
//! 
//! 负责管理UI事件的传播和分发，实现以下功能：
//! 1. Z-order层级管理（自动处理UI元素的绘制和事件优先级）
//! 2. 事件捕获和冒泡（支持组件拦截事件或让事件继续传播）
//! 3. 焦点管理（自动处理输入焦点）
//! 4. 模态对话框支持（阻止底层UI接收点击事件）

use ggez::{Context, graphics::Canvas, input::keyboard::KeyCode};

/// 事件处理结果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventResult {
    /// 事件已处理，停止传播
    Handled,
    /// 事件已处理，但允许继续传播（用于悬停效果）
    HandledContinue,
    /// 事件未处理，继续传播
    Unhandled,
}

impl EventResult {
    /// 检查事件是否应该停止传播
    pub fn should_stop(&self) -> bool {
        matches!(self, EventResult::Handled)
    }
    
    /// 检查事件是否被处理过
    pub fn is_handled(&self) -> bool {
        !matches!(self, EventResult::Unhandled)
    }
}

/// UI组件特性 - 所有可接收事件的UI元素都需要实现此特性
pub trait UIComponent {
    /// 检查点是否在组件内
    fn contains_point(&self, x: f32, y: f32) -> bool;
    
    /// 组件是否可见
    fn is_visible(&self) -> bool { true }
    
    /// 组件是否模态（阻止底层UI接收点击事件）
    fn is_modal(&self) -> bool { false }
    
    /// 组件是否接受键盘焦点
    fn is_focusable(&self) -> bool { false }
    
    /// 组件是否当前拥有焦点
    fn has_focus(&self) -> bool { false }
    
    /// 设置焦点状态
    fn set_focus(&mut self, _focused: bool) {}
    
    // === 鼠标事件 ===
    
    /// 鼠标移动事件
    /// 返回 HandledContinue 允许底层UI显示悬停效果
    fn on_mouse_move(&mut self, _x: f32, _y: f32) -> EventResult {
        EventResult::Unhandled
    }
    
    /// 鼠标按下事件
    fn on_mouse_down(&mut self, _x: f32, _y: f32) -> EventResult {
        EventResult::Unhandled
    }
    
    /// 鼠标释放事件
    fn on_mouse_up(&mut self, _x: f32, _y: f32) -> EventResult {
        EventResult::Unhandled
    }
    
    /// 鼠标点击事件（按下并在同一位置释放）
    fn on_click(&mut self, _x: f32, _y: f32) -> EventResult {
        EventResult::Unhandled
    }
    
    // === 键盘事件 ===
    
    /// 键盘按下事件（仅当组件有焦点时调用）
    fn on_key_down(&mut self, _keycode: KeyCode) -> EventResult {
        EventResult::Unhandled
    }
    
    /// 字符输入事件（仅当组件有焦点时调用）
    fn on_char_input(&mut self, _ch: char) -> EventResult {
        EventResult::Unhandled
    }
    
    // === 焦点事件 ===
    
    /// 获得焦点事件
    fn on_focus_gained(&mut self) {}
    
    /// 失去焦点事件
    fn on_focus_lost(&mut self) {}
}

/// UI层 - 管理一组相关的UI组件
pub struct UILayer {
    /// 层名称（用于调试）
    pub name: String,
    /// 层的Z-order（数值越大越靠前）
    pub z_order: i32,
    /// 是否可见
    pub visible: bool,
    /// 是否模态（阻止底层接收点击事件）
    pub modal: bool,
}

impl UILayer {
    pub fn new(name: impl Into<String>, z_order: i32) -> Self {
        Self {
            name: name.into(),
            z_order,
            visible: true,
            modal: false,
        }
    }
    
    pub fn modal(mut self) -> Self {
        self.modal = true;
        self
    }
}

/// UI事件分发器
pub struct UIEventDispatcher {
    /// 所有UI层（按Z-order排序）
    layers: Vec<UILayer>,
    /// 当前焦点组件的层索引
    focused_layer: Option<usize>,
}

impl UIEventDispatcher {
    pub fn new() -> Self {
        Self {
            layers: Vec::new(),
            focused_layer: None,
        }
    }
    
    /// 添加UI层
    pub fn add_layer(&mut self, layer: UILayer) {
        self.layers.push(layer);
        // 按Z-order降序排序（高Z-order在前）
        self.layers.sort_by(|a, b| b.z_order.cmp(&a.z_order));
    }
    
    /// 移除UI层
    pub fn remove_layer(&mut self, name: &str) -> Option<UILayer> {
        if let Some(idx) = self.layers.iter().position(|l| l.name == name) {
            let layer = self.layers.remove(idx);
            if self.focused_layer == Some(idx) {
                self.focused_layer = None;
            }
            Some(layer)
        } else {
            None
        }
    }
    
    /// 分发鼠标移动事件
    /// 
    /// 策略：
    /// - 从高Z-order向低Z-order遍历
    /// - 如果组件返回Handled，停止传播
    /// - 如果组件返回HandledContinue，继续传播（用于悬停效果）
    /// - 如果遇到模态层，停止传播到更底层
    pub fn dispatch_mouse_move<F>(&mut self, x: f32, y: f32, mut handler: F) 
    where
        F: FnMut(&str) -> EventResult,
    {
        for layer in &self.layers {
            if !layer.visible {
                continue;
            }
            
            let result = handler(&layer.name);
            
            if result.should_stop() {
                break;
            }
            
            // 模态层阻止事件继续向下传播
            if layer.modal {
                break;
            }
        }
    }
    
    /// 分发鼠标点击事件
    /// 
    /// 策略：
    /// - 从高Z-order向低Z-order遍历
    /// - 点击会自动设置焦点到可聚焦组件
    /// - 模态层阻止底层接收点击
    pub fn dispatch_mouse_down<F>(&mut self, x: f32, y: f32, mut handler: F) -> EventResult
    where
        F: FnMut(&str) -> EventResult,
    {
        for (idx, layer) in self.layers.iter().enumerate() {
            if !layer.visible {
                continue;
            }
            
            let result = handler(&layer.name);
            
            if result.is_handled() {
                // 自动设置焦点
                self.focused_layer = Some(idx);
                return result;
            }
            
            // 模态层阻止事件继续向下传播
            if layer.modal {
                return EventResult::Handled;
            }
        }
        
        // 点击空白区域，清除焦点
        self.focused_layer = None;
        EventResult::Unhandled
    }
    
    /// 分发键盘事件（仅发送给有焦点的层）
    pub fn dispatch_key_down<F>(&mut self, keycode: KeyCode, mut handler: F) -> EventResult
    where
        F: FnMut(&str) -> EventResult,
    {
        if let Some(idx) = self.focused_layer {
            if idx < self.layers.len() && self.layers[idx].visible {
                return handler(&self.layers[idx].name);
            }
        }
        EventResult::Unhandled
    }
    
    /// 分发字符输入事件（仅发送给有焦点的层）
    pub fn dispatch_char_input<F>(&mut self, ch: char, mut handler: F) -> EventResult
    where
        F: FnMut(&str) -> EventResult,
    {
        if let Some(idx) = self.focused_layer {
            if idx < self.layers.len() && self.layers[idx].visible {
                return handler(&self.layers[idx].name);
            }
        }
        EventResult::Unhandled
    }
    
    /// 获取当前有焦点的层名称
    pub fn focused_layer_name(&self) -> Option<&str> {
        self.focused_layer
            .and_then(|idx| self.layers.get(idx))
            .map(|layer| layer.name.as_str())
    }
    
    /// 手动设置焦点到指定层
    pub fn set_focus(&mut self, layer_name: &str) {
        self.focused_layer = self.layers.iter()
            .position(|l| l.name == layer_name);
    }
    
    /// 清除焦点
    pub fn clear_focus(&mut self) {
        self.focused_layer = None;
    }
    
    /// 获取可见层的绘制顺序（从底到顶）
    pub fn draw_order(&self) -> impl Iterator<Item = &UILayer> {
        self.layers.iter().rev().filter(|l| l.visible)
    }
}

impl Default for UIEventDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_layer_ordering() {
        let mut dispatcher = UIEventDispatcher::new();
        
        dispatcher.add_layer(UILayer::new("background", 0));
        dispatcher.add_layer(UILayer::new("dialog", 10));
        dispatcher.add_layer(UILayer::new("tooltip", 20));
        
        // 应该按Z-order降序排序
        assert_eq!(dispatcher.layers[0].name, "tooltip");
        assert_eq!(dispatcher.layers[1].name, "dialog");
        assert_eq!(dispatcher.layers[2].name, "background");
    }
    
    #[test]
    fn test_modal_blocking() {
        let mut dispatcher = UIEventDispatcher::new();
        
        dispatcher.add_layer(UILayer::new("background", 0));
        dispatcher.add_layer(UILayer::new("modal_dialog", 10).modal());
        
        let mut events = Vec::new();
        
        dispatcher.dispatch_mouse_down(0.0, 0.0, |name| {
            events.push(name.to_string());
            EventResult::Unhandled
        });
        
        // 模态对话框应该阻止事件传播到background
        assert_eq!(events, vec!["modal_dialog"]);
    }
}
