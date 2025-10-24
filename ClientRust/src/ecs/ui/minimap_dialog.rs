// ============================================================================
// 小地图对话框 - MiniMapDialog
// ============================================================================

use ggez::{Context, GameResult};
use ggez::graphics::{Canvas, Color, DrawParam, Rect};

/// 小地图对话框
pub struct MiniMapDialog {
    /// 背景图像索引 (Prguse2)
    background_index: u16,
    
    /// 对话框位置 (屏幕坐标)
    position: (f32, f32),
    
    /// 对话框尺寸
    size: (f32, f32),
    
    /// 是否展开 (true=大地图, false=小地图)
    expanded: bool,
}

impl MiniMapDialog {
    /// 创建新的小地图对话框
    pub fn new() -> Self {
        Self {
            background_index: 2081, // 小地图背景 (需要从 C# 客户端确认)
            position: (1024.0 - 150.0, 0.0), // 右上角
            size: (150.0, 150.0),
            expanded: false,
        }
    }
    
    /// 切换展开/收起状态
    pub fn toggle_expand(&mut self) {
        self.expanded = !self.expanded;
        if self.expanded {
            // 展开为大地图
            self.size = (400.0, 400.0);
            self.position = ((1024.0 - 400.0) / 2.0, (768.0 - 400.0) / 2.0);
        } else {
            // 收起为小地图
            self.size = (150.0, 150.0);
            self.position = (1024.0 - 150.0, 0.0);
        }
    }
    
    /// 绘制小地图
    pub fn draw(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        // TODO: 实现实际的地图渲染
        // 1. 绘制背景
        // 2. 绘制地图缩略图
        // 3. 绘制玩家位置标记
        // 4. 绘制队友位置 (如果有)
        
        Ok(())
    }
    
    /// 处理鼠标点击
    pub fn handle_click(&mut self, x: f32, y: f32) -> bool {
        // 检查点击是否在对话框范围内
        if x >= self.position.0 && x <= self.position.0 + self.size.0
            && y >= self.position.1 && y <= self.position.1 + self.size.1
        {
            // TODO: 处理地图点击 (传送、标记等)
            return true;
        }
        false
    }
}

/// 小地图对话框组件
pub struct MiniMapDialogComp {
    pub dialog: MiniMapDialog,
    pub is_open: bool,
}

impl MiniMapDialogComp {
    pub fn new() -> Self {
        Self {
            dialog: MiniMapDialog::new(),
            is_open: false,
        }
    }
}
