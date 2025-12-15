// ============================================================================
// 地图渲染系统 - 混合系统(Hybrid System)
// ============================================================================
//
// 目标：把 scenes/ 内的地图渲染（MeshMapRenderer + MapReader）迁移到 ECS 渲染管线。
//
// 职责：
// - update():
//   - 跟随 MapManager.current_map_file 切换 MapReader
//   - 驱动 MeshMapRenderer 动画
//   - 写入 UiState.minimap_world_size
//   - 计算 FrontOcclusion.local_player_occluded（用于 PostFront ghost）
// - draw():
//   - RenderStage::Normal 画 Back + Middle
//   - RenderStage::PostFront 画 Front
// ============================================================================

use macroquad::prelude::*;

use crate::components::{Camera, FrontOcclusion, LocalPlayer, Position, RenderConfig, RenderPass, RenderStage};
use crate::game::{GameContext, GameResult};
use crate::map_renderer::MeshMapRenderer;
use crate::resources::MapReader;
use crate::systems::{MapManager, RenderSystem};
use crate::ui::ui_state::UiState;

#[derive(ecs_macros::RenderSystem)]
pub struct MapRenderSystem {
    renderer: MeshMapRenderer,
    map_reader: Option<MapReader>,
    loaded_map_file: Option<String>,
}

impl MapRenderSystem {
    pub fn new() -> Self {
        Self {
            renderer: MeshMapRenderer::new(48.0, 32.0),
            map_reader: None,
            loaded_map_file: None,
        }
    }

    fn desired_map_file(ctx: &GameContext) -> Option<String> {
        let mut q = ctx.world.query::<&MapManager>();
        q.iter().next().map(|(_, mgr)| mgr.current_map_file.clone())
    }

    fn render_config(ctx: &GameContext) -> RenderConfig {
        let mut q = ctx.world.query::<&RenderConfig>();
        q.iter().next().map(|(_, cfg)| cfg.clone()).unwrap_or_default()
    }

    fn update_minimap_world_size(ctx: &mut GameContext, width: i32, height: i32) {
        let mut q = ctx.world.query::<&UiState>();
        let Some((_e, ui)) = q.iter().next() else {
            return;
        };
        let mut ui = ui.borrow_mut();
        ui.minimap_world_size = Some(Vec2::new(width as f32, height as f32));
    }

    fn update_front_occlusion(ctx: &mut GameContext, occluded: bool) {
        // 约定：FrontOcclusion 挂在 RenderPass 单例实体上。
        if let Some((_e, o)) = ctx.world.query_mut::<&mut FrontOcclusion>().into_iter().next() {
            o.local_player_occluded = occluded;
            return;
        }

        // 兜底：若未挂载，动态创建一个（避免初始化顺序导致不可用）。
        ctx.world.spawn((FrontOcclusion { local_player_occluded: occluded },));
    }
}

impl Default for MapRenderSystem {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// RenderSystem 实现 - 渲染地图
// ============================================================================
impl RenderSystem for MapRenderSystem {
    fn update(&mut self, ctx: &mut GameContext, delta_time: f32) -> GameResult {
        let cfg = Self::render_config(ctx);

        // 1) 跟随 MapManager 切换地图
        if let Some(file) = Self::desired_map_file(ctx) {
            if !file.is_empty() && self.loaded_map_file.as_deref() != Some(file.as_str()) {
                let map_path = crate::resources::map_reader::resolve_map_path(&file);
                match MapReader::new(&map_path) {
                    Ok(reader) => {
                        tracing::info!("🗺️ MapRenderSystem: loaded map {} ({}x{})", map_path, reader.width, reader.height);
                        Self::update_minimap_world_size(ctx, reader.width, reader.height);
                        self.map_reader = Some(reader);
                        self.loaded_map_file = Some(file);
                    }
                    Err(e) => {
                        tracing::warn!("⚠️ MapRenderSystem: failed to load map {}: {}", map_path, e);
                        self.map_reader = None;
                        self.loaded_map_file = None;
                    }
                }
            }
        }

        // 2) 动画
        if cfg.show_animations {
            self.renderer.update(delta_time);
        }

        // 3) 前景遮挡检测（用于本地玩家 ghost）
        let occluded = if cfg.show_front {
            match self.map_reader.as_ref() {
                Some(map) => {
                    // 本地玩家脚下位置（复制出数值，避免 query borrow 生命周期问题）
                    let (px, py) = {
                        let mut q = ctx.world.query::<(&LocalPlayer, &Position)>();
                        match q.iter().next() {
                            Some((_e, (_local, p))) => (p.x, p.y),
                            None => (f32::NAN, f32::NAN),
                        }
                    };

                    if !px.is_finite() || !py.is_finite() {
                        false
                    } else {
                        const FRONT_OCCLUSION_SEARCH_RADIUS_X_TILES: i32 = 3;
                        const FRONT_OCCLUSION_SEARCH_RADIUS_Y_TILES: i32 = 10;
                        const PLAYER_OCCLUSION_PROBE_WIDTH_PX: f32 = 26.0;
                        const PLAYER_OCCLUSION_PROBE_HEIGHT_PX: f32 = 56.0;

                        let foot = vec2(px, py);
                        let probe = Rect::new(
                            foot.x - PLAYER_OCCLUSION_PROBE_WIDTH_PX * 0.5,
                            foot.y - PLAYER_OCCLUSION_PROBE_HEIGHT_PX,
                            PLAYER_OCCLUSION_PROBE_WIDTH_PX,
                            PLAYER_OCCLUSION_PROBE_HEIGHT_PX,
                        );
                        self.renderer.front_layer_occludes_probe(
                            map,
                            probe,
                            FRONT_OCCLUSION_SEARCH_RADIUS_X_TILES,
                            FRONT_OCCLUSION_SEARCH_RADIUS_Y_TILES,
                        )
                    }
                }
                None => false,
            }
        } else {
            false
        };
        Self::update_front_occlusion(ctx, occluded);

        Ok(())
    }

