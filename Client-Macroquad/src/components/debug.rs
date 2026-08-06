// ============================================================================
// 调试组件
// ============================================================================

use std::time::Instant;

/// 调试计数器组件 - 用于替代 unsafe static mut
///
/// 用于管理所有调试相关的计数器和标志,避免使用 unsafe 代码
#[derive(Debug, Clone)]
pub struct DebugCounters {
    /// 同步计数器 (Camera 同步日志)
    pub sync_counter: u32,

    /// 绘制帧计数 (draw 方法调用次数)
    pub draw_count: u32,

    /// Y-sorting 帧计数器
    pub frame_counter: u32,

    /// 是否启用调试日志
    pub enable_debug_logs: bool,

    /// 上次重置时间
    pub last_reset: Instant,
}

impl DebugCounters {
    /// 创建新的调试计数器
    pub fn new() -> Self {
        Self {
            sync_counter: 0,
            draw_count: 0,
            frame_counter: 0,
            enable_debug_logs: true, // 默认启用,可以通过配置关闭
            last_reset: Instant::now(),
        }
    }

    /// 递增同步计数器并检查是否应该打印日志
    ///
    /// # 返回
    /// - `true`: 应该打印日志 (首次或每300帧)
    /// - `false`: 不应该打印日志
    pub fn should_log_sync(&mut self) -> bool {
        if !self.enable_debug_logs {
            return false;
        }

        self.sync_counter += 1;
        self.sync_counter == 1 || self.sync_counter.is_multiple_of(300)
    }

    /// 递增绘制计数器并检查是否应该打印日志
    ///
    /// # 返回
    /// - `true`: 应该打印日志 (前3帧)
    /// - `false`: 不应该打印日志
    pub fn should_log_draw(&mut self) -> bool {
        if !self.enable_debug_logs {
            return false;
        }

        self.draw_count += 1;
        self.draw_count <= 3
    }

    /// 获取当前绘制计数
    pub fn get_draw_count(&self) -> u32 {
        self.draw_count
    }

    /// 递增帧计数器并检查是否应该打印 Y-sorting 日志
    ///
    /// # 返回
    /// - `true`: 应该打印日志 (每60帧)
    /// - `false`: 不应该打印日志
    pub fn should_log_y_sorting(&mut self) -> bool {
        if !self.enable_debug_logs {
            return false;
        }

        self.frame_counter += 1;
        self.frame_counter.is_multiple_of(60)
    }

    /// 重置所有计数器
    pub fn reset(&mut self) {
        self.sync_counter = 0;
        self.draw_count = 0;
        self.frame_counter = 0;
        self.last_reset = Instant::now();
    }

    /// 启用调试日志
    pub fn enable_logs(&mut self) {
        self.enable_debug_logs = true;
    }

    /// 禁用调试日志
    pub fn disable_logs(&mut self) {
        self.enable_debug_logs = false;
    }
}

impl Default for DebugCounters {
    fn default() -> Self {
        Self::new()
    }
}
