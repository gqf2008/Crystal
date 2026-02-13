// ============================================================================
// 下拉框控件 — MirDropDownBox (对应 C# MirDropDownBox.cs)
// ============================================================================
//
// 提供下拉选择功能，支持滚动浏览选项列表。
// 用于设置界面的分辨率选择、选项切换等。

use ggez::{Context, GameResult};
use ggez::graphics::Canvas;

/// 下拉框控件
pub struct MirDropDownBox {
    /// 是否可见
    pub visible: bool,
    /// 是否启用
    pub enabled: bool,
    /// 位置 (屏幕坐标)
    pub position: (f32, f32),
    /// 尺寸
    pub size: (f32, f32),
    /// 是否展开
    pub is_open: bool,
    /// 选项列表
    pub items: Vec<String>,
    /// 当前选中索引 (-1 表示无选中)
    pub selected_index: i32,
    /// 期望选中索引 (用于 ValueChanged 事件)
    pub wanted_index: i32,
    /// 最小选项索引
    pub minimum_option: usize,
    /// 滚动偏移
    pub scroll_index: usize,
    /// 原始高度 (折叠时)
    pub orig_height: f32,
    /// 最大可见选项数
    max_visible_options: usize,
    /// 背景颜色
    pub back_color: (u8, u8, u8, u8),
    /// 前景颜色
    pub fore_color: (u8, u8, u8, u8),
}

impl MirDropDownBox {
    /// 创建新的下拉框
    pub fn new() -> Self {
        Self {
            visible: true,
            enabled: true,
            position: (0.0, 0.0),
            size: (120.0, 20.0),
            is_open: false,
            items: Vec::new(),
            selected_index: -1,
            wanted_index: -1,
            minimum_option: 0,
            scroll_index: 0,
            orig_height: 20.0,
            max_visible_options: 5,
            back_color: (255, 6, 6, 6),
            fore_color: (255, 255, 255, 255),
        }
    }

    /// 设置选项列表
    pub fn set_items(&mut self, items: Vec<String>) {
        self.items = items;
        self.selected_index = -1;
        self.close();
    }

    /// 获取选中的文本
    pub fn selected_text(&self) -> Option<&str> {
        if self.selected_index >= 0 && (self.selected_index as usize) < self.items.len() {
            Some(&self.items[self.selected_index as usize])
        } else {
            None
        }
    }

    /// 切换展开/折叠
    pub fn toggle(&mut self) {
        if self.is_open {
            self.close();
        } else {
            self.open();
        }
    }

    /// 展开下拉框
    pub fn open(&mut self) {
        if self.items.is_empty() {
            return;
        }
        self.is_open = true;
        self.scroll_index = if self.items.len() > self.max_visible_options
            && self.selected_index > 3
        {
            (self.selected_index as usize).saturating_sub(2)
        } else {
            0
        };
        tracing::debug!("📂 下拉框展开: {} 个选项", self.items.len());
    }

    /// 折叠下拉框
    pub fn close(&mut self) {
        self.is_open = false;
    }

    /// 选择选项
    pub fn select_option(&mut self, visible_index: usize) -> bool {
        let actual_index = self.scroll_index + visible_index + self.minimum_option;
        if actual_index < self.items.len() {
            self.wanted_index = actual_index as i32;
            self.selected_index = actual_index as i32;
            self.close();
            tracing::debug!(
                "✅ 下拉框选择: [{}] {}",
                actual_index,
                self.items[actual_index]
            );
            return true;
        }
        false
    }

    /// 向上滚动
    pub fn scroll_up(&mut self) {
        if self.scroll_index > 0 {
            self.scroll_index -= 1;
        }
    }

    /// 向下滚动
    pub fn scroll_down(&mut self) {
        if self.scroll_index + self.max_visible_options < self.items.len() {
            self.scroll_index += 1;
        }
    }

    /// 获取当前可见的选项
    pub fn visible_options(&self) -> Vec<(usize, &str)> {
        let start = self.scroll_index + self.minimum_option;
        let end = (start + self.max_visible_options).min(self.items.len());
        (start..end)
            .map(|i| (i, self.items[i].as_str()))
            .collect()
    }

    /// 是否需要滚动条
    pub fn needs_scrollbar(&self) -> bool {
        self.items.len() > self.max_visible_options
    }

    /// 处理点击事件
    pub fn handle_click(&mut self, x: f32, y: f32) -> bool {
        if !self.visible || !self.enabled {
            return false;
        }

        // 检查是否点击了下拉按钮区域
        let button_x = self.position.0 + self.size.0 - 18.0;
        if x >= button_x
            && x <= self.position.0 + self.size.0
            && y >= self.position.1
            && y <= self.position.1 + self.orig_height
        {
            self.toggle();
            return true;
        }

        // 检查是否点击了选项 (展开时)
        if self.is_open {
            let option_y_start = self.position.1 + self.orig_height;
            for i in 0..self.max_visible_options.min(self.items.len()) {
                let opt_y = option_y_start + (i as f32) * 15.0;
                if x >= self.position.0
                    && x <= self.position.0 + self.size.0 - 16.0
                    && y >= opt_y
                    && y <= opt_y + 15.0
                {
                    self.select_option(i);
                    return true;
                }
            }
        }

        false
    }

    /// 绘制下拉框
    pub fn draw(&self, _ctx: &mut Context, _canvas: &mut Canvas) -> GameResult {
        if !self.visible {
            return Ok(());
        }
        // TODO: 绘制下拉框背景
        // TODO: 绘制当前选中文本
        // TODO: 绘制下拉按钮
        // TODO: 如果展开，绘制选项列表
        // TODO: 如果需要，绘制滚动条
        Ok(())
    }
}

impl Default for MirDropDownBox {
    fn default() -> Self {
        Self::new()
    }
}
