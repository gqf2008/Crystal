// ============================================================================
// 排行榜对话框 — RankingDialog (对应 C# RankingDialog.cs)
// ============================================================================

use ggez::{Context, GameResult};
use ggez::graphics::Canvas;

/// 排行榜分类
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RankingCategory {
    Level,
    /// 按职业
    ByClass,
    /// 在线时间
    OnlineTime,
}

/// 排行信息
#[derive(Debug, Clone)]
pub struct RankingEntry {
    pub rank: u32,
    pub name: String,
    pub level: u16,
    pub class: String,
}

/// 排行榜对话框
pub struct RankingDialog {
    pub visible: bool,
    pub position: (f32, f32),
    pub size: (f32, f32),
    pub current_category: RankingCategory,
    pub entries: Vec<RankingEntry>,
    pub current_page: usize,
}

impl RankingDialog {
    pub fn new() -> Self {
        Self {
            visible: false,
            position: (150.0, 80.0),
            size: (400.0, 450.0),
            current_category: RankingCategory::Level,
            entries: Vec::new(),
            current_page: 0,
        }
    }

    pub fn close(&mut self) { self.visible = false; }

    pub fn draw(&self, _ctx: &mut Context, _canvas: &mut Canvas) -> GameResult {
        if !self.visible { return Ok(()); }
        // TODO: 绘制排行榜
        Ok(())
    }
}

impl Default for RankingDialog {
    fn default() -> Self {
        Self::new()
    }
}
