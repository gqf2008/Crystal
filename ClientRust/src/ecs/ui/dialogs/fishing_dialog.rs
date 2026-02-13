// ============================================================================
// 钓鱼对话框 — FishingDialog (对应 C# FishingDialog.cs)
// ============================================================================

use ggez::{Context, GameResult};
use ggez::graphics::Canvas;

/// 钓鱼状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FishingState {
    /// 空闲
    Idle,
    /// 抛竿中
    Casting,
    /// 等待上钩
    Waiting,
    /// 鱼上钩了
    Hooked,
    /// 收线中
    Reeling,
}

/// 钓鱼对话框
pub struct FishingDialog {
    pub visible: bool,
    pub position: (f32, f32),
    pub size: (f32, f32),
    /// 当前状态
    pub state: FishingState,
    /// 进度条 (0.0 ~ 1.0)
    pub progress: f32,
    /// 是否自动钓鱼
    pub auto_fish: bool,
    /// 连续钓鱼次数
    pub catch_count: u32,
}

impl FishingDialog {
    pub fn new() -> Self {
        Self {
            visible: false,
            position: (300.0, 400.0),
            size: (200.0, 100.0),
            state: FishingState::Idle,
            progress: 0.0,
            auto_fish: false,
            catch_count: 0,
        }
    }

    pub fn start_fishing(&mut self) {
        self.state = FishingState::Casting;
        self.progress = 0.0;
        self.visible = true;
        tracing::info!("🎣 开始钓鱼");
    }

    pub fn on_hooked(&mut self) {
        self.state = FishingState::Hooked;
        tracing::info!("🐟 鱼上钩了!");
    }

    pub fn on_catch(&mut self) {
        self.catch_count += 1;
        self.state = FishingState::Idle;
        tracing::info!("🎉 钓到鱼了! 总计: {}", self.catch_count);
    }

    pub fn close(&mut self) {
        self.visible = false;
        self.state = FishingState::Idle;
    }

    pub fn draw(&self, _ctx: &mut Context, _canvas: &mut Canvas) -> GameResult {
        if !self.visible { return Ok(()); }
        // TODO: 绘制钓鱼界面
        Ok(())
    }
}

impl Default for FishingDialog {
    fn default() -> Self {
        Self::new()
    }
}

/// 钓鱼状态栏 (对应 C# FishingStatusDialog)
pub struct FishingStatusDialog {
    pub visible: bool,
    pub position: (f32, f32),
    /// 当前鱼竿耐久
    pub rod_durability: (u16, u16),
    /// 鱼饵数量
    pub bait_count: u32,
}

impl FishingStatusDialog {
    pub fn new() -> Self {
        Self {
            visible: false,
            position: (0.0, 0.0),
            rod_durability: (0, 0),
            bait_count: 0,
        }
    }
}

impl Default for FishingStatusDialog {
    fn default() -> Self {
        Self::new()
    }
}
