// ============================================================================
// 坐骑对话框 — MountDialog (对应 C# MountDialog.cs)
// ============================================================================

use ggez::{Context, GameResult};
use ggez::graphics::Canvas;

/// 坐骑对话框
pub struct MountDialog {
    pub visible: bool,
    pub position: (f32, f32),
    pub size: (f32, f32),
    /// 坐骑名称
    pub mount_name: String,
    /// 坐骑等级
    pub mount_level: u16,
    /// 坐骑经验
    pub mount_exp: (u64, u64),
    /// 是否已骑乘
    pub is_riding: bool,
}

impl MountDialog {
    pub fn new() -> Self {
        Self {
            visible: false,
            position: (200.0, 150.0),
            size: (260.0, 300.0),
            mount_name: String::new(),
            mount_level: 0,
            mount_exp: (0, 0),
            is_riding: false,
        }
    }

    pub fn show(&mut self, name: &str, level: u16) {
        self.visible = true;
        self.mount_name = name.to_string();
        self.mount_level = level;
        tracing::info!("🐴 坐骑: {} (Lv.{})", name, level);
    }

    pub fn close(&mut self) { self.visible = false; }

    pub fn draw(&self, _ctx: &mut Context, _canvas: &mut Canvas) -> GameResult {
        if !self.visible { return Ok(()); }
        // TODO: 绘制坐骑界面
        Ok(())
    }
}

impl Default for MountDialog {
    fn default() -> Self {
        Self::new()
    }
}
