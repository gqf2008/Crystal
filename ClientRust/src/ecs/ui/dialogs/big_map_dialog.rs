// ============================================================================
// 大地图对话框 — BigMapDialog (对应 C# BigMapDialog.cs)
// ============================================================================

use ggez::{Context, GameResult};
use ggez::graphics::Canvas;

/// 大地图对话框
pub struct BigMapDialog {
    /// 是否可见
    pub visible: bool,
    /// 位置
    pub position: (f32, f32),
    /// 尺寸
    pub size: (f32, f32),
    /// 当前地图名称
    pub map_title: String,
    /// 地图大小 (格子数)
    pub map_size: (i32, i32),
    /// 玩家位置 (格子坐标)
    pub player_position: (i32, i32),
    /// 缩放比例
    pub zoom: f32,
    /// 视口偏移
    pub view_offset: (f32, f32),
    /// 标记点列表 (名称, 位置)
    pub markers: Vec<(String, (i32, i32))>,
}

impl BigMapDialog {
    pub fn new() -> Self {
        Self {
            visible: false,
            position: (50.0, 50.0),
            size: (600.0, 500.0),
            map_title: String::new(),
            map_size: (0, 0),
            player_position: (0, 0),
            zoom: 1.0,
            view_offset: (0.0, 0.0),
            markers: Vec::new(),
        }
    }

    /// 显示地图
    pub fn show(&mut self, title: &str, map_size: (i32, i32), player_pos: (i32, i32)) {
        self.visible = true;
        self.map_title = title.to_string();
        self.map_size = map_size;
        self.player_position = player_pos;
        self.center_on_player();
        tracing::info!("🗺️ 大地图: {} ({}x{})", title, map_size.0, map_size.1);
    }

    /// 居中到玩家位置
    pub fn center_on_player(&mut self) {
        self.view_offset = (
            self.player_position.0 as f32 - self.size.0 / 2.0 / self.zoom,
            self.player_position.1 as f32 - self.size.1 / 2.0 / self.zoom,
        );
    }

    /// 缩放
    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom.clamp(0.25, 4.0);
    }

    pub fn close(&mut self) {
        self.visible = false;
    }

    pub fn draw(&self, _ctx: &mut Context, _canvas: &mut Canvas) -> GameResult {
        if !self.visible {
            return Ok(());
        }
        // TODO: 绘制大地图
        Ok(())
    }

    pub fn handle_click(&mut self, _x: f32, _y: f32) -> bool {
        if !self.visible {
            return false;
        }
        // TODO: 处理拖拽/缩放/标记点击
        false
    }
}

impl Default for BigMapDialog {
    fn default() -> Self {
        Self::new()
    }
}
