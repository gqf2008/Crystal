//! 实体渲染系统 (EntityRenderSystem)
//!
//! **⚠️ 已废弃**: 本系统已被 `SpriteRenderSystem`（`sprite_system.rs` + `character.rs`）取代。
//! 实际角色渲染走 `SpriteRenderSystem::draw_character`，包含完整的 armour/hair/weapon/wing/mount 多层绘制。
//! 本文件保留仅作为参考，不应在 GameScene 中注册。
//!
//! **优先级**: 1020 (RENDER 层)
//! **职责**: ~~渲染玩家和怪物实体~~（已由 SpriteRenderSystem 接管）
//! 
//! ## ECS 架构
//! 
//! ### 输入
//! - 查询 `(Entity, &Position, &Sprite)` - 所有带精灵图的实体
//! - 查询 `(Entity, &Position, &Sprite, &Player)` - 玩家实体
//! - 查询 `(Entity, &Position, &Sprite, &Monster)` - 怪物实体
//! - 查询 `(&Camera, &Position)` - 相机位置和缩放
//! 
//! ### 输出
//! - 将实体精灵绘制到 Canvas
//! 
//! ### 组件依赖
//! - **读取**: Position, Sprite, Camera, Player, Monster
//! - **写入**: 无（纯渲染）
//! 
//! ## 渲染流程
//! 
//! 1. 获取相机位置和缩放
//! 2. 计算屏幕可见区域
//! 3. 查询所有实体 (Position + Sprite)
//! 4. 视锥裁剪（culling）- 只渲染可见实体
//! 5. 按 Y 坐标排序（深度排序）
//! 6. 从图形库读取精灵图
//! 7. 应用相机变换，绘制到 Canvas
//! 
//! ## 深度排序
//! 
//! - Y 坐标越大（越靠下）→ 渲染优先级越高（后绘制，遮挡前面的）
//! - 相同 Y 坐标 → 按实体类型排序（地面 < 玩家 < 怪物 < 特效）
//! 
//! ## 示例
//! 
//! ```rust
//! // 创建一个玩家实体
//! world.spawn((
//!     Position { x: 100.0, y: 200.0 },
//!     Sprite { library: 0, index: 1, frame: 0, blend_mode: SpriteBlendMode::Alpha },
//!     Player { id: 1 },
//! ));
//! 
//! // EntityRenderSystem 会自动渲染它
//! ```

use hecs::World;
// use ggez::{Context, GameResult};
use crate::compat::{Canvas, Color, DrawParam, GraphicsContext, Rect};
use tracing::{info, debug};

use crate::components::{Position, Sprite, Camera, Player, Monster};
use crate::systems::RenderSystem;
use crate::resources::Libraries;

/// 实体渲染系统
/// 
/// **优先级**: 1020 (在地图之后，UI之前)
#[derive(ecs_macros::RenderSystem)]
pub struct EntityRenderSystem;

impl EntityRenderSystem {
    /// 获取相机视图范围
    fn get_camera_view_bounds(world: &World, screen_width: f32, screen_height: f32) 
        -> Option<(f32, f32, f32, f32, f32)> 
    {
        let mut query = world.query::<(&Camera, &Position)>();
        if let Some((camera, cam_pos)) = query.iter().next() {
            let zoom = camera.zoom;
            let half_width = (screen_width / 2.0) / zoom;
            let half_height = (screen_height / 2.0) / zoom;
            
            Some((
                cam_pos.x - half_width,  // min_x
                cam_pos.y - half_height, // min_y
                cam_pos.x + half_width,  // max_x
                cam_pos.y + half_height, // max_y
                zoom,
            ))
        } else {
            None
        }
    }

    /// 检查实体是否在可见区域内
    fn is_visible(pos: &Position, min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> bool {
        // 添加一些边距以避免边缘裁剪
        const MARGIN: f32 = 100.0;
        pos.x >= min_x - MARGIN 
            && pos.x <= max_x + MARGIN
            && pos.y >= min_y - MARGIN
            && pos.y <= max_y + MARGIN
    }

    /// 渲染单个精灵
    fn render_sprite(
        canvas: &mut Canvas,
        sprite: &Sprite,
        world_x: f32,
        world_y: f32,
        camera_x: f32,
        camera_y: f32,
        zoom: f32,
        screen_width: f32,
        screen_height: f32,
    ) -> GameResult {
        // ⚠️ 已废弃：精灵数据获取已由 SpriteRenderSystem 中的 LibraryName 系统完成
        // 不再需要从此处读取 Libraries 精灵图数据
        
        // 将世界坐标转换为屏幕坐标
        let screen_x = (world_x - camera_x) * zoom + screen_width / 2.0;
        let screen_y = (world_y - camera_y) * zoom + screen_height / 2.0;
        
        // 绘制占位矩形 (32x48 像素的人物大小)
        let width = 32.0 * zoom;
        let height = 48.0 * zoom;
        
        let rect = Rect::new(screen_x - width / 2.0, screen_y - height, width, height);
        let color = Color::from_rgba(100, 150, 255, 180);
        
        canvas.draw(
            &ggez::graphics::Quad,
            DrawParam::new()
                .dest([rect.x, rect.y])
                .scale([rect.w, rect.h])
                .color(color),
        );
        
        Ok(())
    }
}

impl RenderSystem for EntityRenderSystem {
    fn draw(
        &mut self,
        _world: &GameWorld,
    ) -> crate::compat::GameResult {
        // ⚠️ 已废弃：本 draw() 返回 Ok(()) 是占位，实际渲染由 SpriteRenderSystem 处理
        Ok(())
    }
}