    fn draw(&mut self, world: &hecs::World) -> GameResult {
        let pass = world
            .query::<&RenderPass>()
            .iter()
            .next()
            .map(|(_, p)| *p)
            .unwrap_or_default();

        if pass.stage == RenderStage::Ui {
            return Ok(());
        }

        let cfg = world
            .query::<&RenderConfig>()
            .iter()
            .next()
            .map(|(_, c)| c.clone())
            .unwrap_or_default();

        let Some(map) = self.map_reader.as_ref() else {
            return Ok(());
        };

        // camera 参数：用于 MeshMapRenderer 计算视口范围；真实坐标变换仍由 GameScene 的 Camera2D 完成。
        let (cam_x, cam_y, cam_zoom, sw, sh) = world
            .query::<(&Camera, &Position)>()
            .iter()
            .next()
            .map(|(_, (c, p))| (p.x, p.y, c.zoom, c.screen_width, c.screen_height))
            .unwrap_or((0.0, 0.0, 1.0, screen_width(), screen_height()));

        let zoom = cam_zoom.max(0.0001);

        // 与 GameScene 的像素对齐策略保持一致
        let snapped_x = (cam_x * zoom).round() / zoom;
        let snapped_y = (cam_y * zoom).round() / zoom;

        self.renderer.show_back_layer = cfg.show_back;
        self.renderer.show_middle_layer = cfg.show_middle;
        self.renderer.show_front_layer = cfg.show_front;

        match pass.stage {
            RenderStage::Normal => {
                // 只画 Back+Middle
                let original_front = self.renderer.show_front_layer;
                self.renderer.show_front_layer = false;
                let _ = self
                    .renderer
                    .render(map, snapped_x, snapped_y, sw, sh, zoom, WHITE);
                self.renderer.show_front_layer = original_front;
            }
            RenderStage::PostFront => {
                // 只画 Front
                if cfg.show_front {
                    let _ = self.renderer.render_front_layer_with_focus(
                        map,
                        snapped_x,
                        snapped_y,
                        sw,
                        sh,
                        zoom,
                        WHITE,
                        None,
                        0,
                        0,
                        1.0,
                    );
                }
            }
            RenderStage::Ui => {}
        }

        Ok(())
    }
}

// ============================================================================
// 总结
// ============================================================================
//
// 地图渲染系统的核心流程：
// 1. 获取相机和配置
// 2. 计算视口范围
// 3. 按层级循环（Back → Middle → Front）
//    3.1 准备瓦片容器
//    3.2 计算视口扩展
//    3.3 标记动画瓦片
//    3.4 收集静态瓦片
//    3.5 收集动画瓦片
//    3.6 渲染所有瓦片
//        3.6.1 获取图库
//        3.6.2 获取图像尺寸
//        3.6.3 获取纹理信息
//        3.6.4 计算基础世界坐标
//        3.6.5 计算Y轴偏移（层级规则）
//        3.6.6 判断图像内部偏移（特殊情况）
//        3.6.7 世界→屏幕坐标转换
//        3.6.8 设置混合模式
//        3.6.9 绘制瓦片
//        3.6.10 恢复混合模式
// 4. 返回渲染结果
//
// 关键要点：
// - 视口裁剪减少99%渲染量
// - 层级顺序确保正确遮挡关系
// - 精确偏移计算确保对齐
// - 条件偏移防止Back层错位
// - 混合模式实现发光效果
//
// 调试技巧：
// - 按1/2/3切换层级查看
// - 按S/D分离静态/动画瓦片
// - 按G显示网格辅助定位
// - 检查LOG输出排查问题
//
// ============================================================================
