// ============================================================================
// 复选框控件 — MirCheckBox (对应 C# MirCheckBox.cs)
// ============================================================================
//
// 继承自 MirButton，提供勾选/取消勾选功能。
// 用于设置界面的选项切换。

use ggez::{Context, GameResult};
use ggez::graphics::Canvas;

/// 复选框控件
pub struct MirCheckBox {
    /// 是否可见
    pub visible: bool,
    /// 是否启用
    pub enabled: bool,
    /// 位置 (屏幕坐标)
    pub position: (f32, f32),
    /// 尺寸
    pub size: (f32, f32),
    /// 是否选中
    pub checked: bool,
    /// 选中时的图像索引
    pub ticked_index: i32,
    /// 未选中时的图像索引
    pub unticked_index: i32,
    /// 标签文本
    pub label_text: String,
    /// 是否居中文本
    pub center_label_text: bool,
}

impl MirCheckBox {
    /// 创建新的复选框
    pub fn new() -> Self {
        Self {
            visible: true,
            enabled: true,
            position: (0.0, 0.0),
            size: (16.0, 16.0),
            checked: false,
            ticked_index: -1,
            unticked_index: -1,
            label_text: String::new(),
            center_label_text: false,
        }
    }

    /// 创建带标签的复选框
    pub fn with_label(label: &str) -> Self {
        let mut cb = Self::new();
        cb.label_text = label.to_string();
        cb
    }

    /// 切换选中状态
    pub fn toggle(&mut self) {
        if !self.enabled {
            return;
        }
        self.checked = !self.checked;
        tracing::debug!("☑️ 复选框切换: {} -> {}", self.label_text, self.checked);
    }

    /// 设置选中状态
    pub fn set_checked(&mut self, checked: bool) {
        self.checked = checked;
    }

    /// 获取当前显示的图像索引
    pub fn current_index(&self) -> i32 {
        if self.checked {
            self.ticked_index
        } else {
            self.unticked_index
        }
    }

    /// 处理点击事件
    pub fn handle_click(&mut self, x: f32, y: f32) -> bool {
        if !self.visible || !self.enabled {
            return false;
        }
        if x >= self.position.0
            && x <= self.position.0 + self.size.0
            && y >= self.position.1
            && y <= self.position.1 + self.size.1
        {
            self.toggle();
            return true;
        }
        false
    }

    /// 绘制复选框
    pub fn draw(&self, _ctx: &mut Context, _canvas: &mut Canvas) -> GameResult {
        if !self.visible {
            return Ok(());
        }
        // TODO: 绘制复选框图像 (ticked/unticked)
        // TODO: 绘制标签文本
        Ok(())
    }
}

impl Default for MirCheckBox {
    fn default() -> Self {
        Self::new()
    }
}
