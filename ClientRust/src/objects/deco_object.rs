// ============================================================================
// 装饰对象 — DecoObject (对应 C# DecoObject.cs)
// ============================================================================
//
// 地图上的纯装饰性对象，不阻挡移动，不可交互。
// 用于显示地图上的装饰物（花草、石头、路标等）。

/// 装饰对象 (不可阻挡、不可交互)
#[derive(Debug, Clone)]
pub struct DecoObject {
    /// 对象唯一ID
    pub object_id: u32,
    /// 当前位置 (格子坐标)
    pub current_location: (i32, i32),
    /// 地图位置 (格子坐标)
    pub map_location: (i32, i32),
    /// 绘制位置 (屏幕坐标)
    pub draw_location: (i32, i32),
    /// 图像索引
    pub image: i32,
    /// 绘制颜色
    pub draw_color: (u8, u8, u8, u8),
    /// 全局显示偏移
    pub global_display_offset: (i32, i32),
}

impl DecoObject {
    /// 创建新的装饰对象
    pub fn new(object_id: u32) -> Self {
        Self {
            object_id,
            current_location: (0, 0),
            map_location: (0, 0),
            draw_location: (0, 0),
            image: 0,
            draw_color: (255, 255, 255, 255),
            global_display_offset: (0, 0),
        }
    }

    /// 从服务器数据加载
    pub fn load(&mut self, location: (i32, i32), image: i32) {
        self.current_location = location;
        self.map_location = location;
        self.image = image;
        tracing::debug!(
            "🎨 装饰对象加载: id={}, pos=({},{}), image={}",
            self.object_id,
            location.0,
            location.1,
            image
        );
    }

    /// 处理 (更新绘制位置)
    pub fn process(
        &mut self,
        user_movement: (i32, i32),
        user_offset_move: (i32, i32),
        offset_x: i32,
        offset_y: i32,
        cell_width: i32,
        cell_height: i32,
    ) {
        self.draw_location = (
            (self.current_location.0 - user_movement.0 + offset_x) * cell_width
                + self.global_display_offset.0
                + user_offset_move.0,
            (self.current_location.1 - user_movement.1 + offset_y) * cell_height
                + self.global_display_offset.1
                + user_offset_move.1,
        );
    }

    /// 是否阻挡 (装饰对象永远不阻挡)
    pub fn is_blocking(&self) -> bool {
        false
    }

    /// 鼠标是否悬停 (装饰对象不响应鼠标)
    pub fn mouse_over(&self, _point: (i32, i32)) -> bool {
        false
    }
}
