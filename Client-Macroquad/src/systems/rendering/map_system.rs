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

use std::time::{Duration, Instant};

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

    // 遮挡检测很重（会扫描大量 front tiles）。这里做缓存+节流，避免每帧都跑。
    last_occlusion_check: Instant,
    last_occlusion_pos: Vec2,
    last_occluded: bool,
}

impl MapRenderSystem {
    pub fn new() -> Self {
        Self {
            renderer: MeshMapRenderer::new(48.0, 32.0),
            map_reader: None,
            loaded_map_file: None,

            last_occlusion_check: Instant::now(),
            last_occlusion_pos: vec2(f32::NAN, f32::NAN),
            last_occluded: false,
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
                        self.last_occluded = false;
                        self.last_occlusion_pos = vec2(px, py);
                        self.last_occlusion_check = Instant::now();
                        false
                    } else {
                        // 节流：大多数帧复用上次结果。
                        // 经验值：100ms 对“遮挡半透明”视觉足够稳定，但能显著省 CPU。
                        const OCCLUSION_CHECK_INTERVAL: Duration = Duration::from_millis(100);
                        const OCCLUSION_MOVE_EPS_PX: f32 = 2.0;

                        let now = Instant::now();
                        let moved = (vec2(px, py) - self.last_occlusion_pos).length() > OCCLUSION_MOVE_EPS_PX;
                        let due = now.duration_since(self.last_occlusion_check) >= OCCLUSION_CHECK_INTERVAL;
                        let use_cache = !moved && !due;

                        if use_cache {
                            self.last_occluded
                        } else {
                            // 一些很长的 Front 贴图（屋檐/墙体/树冠）可能“基准格”离玩家较远，
                            // 但实际贴图矩形会覆盖到玩家；需要扩大搜索半径以避免漏判。
                            const FRONT_OCCLUSION_SEARCH_RADIUS_X_TILES: i32 = 16;
                            const FRONT_OCCLUSION_SEARCH_RADIUS_Y_TILES: i32 = 14;
                            // 说明：玩家实际可见范围（衣服/翅膀/武器）会比“脚下 26x56”更大，
                            // 而一些前景物（树叶/屋檐）可能只遮住上半身/左右边缘。
                            // 为了让“部分遮挡”也触发 ghost，这里使用多个 probe 覆盖脚下到头部的大致范围。
                            let foot = vec2(px, py);

                            let probes = [
                                // 1) 原来的窄探针（脚下到上半身）
                                Rect::new(foot.x - 13.0, foot.y - 56.0, 26.0, 56.0),
                                // 2) 更高：覆盖到头顶（常见“遮住上半身但脚下露出”的情况）
                                Rect::new(foot.x - 16.0, foot.y - 88.0, 32.0, 88.0),
                                // 3) 更宽：覆盖左右边缘（常见“只遮住一侧肩膀/翅膀”的情况）
                                Rect::new(foot.x - 24.0, foot.y - 72.0, 48.0, 72.0),
                                // 4) 更大：覆盖翅膀/武器上半部的极端外扩（避免“明明遮住了翅膀但不触发 ghost”）
                                Rect::new(foot.x - 40.0, foot.y - 112.0, 80.0, 112.0),
                            ];

                            let computed = probes.into_iter().any(|probe| {
                                self.renderer.front_layer_occludes_probe(
                                    map,
                                    probe,
                                    FRONT_OCCLUSION_SEARCH_RADIUS_X_TILES,
                                    FRONT_OCCLUSION_SEARCH_RADIUS_Y_TILES,
                                )
                            });

                            self.last_occluded = computed;
                            self.last_occlusion_pos = vec2(px, py);
                            self.last_occlusion_check = now;
                            computed
                        }
                    }
                }
                None => false,
            }
        } else {
            self.last_occluded = false;
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
                    // 前景建筑贴图可能非常“长”，其基准格子在屏幕外（更下方），
                    // 但贴图本体会伸到屏幕底部区域；需要额外向下过扫描避免缺块。
                    let old_bottom_margin = self.renderer.bottom_margin;
                    self.renderer.bottom_margin = old_bottom_margin.max(420.0);

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

                    self.renderer.bottom_margin = old_bottom_margin;
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
