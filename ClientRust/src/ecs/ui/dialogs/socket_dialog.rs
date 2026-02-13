// ============================================================================
// 装备强化对话框 — SocketDialog (对应 C# SocketDialog.cs)
// ============================================================================

use ggez::{Context, GameResult};
use ggez::graphics::Canvas;

/// 装备强化/镶嵌对话框
pub struct SocketDialog {
    /// 是否可见
    pub visible: bool,
    /// 位置
    pub position: (f32, f32),
    /// 尺寸
    pub size: (f32, f32),
    /// 当前选中的装备槽位
    pub selected_slot: Option<usize>,
    /// 宝石槽位数量
    pub socket_count: usize,
}

impl SocketDialog {
    pub fn new() -> Self {
        Self {
            visible: false,
            position: (200.0, 150.0),
            size: (280.0, 350.0),
            selected_slot: None,
            socket_count: 0,
        }
    }

    pub fn show(&mut self, socket_count: usize) {
        self.visible = true;
        self.socket_count = socket_count;
        self.selected_slot = None;
        tracing::info!("💎 装备强化: {} 个宝石槽", socket_count);
    }

    pub fn close(&mut self) {
        self.visible = false;
    }

    pub fn draw(&self, _ctx: &mut Context, _canvas: &mut Canvas) -> GameResult {
        if !self.visible { return Ok(()); }
        // TODO: 绘制镶嵌界面
        Ok(())
    }

    pub fn handle_click(&mut self, _x: f32, _y: f32) -> bool {
        if !self.visible { return false; }
        // TODO: 处理宝石拖放
        false
    }
}

impl Default for SocketDialog {
    fn default() -> Self {
        Self::new()
    }
}
