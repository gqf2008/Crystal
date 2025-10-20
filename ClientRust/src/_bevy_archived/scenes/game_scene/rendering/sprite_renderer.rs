// Sprite Renderer - Bevy Sprite 批处理渲染
// 
// 功能说明:
// 使用 Bevy 的 Sprite 组件批处理渲染游戏对象
// 
// TODO: 实现批处理优化
// 参考: Bevy 2D sprite batching examples

use bevy::prelude::*;

/// Sprite 渲染器配置
#[derive(Resource)]
pub struct SpriteRenderer {
    /// 最大批处理数量
    pub max_batch_size: usize,
}

impl Default for SpriteRenderer {
    fn default() -> Self {
        Self {
            max_batch_size: 1000,
        }
    }
}

/// 初始化 Sprite 渲染器
pub fn setup_sprite_renderer(mut commands: Commands) {
    commands.insert_resource(SpriteRenderer::default());
    info!("✅ SpriteRenderer 初始化完成");
}
